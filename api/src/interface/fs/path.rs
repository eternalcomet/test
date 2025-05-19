use crate::imp::fs::path::*;
use crate::ptr::UserInPtr;
use axerrno::{LinuxError, LinuxResult};
use core::ffi::{c_char, c_int, c_uint};
use linux_raw_sys::general::{AT_FDCWD, AT_REMOVEDIR};
use syscall_trace::syscall_trace;

#[syscall_trace]
pub fn sys_rename(old_path: UserInPtr<c_char>, new_path: UserInPtr<c_char>) -> LinuxResult<isize> {
    sys_rename_impl(
        AT_FDCWD,
        old_path.get_as_str()?,
        AT_FDCWD,
        new_path.get_as_str()?,
        RenameFlags::empty(),
    )
}

#[syscall_trace]
pub fn sys_renameat(
    old_dir_fd: c_int,
    old_path: UserInPtr<c_char>,
    new_dir_fd: c_int,
    new_path: UserInPtr<c_char>,
) -> LinuxResult<isize> {
    sys_rename_impl(
        old_dir_fd,
        old_path.get_as_str()?,
        new_dir_fd,
        new_path.get_as_str()?,
        RenameFlags::empty(),
    )
}

#[syscall_trace]
pub fn sys_renameat2(
    old_dir_fd: c_int,
    old_path: UserInPtr<c_char>,
    new_dir_fd: c_int,
    new_path: UserInPtr<c_char>,
    flags: c_uint,
) -> LinuxResult<isize> {
    let flags = RenameFlags::from_bits(flags).ok_or(LinuxError::EINVAL)?;
    sys_rename_impl(
        old_dir_fd,
        old_path.get_as_str()?,
        new_dir_fd,
        new_path.get_as_str()?,
        flags,
    )
}

#[syscall_trace]
pub fn sys_mkdir(path_name: UserInPtr<c_char>, mode: c_uint) -> LinuxResult<isize> {
    let path_name = path_name.get_as_str()?;
    let mode = mode as u16;
    sys_mkdir_impl(AT_FDCWD, path_name, mode)
}

#[syscall_trace]
pub fn sys_mkdirat(
    dir_fd: c_int,
    path_name: UserInPtr<c_char>,
    mode: c_uint,
) -> LinuxResult<isize> {
    let path_name = path_name.get_as_str()?;
    let mode = mode as u16;
    sys_mkdir_impl(dir_fd, path_name, mode)
}

#[syscall_trace]
pub fn sys_unlink(path_name: UserInPtr<c_char>) -> LinuxResult<isize> {
    let path_name = path_name.get_as_str()?;
    sys_unlink_impl(AT_FDCWD, path_name, UnlinkFlags::NO_REMOVE_DIR)
}

#[syscall_trace]
pub fn sys_unlinkat(
    dir_fd: c_int,
    path_name: UserInPtr<c_char>,
    flags: c_uint,
) -> LinuxResult<isize> {
    let path_name = path_name.get_as_str()?;
    let flags = if flags & AT_REMOVEDIR != 0 {
        UnlinkFlags::empty()
    } else {
        UnlinkFlags::NO_REMOVE_DIR
    };
    sys_unlink_impl(dir_fd, path_name, flags)
}

#[syscall_trace]
pub fn sys_rmdir(path_name: UserInPtr<c_char>) -> LinuxResult<isize> {
    let path_name = path_name.get_as_str()?;
    sys_unlink_impl(AT_FDCWD, path_name, UnlinkFlags::NO_REMOVE_FILE)
}
