use core::ffi::c_int;

use crate::core::fs::dir::Directory;
use crate::core::fs::fd::{
    FdFlags, FileDescriptor, FileLike, fd_add, fd_add_at, fd_get_flags, fd_lookup, fd_remove,
    fd_set_flags,
};
use crate::interface::user::identity::{sys_getegid, sys_geteuid};
use crate::utils::path::{fd_add_result, get_fs_context};
use axerrno::{LinuxError, LinuxResult};
use axfs_ng::api::{FileFlags, open};
use linux_raw_sys::general::{
    AT_FDCWD, F_DUPFD, F_DUPFD_CLOEXEC, F_GETFD, F_GETFL, F_SETFD, F_SETFL, O_CLOEXEC, O_RDONLY,
    O_RDWR, O_WRONLY,
};
use starry_core::resource::ResourceLimitType;
use starry_core::task::current_process_data;
use syscall_trace::syscall_trace;
use undefined_vfs::path::Path;

/// Convert posix raw file flags to `FileFlags`.
pub fn to_file_flags(flags: u32) -> FileFlags {
    let access_mode = match flags & 0b11 {
        O_RDONLY => FileFlags::READ,
        O_WRONLY => FileFlags::WRITE,
        O_RDWR => FileFlags::READ | FileFlags::WRITE,
        _ => FileFlags::empty(),
    };
    FileFlags::from_bits_truncate(flags & !0b11) | access_mode
}

/// Convert `FileFlags` to posix raw file flags.
pub fn from_file_flags(flags: FileFlags) -> u32 {
    let mut result = flags.bits() & !0b11; // Clear the access mode bits
    if flags.contains(FileFlags::READ | FileFlags::WRITE) {
        result |= O_RDWR;
    } else if flags.contains(FileFlags::READ) {
        result |= O_RDONLY;
    } else if flags.contains(FileFlags::WRITE) {
        result |= O_WRONLY;
    }
    result
}

pub fn sys_open_impl(
    parent_fd: FileDescriptor,
    path: &Path,
    flags: u32,
    create_mode: u32,
) -> LinuxResult<FileDescriptor> {
    let open_flags = to_file_flags(flags);
    let context = get_fs_context();
    let uid = sys_geteuid()? as u32;
    let gid = sys_getegid()? as u32;
    let create_user = Some((uid, gid));

    // 这里不使用 `resolve_path_at` 是因为我们需要容忍可能不存在的文件
    let result = if parent_fd == AT_FDCWD {
        open(path, &context, open_flags, Some(create_mode), create_user)?
    } else {
        let dir = Directory::from_fd(parent_fd)?;
        let context = context.with_current_dir(dir.inner().location().clone())?;
        open(path, &context, open_flags, Some(create_mode), create_user)?
    };
    let fd_flags = if flags & O_CLOEXEC != 0 {
        FdFlags::CLOSE_ON_EXEC
    } else {
        FdFlags::empty()
    };
    let fd = fd_add_result(result, fd_flags)?;
    Ok(fd as _)
}

#[syscall_trace]
pub fn sys_dup(old_fd: c_int) -> LinuxResult<isize> {
    let old_file_like = fd_lookup(old_fd as _)?;
    // The two file descriptors do not share file descriptor flags (the close-on-exec flag).
    // The close-on-exec flag (FD_CLOEXEC; see `fcntl`) for the duplicate descriptor is off.
    Ok(fd_add(old_file_like, FdFlags::empty())? as _)
}

#[syscall_trace]
pub fn sys_dup2(old_fd: c_int, new_fd: c_int) -> LinuxResult<isize> {
    // If old_fd is not a valid file descriptor, then the call fails,
    // and new_fd is not closed.
    let old_file_like = fd_lookup(old_fd as _)?;
    if old_fd == new_fd {
        // If old_fd is a valid file descriptor, and new+fd has the same value as old_fd,
        // then dup2() does nothing, and returns new_fd.
        return Ok(new_fd as _);
    }
    // If the file descriptor new_fd was previously open, it is closed  before being reused;
    // the close is performed silently (i.e., any errors during the close are not reported by dup2()).
    let _ = fd_remove(new_fd as _);
    fd_add_at(new_fd, old_file_like, FdFlags::empty())?;
    Ok(new_fd as _)
}

#[syscall_trace]
pub fn sys_dup3(old_fd: c_int, new_fd: c_int, flags: c_int) -> LinuxResult<isize> {
    let old_file_like = fd_lookup(old_fd as _)?;
    if old_fd == new_fd {
        // If old_fd equals new_fd, then dup3() fails with the error EINVAL.
        return Err(LinuxError::EINVAL);
    }
    let _ = fd_remove(new_fd as _);
    // The caller can force the close-on-exec flag to be set for the new file descriptor
    // by specifying O_CLOEXEC in flags.
    let fd_flags = FdFlags::from_bits_truncate(flags as _);
    fd_add_at(new_fd, old_file_like, fd_flags)?;
    Ok(new_fd as _)
}

#[syscall_trace]
pub fn sys_close(fd: c_int) -> LinuxResult<isize> {
    fd_remove(fd as _)?;
    Ok(0)
}

#[syscall_trace]
pub fn sys_fcntl(fd: c_int, op: c_int, arg: isize) -> LinuxResult<isize> {
    let fd = fd as FileDescriptor;
    let file_like = fd_lookup(fd)?;
    let op = op as u32;
    match op {
        F_DUPFD | F_DUPFD_CLOEXEC => {
            // Duplicate the file descriptor fd using the lowest-numbered available file descriptor
            // greater than or equal to arg.
            let mut new_fd = arg as FileDescriptor;
            if new_fd < 0 {
                return Err(LinuxError::EINVAL);
            }
            let fd_flags = if op == F_DUPFD_CLOEXEC {
                FdFlags::CLOSE_ON_EXEC
            } else {
                FdFlags::empty()
            };
            let limit = current_process_data()
                .resource_limits
                .lock()
                .get_soft(&ResourceLimitType::NOFILE) as FileDescriptor;
            while new_fd < limit {
                if fd_add_at(new_fd, file_like.clone(), fd_flags).is_ok() {
                    return Ok(new_fd as _);
                }
                new_fd += 1;
            }
            return Err(LinuxError::EMFILE);
        }
        F_GETFD => {
            // Get the file descriptor flags for fd.
            return Ok(fd_get_flags(fd)?.bits() as _);
        }
        F_SETFD => {
            // Set the file descriptor flags for fd to arg.
            let flags = FdFlags::from_bits_truncate(arg as _);
            fd_set_flags(fd, flags)?;
            return Ok(0);
        }
        F_GETFL => {
            // Get the file status flags for fd.
            let file_flags = file_like.get_flags();
            return Ok(from_file_flags(file_flags) as _);
        }
        F_SETFL => {
            // Set the file status flags for fd to arg.
            let flags = to_file_flags(arg as _);
            file_like.set_flags(flags);
            return Ok(0);
        }
        _ => {
            warn!("unimplemented fcntl operation: {}", op);
            // The value specified in op is not recognized by this kernel.
            return Err(LinuxError::EINVAL);
        }
    }
}
