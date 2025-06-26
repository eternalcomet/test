use crate::core::fs::fd::FileLike;
use crate::core::fs::file::File;
use axerrno::LinuxResult;

pub fn sys_truncate_impl(file: &File, length: u64) -> LinuxResult<isize> {
    // set file size to length
    file.inner().resize(length)?;
    Ok(0)
}

pub fn sys_read_impl(file_like: &dyn FileLike, buf: &mut [u8]) -> LinuxResult<isize> {
    let read_len = file_like.read(buf)?;
    Ok(read_len as _)
}

pub fn sys_write_impl(file_like: &dyn FileLike, buf: &[u8]) -> LinuxResult<isize> {
    let write_len = file_like.write(buf)?;
    Ok(write_len as _)
}

pub fn sys_pwrite_impl(fd: i32, buf: &[u8], offset: isize) -> LinuxResult<isize> {
    let file = File::from_fd(fd)?;
    let write_len = file.inner().write_at(buf, offset as _)?;
    Ok(write_len as _)
}

pub fn sys_pread_impl(fd: i32, buf: &mut [u8], offset: isize) -> LinuxResult<isize> {
    let file = File::from_fd(fd)?;
    let read_len = file.inner().read_at(buf, offset as _)?;
    Ok(read_len as _)
}
