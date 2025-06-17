use crate::imp::fs::fs::open_file_like;
use crate::imp::utils::path::resolve_path_with_parent;
pub(crate) use arceos_posix_api::{FileStatus, get_file_like};
use axerrno::LinuxResult;
use axhal::time::monotonic_time_nanos;

/// syscall impl: get file status
/// [Availability] Most
/// TODO: add support for symlink
pub fn sys_stat_impl(dir_fd: i32, path: &str, _follow_symlinks: bool) -> LinuxResult<FileStatus> {
    let start_time = monotonic_time_nanos();
    let file = if path.is_empty() {
        get_file_like(dir_fd)?
    } else {
        let path = resolve_path_with_parent(dir_fd, path)?;
        open_file_like(path.as_str(), None)?
    };
    let end_time = monotonic_time_nanos();
    error!("[fstatat] open took {} ns", end_time - start_time);
    error!("[fstatat] [{}] impl open end", monotonic_time_nanos());
    let file_status = file.stat()?;
    error!("[fstatat] [{}] impl stat end", monotonic_time_nanos());
    Ok(file_status)
}
