use crate::api::resolve_path;
use alloc::vec;
use alloc::vec::Vec;
use lock_api::RawMutex;
use log::error;
use undefined_vfs::VfsResult;
use undefined_vfs::mount::Location;
use undefined_vfs::path::Path;
use undefined_vfs::types::NodePermission;

/// The default umask value for file creation.
/// rwx, r-x, r-x
pub const DEFAULT_UMASK: u32 = 0o022;

// TODO: replace with thread data
axns::def_resource! {
    pub static FS_CONTEXT: axns::ResArc<axsync::Mutex<FsContext<axsync::RawMutex>>> = axns::ResArc::new();
}

impl FS_CONTEXT {
    pub fn copy_inner(&self) -> axsync::Mutex<FsContext<axsync::RawMutex>> {
        axsync::Mutex::new(self.lock().clone())
    }
}

pub struct FsContext<M> {
    pub root_dir: Location<M>,
    pub current_dir: Location<M>,
    /// Permissions in the umask are turned off from
    /// the mode argument to `open` and `mkdir`.
    pub umask: u32,
    // TODO: 当前使用者的 uid 和 gid等 用于检查权限
}

impl<M> Drop for FsContext<M> {
    fn drop(&mut self) {
        // 这里不需要做任何事情
        // 由于FsContext是一个线程资源，所以它的生命周期由线程决定
        // 当线程结束时，FsContext会被自动释放
        error!("fs context dropped");
    }
}

// 这里面不能自动实现clone是因为锁M没有实现clone

impl<M> Clone for FsContext<M> {
    fn clone(&self) -> Self {
        Self {
            root_dir: self.root_dir.clone(),
            current_dir: self.current_dir.clone(),
            umask: self.umask,
        }
    }
}

impl<M: RawMutex> FsContext<M> {
    pub fn new(root_dir: Location<M>) -> Self {
        Self {
            root_dir: root_dir.clone(),
            current_dir: root_dir,
            umask: DEFAULT_UMASK,
        }
    }

    pub fn change_dir(&mut self, current_dir: Location<M>) -> VfsResult<()> {
        current_dir.check_is_dir()?;
        self.current_dir = current_dir;
        Ok(())
    }

    pub fn change_root(&mut self, root_dir: Location<M>) -> VfsResult<()> {
        root_dir.check_is_dir()?;
        self.root_dir = root_dir.clone();
        self.current_dir = root_dir;
        Ok(())
    }

    pub fn with_current_dir(&self, current_dir: Location<M>) -> VfsResult<Self> {
        let mut context = (*self).clone();
        context.change_dir(current_dir)?;
        Ok(context)
    }

    pub fn get_permissions(&self, mode: u32) -> NodePermission {
        NodePermission::from_bits_truncate((mode & !self.umask) as u16)
    }

    pub fn read(&self, path: impl AsRef<Path>) -> VfsResult<Vec<u8>> {
        let location = resolve_path(self, path, &mut 0, false)?;
        let mut buf = vec![0; location.size()? as _];
        let file = location.entry().as_file()?;
        let length = file.read_at(&mut buf, 0)?;
        debug_assert!(length == buf.len());
        Ok(buf)
    }
}
