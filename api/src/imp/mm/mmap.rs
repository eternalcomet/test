use crate::core::fs::fd::FileLike;
use crate::core::fs::file::File;
use crate::ptr::UserInPtr;
use crate::{
    ptr::{PtrWrapper, UserPtr},
    syscall_instrument,
};
use alloc::vec;
use axerrno::{LinuxError, LinuxResult};
use axhal::paging::{MappingFlags, PageSize};
use linux_raw_sys::general::{
    MAP_ANONYMOUS, MAP_FIXED, MAP_FIXED_NOREPLACE, MAP_HUGE_1GB, MAP_HUGE_2MB, MAP_HUGETLB,
    MAP_NORESERVE, MAP_PRIVATE, MAP_SHARED, MAP_STACK, PROT_EXEC, PROT_GROWSDOWN, PROT_GROWSUP,
    PROT_READ, PROT_WRITE,
};
use macro_rules_attribute::apply;
use memory_addr::{MemoryAddr, VirtAddr, VirtAddrRange, align_up};
use starry_core::task::current_process_data;
use syscall_trace::syscall_trace;

bitflags::bitflags! {
    /// permissions for sys_mmap
    ///
    /// See <https://github.com/bminor/glibc/blob/master/bits/mman.h>
    #[derive(Debug)]
    struct MmapProt: u32 {
        /// Page can be read.
        const PROT_READ = PROT_READ;
        /// Page can be written.
        const PROT_WRITE = PROT_WRITE;
        /// Page can be executed.
        const PROT_EXEC = PROT_EXEC;
        /// Extend change to start of growsdown vma (mprotect only).
        const PROT_GROWDOWN = PROT_GROWSDOWN;
        /// Extend change to start of growsup vma (mprotect only).
        const PROT_GROWSUP = PROT_GROWSUP;
    }
}

impl From<MmapProt> for MappingFlags {
    fn from(value: MmapProt) -> Self {
        let mut flags = MappingFlags::USER;
        if value.contains(MmapProt::PROT_READ) {
            flags |= MappingFlags::READ;
        }
        if value.contains(MmapProt::PROT_WRITE) {
            flags |= MappingFlags::WRITE;
        }
        if value.contains(MmapProt::PROT_EXEC) {
            flags |= MappingFlags::EXECUTE;
        }
        flags
    }
}

bitflags::bitflags! {
    /// flags for sys_mmap
    ///
    /// See <https://github.com/bminor/glibc/blob/master/bits/mman.h>
    #[derive(Debug)]
    struct MmapFlags: u32 {
        /// Share changes
        const MAP_SHARED = MAP_SHARED;
        /// Changes private; copy pages on write.
        const MAP_PRIVATE = MAP_PRIVATE;
        /// Map address must be exactly as requested, no matter whether it is available.
        const MAP_FIXED = MAP_FIXED;
        /// Map address must be exactly as requested, but fail if it is not available.
        const MAP_FIXED_NOREPLACE = MAP_FIXED_NOREPLACE;
        /// Don't use a file.
        const MAP_ANONYMOUS = MAP_ANONYMOUS;
        /// Don't check for reservations.
        const MAP_NORESERVE = MAP_NORESERVE;
        /// Allocation is for a stack.
        const MAP_STACK = MAP_STACK;
        /// Huge page
        const HUGETLB = MAP_HUGETLB;
        /// Huge page 2m size
        const HUGE_2MB = MAP_HUGE_2MB;
        /// Huge page 1g size
        const HUGE_1GB = MAP_HUGE_1GB;
    }
}

#[syscall_trace]
pub fn sys_mmap(
    addr: usize,
    length: usize,
    prot: u32,
    flags: u32,
    fd: i32,
    offset: isize,
) -> LinuxResult<isize> {
    let current = current_process_data();
    let mut aspace = current.addr_space.lock();
    let permission_flags = MmapProt::from_bits_truncate(prot);
    let map_flags = MmapFlags::from_bits_truncate(flags);

    // validate flags
    // TODO: more checks
    if map_flags.contains(MmapFlags::MAP_PRIVATE | MmapFlags::MAP_SHARED) {
        return Err(LinuxError::EINVAL);
    }

    // determine page size
    let page_size = if map_flags.contains(MmapFlags::HUGETLB) {
        if map_flags.contains(MmapFlags::HUGE_1GB) {
            PageSize::Size1G
        } else if map_flags.contains(MmapFlags::HUGE_2MB) {
            PageSize::Size2M
        } else {
            error!("[sys_mmap] HUGETLB flag is set, but no supported huge page size is specified.");
            return Err(LinuxError::EINVAL);
        }
    } else {
        PageSize::Size4K
    };

    info!(
        "mmap: addr: {:?}, length: {:x?}, prot: {:?}, flags: {:?}, fd: {:?}, offset: {:?}, page_size: {:?}",
        addr, length, permission_flags, map_flags, fd, offset, page_size
    );

    let aligned_length = align_up(length, page_size.into());

    let addr = VirtAddr::from(addr);
    let start_addr = if map_flags.intersects(MmapFlags::MAP_FIXED | MmapFlags::MAP_FIXED_NOREPLACE)
    {
        // If the memory region specified by addr and length overlaps pages of any existing mapping(s),
        // then the overlapped part of the existing mapping(s) will be discarded.
        if map_flags.contains(MmapFlags::MAP_FIXED) {
            aspace.unmap(addr, aligned_length)?;
        }
        // If the MAP_FIXED flag is specified, and addr is 0 (NULL), then the mapped address will be 0 (NULL).
        // so we needn't check if addr is NULL.
        addr
    } else {
        // currently we find free area in the whole address space
        // in Linux, the boundary is above or equal to the value specified by `/proc/sys/vm/mmap_min_addr`
        let range = VirtAddrRange::new(aspace.base(), aspace.end());
        let addr = addr.align_down(page_size);
        aspace
            .find_free_area(addr, length, range, page_size)
            .or(aspace.find_free_area(aspace.base(), length, range, page_size))
            .ok_or(LinuxError::ENOMEM)?
    };

    let populate = if fd == -1 {
        false
    } else {
        !map_flags.contains(MmapFlags::MAP_ANONYMOUS)
    };

    let writeable = permission_flags.contains(MmapProt::PROT_WRITE)
        && map_flags.contains(MmapFlags::MAP_SHARED);

    aspace.map_alloc(
        start_addr,
        aligned_length,
        permission_flags.into(),
        populate,
        page_size,
    )?;

    if populate {
        let file = File::from_fd(fd)?;
        let file_size = file.status()?.size as usize;

        if writeable {
            error!(
                "we don't support PROT_WRITE for mmap with fd yet. file: {}.",
                file.inner().location().absolute_path()?.as_str()
            );
        }
        let mut file = file.inner();
        if offset < 0 || offset as usize >= file_size {
            return Err(LinuxError::EINVAL);
        }
        let offset = offset as usize;
        let length = core::cmp::min(length, file_size - offset);
        let mut buf = vec![0u8; length];
        file.read_at(&mut buf, offset as _)?;
        aspace.write(start_addr, page_size, &buf)?;
    }
    Ok(start_addr.as_usize() as _)
}

#[apply(syscall_instrument)]
pub fn sys_munmap(addr: UserPtr<usize>, length: usize) -> LinuxResult<isize> {
    // Safety: addr is used for mapping, and we won't directly access it.
    let addr = unsafe { addr.get_unchecked() };

    let current = current_process_data();
    let mut aspace = current.addr_space.lock();
    let length = memory_addr::align_up_4k(length);
    let start_addr = VirtAddr::from(addr as usize);
    aspace.unmap(start_addr, length)?;
    axhal::arch::flush_tlb(None);
    Ok(0)
}

#[syscall_trace]
pub fn sys_mprotect(addr: UserInPtr<usize>, length: usize, prot: u32) -> LinuxResult<isize> {
    // Safety: addr is used for mapping, and we won't directly access it.
    let addr = unsafe { addr.get_unchecked() };

    // TODO: implement PROT_GROWSUP & PROT_GROWSDOWN
    let Some(permission_flags) = MmapProt::from_bits(prot) else {
        return Err(LinuxError::EINVAL);
    };
    if permission_flags.contains(MmapProt::PROT_GROWDOWN | MmapProt::PROT_GROWSUP) {
        return Err(LinuxError::EINVAL);
    }

    let current = current_process_data();
    let mut aspace = current.addr_space.lock();
    let length = memory_addr::align_up_4k(length);
    let start_addr = VirtAddr::from(addr as usize);
    aspace.protect(start_addr, length, permission_flags.into())?;

    Ok(0)
}
