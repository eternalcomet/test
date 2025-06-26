//! Provides basic packaging for file system operations,
//! adding support for file open flags and permission checks.

use crate::api::FileFlags;
use axio::SeekFrom;
use lock_api::RawMutex;
use undefined_vfs::mount::Location;
use undefined_vfs::node::FileNode;
use undefined_vfs::types::Metadata;
use undefined_vfs::{VfsError, VfsResult};

pub struct File<M> {
    location: Location<M>,
    flags: FileFlags,
    position: u64,
}

impl<M: RawMutex> File<M> {
    pub fn new(location: Location<M>, flags: FileFlags) -> Self {
        let position = if flags.contains(FileFlags::APPEND) {
            // Start at the end of the file for append mode
            location.size().unwrap_or(0)
        } else {
            0 // Start at the beginning for other modes
        };
        Self {
            location,
            flags,
            position,
        }
    }

    pub fn location(&self) -> &Location<M> {
        &self.location
    }

    pub fn access(&self, permission: FileFlags) -> VfsResult<&FileNode<M>> {
        if permission.is_empty() || self.flags.contains(permission) {
            self.location.entry().as_file()
        } else {
            Err(VfsError::EACCES)
        }
    }

    pub fn seek(&mut self, pos: SeekFrom) -> VfsResult<u64> {
        // TODO: Linux lseek() allows the file offset to be set beyond the end of
        //     the file (but this does not change the size of the file).
        //     This leads to a sparse file, which is not supported by this implementation.
        let new_pos = match pos {
            SeekFrom::Start(pos) => pos,
            SeekFrom::End(off) => {
                let size = self.location.size()?;
                size.checked_add_signed(off)
                    .ok_or(VfsError::EINVAL)?
                    .clamp(0, size)
            }
            SeekFrom::Current(off) => {
                let size = self.location.size()?;
                self.position
                    .checked_add_signed(off)
                    .ok_or(VfsError::EINVAL)?
                    .clamp(0, size)
            }
        };
        // File under append mode is seekable, but `write` operations
        // will ignore it and always write at the end.
        self.position = new_pos;
        Ok(new_pos)
    }

    /// Attempts to sync OS-internal file content and metadata to disk.
    ///
    /// If `data_only` is `true`, only the file data is synced, not the metadata.
    pub fn sync(&self, data_only: bool) -> VfsResult<()> {
        self.access(FileFlags::empty())?.sync(data_only)
    }

    /// Truncates or extends the underlying file, updating the size of this file to become `size`.
    pub fn resize(&self, size: u64) -> VfsResult<()> {
        self.access(FileFlags::WRITE)?.resize(size)
    }

    /// Queries metadata about the underlying file.
    pub fn metadata(&self) -> VfsResult<Metadata> {
        self.access(FileFlags::READ)?;
        self.location.metadata()
    }

    /// Reads a number of bytes starting from a given offset.
    /// Returns the number of bytes read.
    pub fn read_at(&mut self, buf: &mut [u8], offset: u64) -> VfsResult<usize> {
        self.access(FileFlags::READ)?.read_at(buf, offset)
    }

    /// Writes a number of bytes starting from a given offset.
    /// Returns the number of bytes written.
    pub fn write_at(&mut self, buf: &[u8], offset: u64) -> VfsResult<usize> {
        self.access(FileFlags::WRITE)?.write_at(buf, offset)
    }

    /// Reads a number of bytes starting from the current position.
    /// Returns the number of bytes read.
    pub fn read(&mut self, buf: &mut [u8]) -> VfsResult<usize> {
        let n = self.read_at(buf, self.position)?;
        self.position += n as u64;
        Ok(n)
    }

    /// Writes a number of bytes starting from the current position.
    /// Returns the number of bytes written.
    pub fn write(&mut self, buf: &[u8]) -> VfsResult<usize> {
        if self.flags.contains(FileFlags::APPEND) {
            // 如果是追加模式，则忽略当前的position，直接写入到文件末尾
            let (written, offset) = self.access(FileFlags::WRITE)?.append(buf)?;
            self.position = offset;
            Ok(written)
        } else {
            let n = self.write_at(buf, self.position)?;
            self.position += n as u64;
            Ok(n)
        }
    }

    // TODO: set flags with checks
    pub fn get_flags(&self) -> FileFlags {
        self.flags
    }
}
