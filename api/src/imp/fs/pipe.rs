use alloc::sync::Arc;
use crate::core::fs::fd::{FdFlags, FileDescriptor, fd_add, fd_remove};
use crate::core::fs::pipe::Pipe;
use crate::ptr::UserOutPtr;
use axerrno::LinuxResult;
use axfs_ng::api::FileFlags;
use linux_raw_sys::general::O_CLOEXEC;
use syscall_trace::syscall_trace;

pub fn sys_pipe_impl(flags: u32) -> LinuxResult<[FileDescriptor; 2]> {
    let fd_flags = if flags & O_CLOEXEC != 0 {
        FdFlags::CLOSE_ON_EXEC
    } else {
        FdFlags::empty()
    };
    let file_flags = FileFlags::from_bits_truncate(flags);
    let (read_end, write_end) = Pipe::new(file_flags);
    let read_fd = fd_add(Arc::new(read_end), fd_flags)?;
    let write_fd = fd_add(Arc::new(write_end), fd_flags).inspect_err(|_| {
        let _ = fd_remove(read_fd);
    })?;
    info!(
        "[sys_pipe] created pipe: read_fd = {}, write_fd = {}",
        read_fd, write_fd
    );
    Ok([read_fd, write_fd])
}

#[syscall_trace]
pub fn sys_pipe2(fds: UserOutPtr<i32>, flags: u32) -> LinuxResult<isize> {
    let fds = fds.get_as_mut_slice(2)?;
    let result = sys_pipe_impl(flags)?;
    fds[..2].copy_from_slice(&result);
    Ok(0)
}

#[syscall_trace]
pub fn sys_pipe(fds: UserOutPtr<FileDescriptor>) -> LinuxResult<isize> {
    let fds = fds.get_as_mut_slice(2)?;
    let result = sys_pipe_impl(0)?;
    fds[..2].copy_from_slice(&result);
    Ok(0)
}
