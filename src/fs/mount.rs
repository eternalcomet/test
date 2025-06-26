use crate::fs::imp::{dev, proc, tmp};
use axerrno::LinuxResult;
use axfs_ng::api::{FS_CONTEXT, resolve_path_existed};
use axsync::RawMutex;
use undefined_vfs::fs::Filesystem;
use undefined_vfs::path::Path;
use undefined_vfs::types::{NodePermission, NodeType};

fn mount_at(path: impl AsRef<Path>, mount_fs: Filesystem<RawMutex>) -> LinuxResult<()> {
    let path = path.as_ref();
    let context = FS_CONTEXT.lock();
    let mode = NodePermission::from_bits_truncate(0o755);
    let (location, name) = resolve_path_existed(&context, &path, &mut 0);
    let mount_point = location.create(name.as_ref(), NodeType::Directory, mode)?;
    mount_point.mount(&mount_fs)?;
    info!("Mounted {} at {}", mount_fs.name(), path);
    Ok(())
}

/// Mount all filesystems
pub fn mount_all() -> LinuxResult<()> {
    mount_at("/dev", dev::new_devfs()?)?;
    mount_at("/tmp", tmp::MemoryFs::new())?;
    mount_at("/proc", proc::new_procfs())?;
    Ok(())
}
