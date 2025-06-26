use crate::imp::fs::sys_open_impl;
use crate::ptr::UserInPtr;
use axerrno::LinuxResult;
use core::ffi::{c_char, c_uint};
use linux_raw_sys::general::AT_FDCWD;
use syscall_trace::syscall_trace;

#[syscall_trace]
pub fn sys_openat(
    dir_fd: i32,
    path: UserInPtr<c_char>,
    flags: u32,
    modes: c_uint,
) -> LinuxResult<isize> {
    let path = path.get_as_str()?;
    Ok(sys_open_impl(dir_fd, path.as_ref(), flags, modes)? as _)
}

#[syscall_trace]
pub fn sys_open(path: UserInPtr<c_char>, flags: i32, modes: c_uint) -> LinuxResult<isize> {
    let path = path.get_as_str()?;
    Ok(sys_open_impl(AT_FDCWD, path.as_ref(), flags as u32, modes)? as _)
}
