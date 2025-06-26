use crate::core::random::RANDOM_GENERATOR;
use crate::ptr::UserOutPtr;
use axerrno::LinuxResult;
use core::ffi::c_uint;
use syscall_trace::syscall_trace;

#[syscall_trace]
pub fn sys_getrandom(buf: UserOutPtr<u8>, len: usize, _flags: c_uint) -> LinuxResult<isize> {
    let buf = buf.get_as_mut_slice(len)?;
    let mut rand = RANDOM_GENERATOR.lock();
    rand.fill_bytes(buf);
    Ok(len as isize)
}
