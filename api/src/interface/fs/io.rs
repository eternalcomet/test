use crate::core::fs::fd::{FileLike, fd_lookup, file_like_as};
use crate::core::fs::file::File;
use crate::imp::fs::{
    sys_pread_impl, sys_pwrite_impl, sys_read_impl, sys_truncate_impl, sys_write_impl,
};
use crate::ptr::{UserInOutPtr, UserInPtr, UserOutPtr, nullable};
use crate::utils::path::resolve_path_at_cwd;
use alloc::vec;
use axerrno::{LinuxError, LinuxResult};
use axfs_ng::api::FileFlags;
use axio::SeekFrom;
use core::cmp::min;
use core::ffi::{c_char, c_int, c_long};
use syscall_trace::syscall_trace;

#[syscall_trace]
pub fn sys_truncate(path: UserInPtr<c_char>, length: c_long) -> LinuxResult<isize> {
    // get params
    let path = nullable!(path.get_as_str())?;

    // open file
    let location = resolve_path_at_cwd(path)?;
    let file = File::from_location(location, FileFlags::WRITE);
    sys_truncate_impl(&file, length as _)
}

#[syscall_trace]
pub fn sys_ftruncate(fd: c_int, length: c_long) -> LinuxResult<isize> {
    let file = File::from_fd(fd)?;
    sys_truncate_impl(&file, length as _)
}

#[syscall_trace]
pub fn sys_lseek(fd: c_int, offset: isize, whence: c_int) -> LinuxResult<isize> {
    let pos = match whence {
        0 => SeekFrom::Start(offset as _),
        1 => SeekFrom::Current(offset as _),
        2 => SeekFrom::End(offset as _),
        _ => return Err(LinuxError::EINVAL),
    };
    let file_like = fd_lookup(fd)?.into_any();
    if let Some(file) = file_like.downcast_ref::<File>() {
        let offset = file.inner().seek(pos)?;
        Ok(offset as _)
    } else {
        // For pipes, sockets, FIFOs, they are not seekable, so we return an error.
        Err(LinuxError::ESPIPE)
    }
}

#[syscall_trace]
pub fn sys_read(fd: i32, buf: UserOutPtr<u8>, count: usize) -> LinuxResult<isize> {
    let buf = buf.get_as_mut_slice(count)?;
    let file_like = fd_lookup(fd as _)?;
    sys_read_impl(&*file_like, buf)
}

#[syscall_trace]
pub fn sys_write(fd: i32, buf: UserInPtr<u8>, count: usize) -> LinuxResult<isize> {
    let buf = buf.get_as_slice(count)?;
    let file_like = fd_lookup(fd as _)?;
    sys_write_impl(&*file_like, buf)
}

/// structure for ctype `iovec`, used in readv/writev
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct UserIoVector {
    pub base_addr: *mut u8,
    pub length: usize,
}

#[syscall_trace]
pub fn sys_readv(
    fd: i32,
    io_vectors: UserInPtr<UserIoVector>,
    io_count: i32,
) -> LinuxResult<isize> {
    let io_vectors = io_vectors.get_as_slice(io_count as usize)?;
    let file_like = fd_lookup(fd as _)?;
    let mut total_read: isize = 0;
    for io in io_vectors {
        let buf = unsafe { core::slice::from_raw_parts_mut(io.base_addr, io.length) };
        let read_len = sys_read_impl(&*file_like, buf)?;
        total_read += read_len;
    }
    Ok(total_read)
}

#[syscall_trace]
pub fn sys_writev(
    fd: i32,
    io_vectors: UserInPtr<UserIoVector>,
    io_count: i32,
) -> LinuxResult<isize> {
    let io_vectors = io_vectors.get_as_slice(io_count as usize)?;
    let file_like = fd_lookup(fd as _)?;
    let mut total_written: isize = 0;
    for io in io_vectors {
        let buf = unsafe { core::slice::from_raw_parts(io.base_addr, io.length) };
        let write_len = sys_write_impl(&*file_like, buf)?;
        total_written += write_len;
    }
    Ok(total_written)
}

#[syscall_trace]
pub fn sys_pread64(
    fd: c_int,
    buf: UserOutPtr<u8>,
    count: usize,
    offset: usize,
) -> LinuxResult<isize> {
    let buf = buf.get_as_mut_slice(count)?;
    sys_pread_impl(fd, buf, offset as _)
}

#[syscall_trace]
pub fn sys_pwrite64(
    fd: c_int,
    buf: UserInPtr<u8>,
    count: usize,
    offset: usize,
) -> LinuxResult<isize> {
    let buf = buf.get_as_slice(count)?;
    sys_pwrite_impl(fd, buf, offset as _)
}

#[syscall_trace]
pub fn sys_sendfile(
    out_fd: i32,
    in_fd: i32,
    offset: UserInOutPtr<usize>,
    count: usize,
) -> LinuxResult<isize> {
    let in_file = fd_lookup(in_fd)?;
    let out_file = fd_lookup(out_fd)?;
    if !in_file.poll()?.readable || !out_file.poll()?.writable {
        return Err(LinuxError::EBADF);
    }
    if out_file.get_flags().contains(FileFlags::APPEND) {
        // out_fd has the O_APPEND flag set.
        // This is not currently supported by sendfile().
        return Err(LinuxError::EINVAL);
    }
    let buf_size = count.min(40960);
    let mut buf = vec![0u8; buf_size];
    let mut transferred = 0;
    if offset.is_null() {
        while transferred < count {
            let buffer = &mut buf[..min(buf_size, count - transferred)];
            let read_len = in_file.read(buffer)?;
            if read_len == 0 {
                break; // EOF
            }
            let write_len = out_file.write(&buffer[..read_len])?;
            transferred += write_len;
            if write_len < read_len {
                break;
            }
        }
    } else {
        let offset = offset.get_as_mut_ref()?;
        let in_file = file_like_as::<File>(in_file).ok_or(LinuxError::ESPIPE)?;
        while transferred < count {
            let buffer = &mut buf[..min(buf_size, count - transferred)];
            let read_len = in_file
                .inner()
                .read_at(buffer, (*offset + transferred) as _)?;
            if read_len == 0 {
                break; // EOF
            }
            let write_len = out_file.write(&buffer[..read_len])?;
            transferred += write_len;
            if write_len < read_len {
                break;
            }
        }
        *offset += transferred;
    }

    Ok(transferred as _)
}
