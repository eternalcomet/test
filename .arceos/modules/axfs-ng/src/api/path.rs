use crate::api::FsContext;
use lock_api::RawMutex;
use undefined_vfs::mount::Location;
use undefined_vfs::path::{Component, Path};
use undefined_vfs::types::NodeType;
use undefined_vfs::{VfsError, VfsResult};

pub const SYMLINKS_MAX: usize = 40;

/// 将path解析为文件系统中的位置。
pub fn resolve_path<M: RawMutex>(
    context: &FsContext<M>,
    path: impl AsRef<Path>,
    follow_count: &mut usize,
    no_follow: bool,
) -> VfsResult<Location<M>> {
    let mut follow_symlink = |location: &Location<M>| -> VfsResult<Location<M>> {
        if *follow_count >= SYMLINKS_MAX {
            return Err(VfsError::ELOOP);
        }
        *follow_count += 1;
        let target = location.read_link()?;
        if target.is_empty() {
            return Err(VfsError::ENOENT);
        }
        resolve_path(context, &target, follow_count, false)
    };

    let mut location = &context.current_dir;
    let mut location_owned;
    for component in path.as_ref().components() {
        match component {
            Component::RootDir => location = &context.root_dir,
            Component::CurrentDir => {}
            Component::ParentDir => {
                if let Some(parent) = location.parent() {
                    location_owned = parent;
                    location = &location_owned;
                } else {
                    // Linux认为根目录的父目录是根目录本身
                    location = &context.root_dir;
                }
            }
            Component::Normal(name) => {
                // 检查上次循环结束后的location是不是符号链接，如果是，则需要重新查找
                location_owned = if location.node_type() == NodeType::Symlink {
                    let real_location = follow_symlink(location)?;
                    real_location.lookup_no_follow(name)
                } else {
                    location.lookup_no_follow(name)
                }?;
                location = &location_owned;
                // 先不追踪符号链接，等下次循环再处理
            }
        };
    }
    if !no_follow && location.node_type() == NodeType::Symlink {
        follow_symlink(location)
    } else {
        Ok(location.clone())
    }
}

/// 将path中已经存在的部分解析为文件系统中的位置。
pub fn resolve_path_existed<'a, M: RawMutex>(
    context: &FsContext<M>,
    path: &'a Path,
    follow_count: &mut usize,
) -> (Location<M>, &'a Path) {
    let mut location = &context.current_dir;
    let mut location_owned;
    let mut components = path.components();
    loop {
        let rest_path = components.as_path();
        let component = match components.next() {
            Some(c) => c,
            None => break,
        };
        location = match component {
            Component::RootDir => &context.root_dir,
            Component::CurrentDir => location,
            Component::ParentDir => {
                if let Some(parent) = location.parent() {
                    location_owned = parent;
                    &location_owned
                } else {
                    // Linux认为根目录的父目录是根目录本身
                    &context.root_dir
                }
            }
            Component::Normal(name) => {
                if let Ok(new_location) = lookup_followed(context, location, name, follow_count) {
                    location_owned = new_location;
                    &location_owned
                } else {
                    return (location.clone(), rest_path);
                }
            }
        };
    }
    (location.clone(), "".as_ref())
}

/// 在指定的路径下查找文件或目录，追踪符号链接。
/// follow_count 用于限制符号链接的追踪次数，防止死循环，函数返回时其中保存了符号链接的追踪次数。
pub fn lookup_followed<M: RawMutex>(
    context: &FsContext<M>,
    location: &Location<M>,
    name: &str,
    follow_count: &mut usize,
) -> VfsResult<Location<M>> {
    let location = location.lookup_no_follow(name)?;
    if location.node_type() != NodeType::Symlink {
        return Ok(location);
    }
    if *follow_count >= SYMLINKS_MAX {
        return Err(VfsError::ELOOP);
    }
    *follow_count += 1;
    let target = location.read_link()?;
    if target.is_empty() {
        return Err(VfsError::ENOENT);
    }
    resolve_path(
        &context.with_current_dir(location.clone())?,
        &target,
        follow_count,
        true,
    )
}
