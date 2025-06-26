use crate::api::{Directory, File, FsContext, resolve_path_existed};
use axerrno::LinuxError;
use lock_api::RawMutex;
use undefined_vfs::path::Path;
use undefined_vfs::types::{MetadataUpdate, NodePermission, NodeType};
use undefined_vfs::{VfsError, VfsResult};

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct FileFlags: u32 {
        // Access modes
        const READ          = 0x0001;
        const WRITE         = 0x0002;
        const EXECUTE       = 0x0004;
        // File creation flags
        const CREATE        = 0x0040;
        const CREATE_NEW    = 0x0080;
        const TRUNCATE      = 0x0200;
        const DIRECTORY     = 0x10000;
        // TODO: Add more flags as needed, like NOFOLLOW, TMPFILE, etc.
        // File status flags
        const APPEND        = 0x0400;
        const NON_BLOCK      = 0x0800;
    }
}

impl FileFlags {
    /// Checks if the flags include a read access mode.
    pub fn validate(&self) -> bool {
        if !self.intersects(Self::READ | Self::WRITE | Self::EXECUTE) {
            // If no access mode is set, we assume it's invalid.
            return false;
        }
        if self.contains(Self::CREATE_NEW) && !self.contains(Self::CREATE) {
            // CREATE_NEW and CREATE must be used together.
            return false;
        }
        true
    }
}

pub enum OpenResult<M> {
    File(File<M>),
    Directory(Directory<M>),
}

pub fn open<M: RawMutex>(
    path: impl AsRef<Path>,
    context: &FsContext<M>,
    flags: FileFlags,
    create_mode: Option<u32>,
    create_user: Option<(u32, u32)>,
) -> VfsResult<OpenResult<M>> {
    if !flags.validate() {
        return Err(VfsError::EINVAL);
    }
    let path = path.as_ref();
    // TODO: 每一层的权限检查
    // 默认是当前目录
    let (location, rest) = resolve_path_existed(context, path, &mut 0);
    let file = if rest.is_empty() {
        // 如果路径解析完毕，说明是一个文件或目录，直接打开即可
        if flags.contains(FileFlags::CREATE | FileFlags::CREATE_NEW) {
            return Err(VfsError::EEXIST);
        }
        location
    } else {
        // 路径不存在，需要创建
        if !flags.contains(FileFlags::CREATE) {
            return Err(VfsError::ENOENT);
        }
        let file_path = rest.normalize().ok_or(LinuxError::ENOENT)?;
        if file_path.as_str().find('/').is_some() {
            return Err(VfsError::ENOENT);
        }
        let create_mode = create_mode.unwrap_or(0o666);
        let permission = NodePermission::from_bits_truncate((create_mode & !context.umask) as _);
        let location = location.create(file_path.as_str(), NodeType::RegularFile, permission)?;
        location.update_metadata(MetadataUpdate {
            owner: create_user,
            ..Default::default()
        })?;
        location
    };

    if file.is_dir() && flags.contains(FileFlags::WRITE) {
        return Err(LinuxError::EISDIR);
    }

    if !file.is_dir() && flags.contains(FileFlags::DIRECTORY) {
        return Err(LinuxError::ENOTDIR);
    }

    // TODO: 对最终文件的权限检查，看看flags里面请求的权限是否满足

    if flags.contains(FileFlags::TRUNCATE) {
        file.entry().as_file()?.resize(0)?;
    }

    let result = if file.is_dir() {
        OpenResult::Directory(Directory::new(file, flags))
    } else {
        OpenResult::File(File::new(file, flags))
    };

    Ok(result)
}
