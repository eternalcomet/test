use crate::core::fs::fd::fd_lookup;
use crate::core::time::TimeSpec;
use crate::imp::fs::stat::sys_stat_impl;
use crate::imp::fs::{FsStatxTimestamp, UserStatFs, sys_statfs_impl};
use crate::ptr::{UserInPtr, UserOutPtr, nullable};
use crate::utils::path::{ResolveFlags, resolve_path_at_cwd};
use alloc::format;
use axerrno::LinuxError;
use axerrno::LinuxResult;
use core::ffi::{c_char, c_int, c_long, c_uint, c_ulong};
use linux_raw_sys::general::{AT_FDCWD, STATX_BASIC_STATS};
use syscall_trace::syscall_trace;
use undefined_vfs::types::Metadata;

/// user struct: stat
#[cfg(target_arch = "x86_64")]
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct UserStat {
    /// ID of device containing file
    pub st_dev: c_ulong,
    /// inode number
    pub st_ino: c_ulong,
    /// number of hard links
    /// note that the field sequence is different from other archs
    pub st_nlink: c_ulong,
    /// file type and mode
    pub st_mode: c_uint,
    /// user ID of owner
    pub st_uid: c_uint,
    /// group ID of owner
    pub st_gid: c_uint,
    /// paddings for arch x86_64
    pub _pad0: c_int,
    /// device ID (if special file)
    pub st_rdev: c_ulong,
    /// total size, in bytes
    pub st_size: c_long,
    /// Block size for filesystem I/O
    pub st_blksize: c_long,
    /// number of blocks allocated
    pub st_blocks: c_long,
    /// time of last access
    pub st_atime: TimeSpec,
    /// time of last modification
    pub st_mtime: TimeSpec,
    /// time of last status change
    pub st_ctime: TimeSpec,
    /// glibc reserved for arch x86_64
    pub _glibc_reserved: [c_long; 3],
}

#[cfg(target_arch = "x86_64")]
impl From<Metadata> for UserStat {
    fn from(metadata: Metadata) -> Self {
        let node_type = metadata.node_type as u32;
        let permissions = metadata.mode.bits() as u32;
        let st_mode = (node_type << 12) | permissions;
        UserStat {
            st_dev: metadata.device,
            st_ino: metadata.inode,
            st_nlink: metadata.n_link as _,
            st_mode,
            st_uid: metadata.uid,
            st_gid: metadata.gid,
            st_rdev: metadata.raw_device.as_u64(),
            st_size: metadata.size as _,
            st_blksize: metadata.block_size as _,
            st_blocks: metadata.n_blocks as _,
            st_atime: metadata.access_time.into(),
            st_mtime: metadata.modify_time.into(),
            st_ctime: metadata.change_time.into(),
            ..Default::default()
        }
    }
}

#[cfg(target_arch = "x86_64")]
const _: () = assert!(size_of::<UserStat>() == 144, "size of Stat is not 144");

#[cfg(not(target_arch = "x86_64"))]
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct UserStat {
    /// ID of device containing file
    pub st_dev: c_ulong,
    /// inode number
    pub st_ino: c_ulong,
    /// file type and mode
    pub st_mode: c_uint,
    /// number of hard links
    pub st_nlink: c_uint,
    /// user ID of owner
    pub st_uid: c_uint,
    /// group ID of owner
    pub st_gid: c_uint,
    /// device ID (if special file)
    pub st_rdev: c_ulong,
    /// paddings for arch non x86_64
    pub _pad0: c_long,
    /// total size, in bytes
    pub st_size: c_long,
    /// Block size for filesystem I/O
    pub st_blksize: c_int,
    /// paddings for arch non x86_64
    pub _pad1: c_int,
    /// number of blocks allocated
    pub st_blocks: c_long,
    /// time of last access
    pub st_atime: TimeSpec,
    /// time of last modification
    pub st_mtime: TimeSpec,
    /// time of last status change
    pub st_ctime: TimeSpec,
    /// reserved for arch non x86_64
    pub _unused: [c_int; 2],
}

#[cfg(not(target_arch = "x86_64"))]
impl From<Metadata> for UserStat {
    fn from(metadata: Metadata) -> Self {
        let node_type = metadata.node_type as u32;
        let permissions = metadata.mode.bits() as u32;
        let st_mode = (node_type << 12) | permissions;
        UserStat {
            st_dev: metadata.device,
            st_ino: metadata.inode,
            st_nlink: metadata.n_link as _,
            st_mode,
            st_uid: metadata.uid,
            st_gid: metadata.gid,
            st_rdev: metadata.raw_device.as_u64(),
            st_size: metadata.size as _,
            st_blksize: metadata.block_size as _,
            st_blocks: metadata.n_blocks as _,
            st_atime: metadata.access_time.into(),
            st_mtime: metadata.modify_time.into(),
            st_ctime: metadata.change_time.into(),
            ..Default::default()
        }
    }
}

#[cfg(not(target_arch = "x86_64"))]
const _: () = assert!(size_of::<UserStat>() == 128, "size of Stat is not 128");

/// user struct: statx
#[repr(C)]
#[derive(Debug, Default)]
pub struct UserStatX {
    /// Bitmask of what information to get.
    pub stx_mask: u32,
    /// Block size for filesystem I/O.
    pub stx_blksize: u32,
    /// File attributes.
    pub stx_attributes: u64,
    /// Number of hard links.
    pub stx_nlink: u32,
    /// User ID of owner.
    pub stx_uid: u32,
    /// Group ID of owner.
    pub stx_gid: u32,
    /// File mode (permissions).
    pub stx_mode: u16,
    /// padding
    pub _pad0: u16,
    /// Inode number.
    pub stx_ino: u64,
    /// Total size, in bytes.
    pub stx_size: u64,
    /// Number of 512B blocks allocated.
    pub stx_blocks: u64,
    /// Mask to show what's supported in stx_attributes.
    pub stx_attributes_mask: u64,
    /// Last access timestamp.
    pub stx_atime: FsStatxTimestamp,
    /// Birth (creation) timestamp.
    pub stx_btime: FsStatxTimestamp,
    /// Last status change timestamp.
    pub stx_ctime: FsStatxTimestamp,
    /// Last modification timestamp.
    pub stx_mtime: FsStatxTimestamp,
    /// Major device ID (if special file).
    pub stx_rdev_major: u32,
    /// Minor device ID (if special file).
    pub stx_rdev_minor: u32,
    /// Major device ID of file system.
    pub stx_dev_major: u32,
    /// Minor device ID of file system.
    pub stx_dev_minor: u32,
    /// Mount ID.
    pub stx_mnt_id: u64,
    /// Memory alignment for direct I/O.
    pub stx_dio_mem_align: u32,
    /// Offset alignment for direct I/O.
    pub stx_dio_offset_align: u32,
    /// Reserved for future use.
    pub _spare: [u32; 12],
}

impl From<Metadata> for UserStatX {
    fn from(metadata: Metadata) -> Self {
        let node_type = metadata.node_type as u32;
        let permissions = metadata.mode.bits() as u32;
        let st_mode = (node_type << 12) | permissions;
        Self {
            stx_mask: STATX_BASIC_STATS,
            stx_blksize: metadata.block_size as _,
            stx_nlink: metadata.n_link as _,
            stx_uid: metadata.uid,
            stx_gid: metadata.gid,
            stx_mode: st_mode as _,
            stx_ino: metadata.inode as _,
            stx_size: metadata.size as _,
            stx_blocks: metadata.n_blocks as _,
            stx_atime: metadata.access_time.into(),
            stx_ctime: metadata.change_time.into(),
            stx_mtime: metadata.modify_time.into(),
            stx_dev_major: metadata.device as _,
            stx_rdev_major: metadata.raw_device.major(),
            stx_rdev_minor: metadata.raw_device.minor(),
            ..Default::default()
        }
    }
}

#[syscall_trace]
pub fn sys_stat(path: UserInPtr<c_char>, stat_buf: UserOutPtr<UserStat>) -> LinuxResult<isize> {
    let path = nullable!(path.get_as_str())?;
    let stat_buf = stat_buf.get_as_mut_ref()?;
    let file_status = sys_stat_impl(AT_FDCWD, path, ResolveFlags::empty())?;
    *stat_buf = file_status.into();
    Ok(0)
}

#[syscall_trace]
pub fn sys_lstat(path: UserInPtr<c_char>, stat_buf: UserOutPtr<UserStat>) -> LinuxResult<isize> {
    let path = nullable!(path.get_as_str())?;
    let stat_buf = stat_buf.get_as_mut_ref()?;
    let file_status = sys_stat_impl(AT_FDCWD, path, ResolveFlags::NO_FOLLOW)?;
    *stat_buf = file_status.into();
    Ok(0)
}

#[syscall_trace]
pub fn sys_fstat(fd: c_int, stat_buf: UserOutPtr<UserStat>) -> LinuxResult<isize> {
    let stat_buf = stat_buf.get_as_mut_ref()?;
    let file_status = sys_stat_impl(fd, None, ResolveFlags::empty())?;
    *stat_buf = file_status.into();
    Ok(0)
}

#[syscall_trace]
pub fn sys_fstatat(
    dir_fd: c_int,
    path: UserInPtr<c_char>,
    stat_buf: UserOutPtr<UserStat>,
    flags: c_int,
) -> LinuxResult<isize> {
    let path = nullable!(path.get_as_str())?;
    let stat_buf = stat_buf.get_as_mut_ref()?;
    let flags = ResolveFlags::from_bits_truncate(flags as _);
    let file_status = sys_stat_impl(dir_fd, path, flags)?;
    *stat_buf = file_status.into();
    Ok(0)
}

#[syscall_trace]
pub fn sys_statx(
    dir_fd: c_int,
    path: UserInPtr<c_char>,
    flags: c_int,
    _mask: c_uint,
    statx_buf: UserOutPtr<UserStatX>,
) -> LinuxResult<isize> {
    let path = nullable!(path.get_as_str())?;
    let statx_buf = statx_buf.get_as_mut_ref()?;
    let flags = ResolveFlags::from_bits_truncate(flags as _);
    let file_status = sys_stat_impl(dir_fd, path, flags)?;
    *statx_buf = file_status.into();
    Ok(0)
}

#[syscall_trace]
pub fn sys_statfs(path: UserInPtr<c_char>, buf: UserOutPtr<UserStatFs>) -> LinuxResult<isize> {
    let path = nullable!(path.get_as_str())?;
    let buf = buf.get_as_mut_ref()?;
    let location = resolve_path_at_cwd(path)?;
    let location = location.mountpoint().root_location();
    sys_statfs_impl(&location, buf)?;
    Ok(0)
}

#[syscall_trace]
pub fn sys_fstatfs(fd: c_int, buf: UserOutPtr<UserStatFs>) -> LinuxResult<isize> {
    let buf = buf.get_as_mut_ref()?;
    let file_like = fd_lookup(fd)?;
    let location = file_like.location().ok_or(LinuxError::EINVAL)?;
    let location = location.mountpoint().root_location();
    sys_statfs_impl(&location, buf)?;
    Ok(0)
}
