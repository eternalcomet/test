use crate::core::fs::FsLocation;
use crate::core::fs::dir::Directory;
use crate::core::fs::fd::{FdFlags, FileDescriptor, FileLike, fd_add, fd_lookup};
use crate::core::fs::file::File;
use alloc::sync::Arc;
use axerrno::{LinuxError, LinuxResult};
use axfs_ng::api::{FS_CONTEXT, FsContext, OpenResult, resolve_path, resolve_path_existed};
use axsync::{MutexGuard, RawMutex};
use bitflags::bitflags;
use linux_raw_sys::general::{AT_EMPTY_PATH, AT_FDCWD, AT_NO_AUTOMOUNT, AT_SYMLINK_NOFOLLOW};
use undefined_vfs::path::{Path, PathBuf};

// TODO: 使用thread_data
pub fn get_fs_context() -> MutexGuard<'static, FsContext<RawMutex>> {
    FS_CONTEXT.lock()
}

pub fn change_current_dir(path: impl AsRef<Path>) -> LinuxResult<()> {
    let mut context = get_fs_context();
    let dir = resolve_path(&context, &path, &mut 0, true)?;
    context.change_dir(dir)?;
    Ok(())
}

pub enum Resolve {
    FileLike(Arc<dyn FileLike>),
    Location(FsLocation),
}

impl Resolve {
    pub fn location(&self) -> Option<FsLocation> {
        match self {
            Resolve::FileLike(file_like) => file_like.location(),
            Resolve::Location(location) => Some(location.clone()),
        }
    }
}

bitflags! {
    pub struct ResolveFlags: u32 {
        const NO_FOLLOW = AT_SYMLINK_NOFOLLOW;
        const NO_AUTOMOUNT = AT_NO_AUTOMOUNT;
        const EMPTY_PATH = AT_EMPTY_PATH;
    }
}

pub fn resolve_path_at_cwd(path: Option<impl AsRef<Path>>) -> LinuxResult<FsLocation> {
    let path = path.as_ref().map(AsRef::as_ref);
    let path = path.unwrap_or("".as_ref());
    let context = get_fs_context();
    resolve_path(&context, path, &mut 0, true)
}

/// 为Linux的xxxat系统调用解析路径
pub fn resolve_path_at(
    parent_fd: FileDescriptor,
    path: Option<impl AsRef<Path>>,
    flags: ResolveFlags,
) -> LinuxResult<Resolve> {
    let path = path.as_ref().map(AsRef::as_ref);
    if path.is_none() || (path.unwrap().is_empty() && flags.contains(ResolveFlags::EMPTY_PATH)) {
        // 此时parent_fd对应的不一定是一个目录，可以是任何类型的文件描述符
        // 相当于对这个文件描述符进行操作
        let file_like = fd_lookup(parent_fd)?;
        return Ok(Resolve::FileLike(file_like));
    }
    let path = path.unwrap();
    let context = get_fs_context();
    let no_follow = flags.contains(ResolveFlags::NO_FOLLOW);
    let location = if parent_fd == AT_FDCWD {
        resolve_path(&context, path, &mut 0, no_follow)
    } else {
        let dir = Directory::from_fd(parent_fd)?;
        let context = context.with_current_dir(dir.inner().location().clone())?;
        resolve_path(&context, path, &mut 0, no_follow)
    }?;
    Ok(Resolve::Location(location))
}

pub fn resolve_path_at_existed(
    parent_fd: FileDescriptor,
    path: impl AsRef<Path>,
) -> LinuxResult<(FsLocation, PathBuf)> {
    let context = get_fs_context();
    let path = path.as_ref();
    let (location, rest) = if parent_fd == AT_FDCWD {
        resolve_path_existed(&context, path, &mut 0)
    } else {
        let dir = Directory::from_fd(parent_fd)?;
        let context = context.with_current_dir(dir.inner().location().clone())?;
        resolve_path_existed(&context, path, &mut 0)
    };
    let rest_path = rest.normalize().ok_or(LinuxError::ENOENT)?;
    if rest_path.as_str().find('/').is_some() {
        return Err(LinuxError::ENOENT);
    }
    Ok((location, rest_path))
}

pub fn fd_add_result(
    open_result: OpenResult<RawMutex>,
    fd_flags: FdFlags,
) -> LinuxResult<FileDescriptor> {
    let file_like: Arc<dyn FileLike> = match open_result {
        OpenResult::File(file) => Arc::new(File::new(file)),
        OpenResult::Directory(dir) => Arc::new(Directory::new(dir)),
    };
    fd_add(file_like, fd_flags)
}
