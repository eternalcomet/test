use crate::core::fs::FsLocation;
use crate::core::fs::stdio::{stdin, stdout};
use alloc::sync::Arc;
use alloc::vec::Vec;
use axerrno::{LinuxError, LinuxResult};
use axfs_ng::api::FileFlags;
use axio::{PollState};
use axns::{ResArc, def_resource};
use bitflags::bitflags;
use core::any::Any;
use core::ops::Deref;
use core::ptr::drop_in_place;
use flatten_objects::FlattenObjects;
use linux_raw_sys::general::FD_CLOEXEC;
use starry_core::resource::{RLIMIT_MAX_FILES, ResourceLimitType};
use starry_core::task::current_process_data;
use undefined_vfs::types::Metadata;

pub type FileDescriptor = i32;

#[allow(dead_code)]
pub trait FileLike: Send + Sync {
    /// Read at current position.
    fn read(&self, buf: &mut [u8]) -> LinuxResult<usize>;
    /// Write at current position.
    fn write(&self, buf: &[u8]) -> LinuxResult<usize>;
    /// Get file status: inode, permission, owner, etc.
    fn status(&self) -> LinuxResult<Metadata>;
    /// Get I/O polling state.
    fn poll(&self) -> LinuxResult<PollState>;
    /// Get file flags initialized when the file was opened.
    fn get_flags(&self) -> FileFlags;
    /// Set file flags. Some flags may be ignored.
    fn set_flags(&self, flags: FileFlags);
    /// Used to downcast the file-like object to a specific type.
    fn into_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync>;

    fn location(&self) -> Option<FsLocation> {
        None
    }

    /// Get the file-like object from file descriptor table.
    fn from_fd(fd: FileDescriptor) -> LinuxResult<Arc<Self>>
    where
        Self: Sized + 'static,
    {
        fd_lookup(fd)?
            .into_any()
            .downcast::<Self>()
            .map_err(|_| LinuxError::EINVAL)
    }
}

pub fn file_like_as<T: FileLike + 'static>(file_like: Arc<dyn FileLike>) -> Option<Arc<T>> {
    file_like.into_any().downcast::<T>().ok()
}

bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct FdFlags: u32 {
        /// Close-on-exec flag.
        const CLOSE_ON_EXEC = FD_CLOEXEC;
    }
}

#[derive(Clone)]
pub struct FdTableItem {
    pub file_like: Arc<dyn FileLike>,
    pub flags: FdFlags,
}

impl FdTableItem {
    pub fn new(file_like: Arc<dyn FileLike>) -> Self {
        Self {
            file_like,
            flags: FdFlags::empty(),
        }
    }
}

pub struct FdTable {
    inner: spin::RwLock<FlattenObjects<FdTableItem, RLIMIT_MAX_FILES>>,
}

impl FdTable {
    pub fn new() -> Self {
        let mut fd_table = FlattenObjects::new();
        // initialize standard IO
        fd_table
            .add_at(0, FdTableItem::new(Arc::new(stdin())))
            .unwrap_or_else(|_| panic!()); // stdin
        fd_table
            .add_at(1, FdTableItem::new(Arc::new(stdout())))
            .unwrap_or_else(|_| panic!()); // stdout
        fd_table
            .add_at(2, FdTableItem::new(Arc::new(stdout())))
            .unwrap_or_else(|_| panic!()); // stderr
        Self {
            inner: spin::RwLock::new(fd_table),
        }
    }

    /// Get a file-like object by `fd`.
    pub fn get(&self, fd: FileDescriptor) -> LinuxResult<Arc<dyn FileLike>> {
        let table = self.inner.read();
        let item = table.get(fd as _).ok_or(LinuxError::EBADF)?;
        Ok(item.file_like.clone())
    }

    /// Get fd table item by `fd`.
    pub fn get_item(&self, fd: FileDescriptor) -> LinuxResult<FdTableItem> {
        let table = self.inner.read();
        let item = table.get(fd as _).ok_or(LinuxError::EBADF)?;
        Ok(item.clone())
    }

    /// Add a file-like object to the table.
    pub fn add(&self, file_like: Arc<dyn FileLike>, flags: FdFlags) -> LinuxResult<FileDescriptor> {
        let mut table = self.inner.write();
        let item = FdTableItem { file_like, flags };
        let fd = table.add(item).map_err(|_| LinuxError::EMFILE)?;
        Ok(fd as FileDescriptor)
    }

    /// Add a file-like object to the table at a specific `fd`, replacing the existing one.
    pub fn add_at(
        &self,
        fd: FileDescriptor,
        file_like: Arc<dyn FileLike>,
        flags: FdFlags,
    ) -> LinuxResult<()> {
        let mut table = self.inner.write();
        let item = FdTableItem { file_like, flags };
        if let Err(None) = table.add_or_replace_at(fd as _, item) {
            // Returns Err(None) if the ID is out of range.
            return Err(LinuxError::EMFILE);
        }
        Ok(())
    }

    /// Remove a file-like object by `fd`.
    pub fn remove(&self, fd: FileDescriptor) -> LinuxResult {
        let mut table = self.inner.write();
        table.remove(fd as _).ok_or(LinuxError::EBADF)?;
        Ok(())
    }

    /// Get current file descriptor count.
    pub fn count(&self) -> usize {
        self.inner.read().count()
    }

    // TODO: change fd flags

    // TODO: optimize
    /// Return a copy of the inner table.
    pub fn copy_inner(&self) -> FdTable {
        let table = self.inner.read();
        let mut new_table = FlattenObjects::new();
        for id in table.ids() {
            let _ = new_table.add_at(id, table.get(id).unwrap().clone());
        }
        Self {
            inner: spin::RwLock::new(new_table),
        }
    }

    // TODO: optimize
    pub fn clear(&self) {
        let mut table = self.inner.write();
        let all_ids: Vec<_> = table.ids().collect();
        for id in all_ids {
            table.remove(id);
        }
    }
}

// TODO: do not use axns
// put the table into ThreadData
def_resource! {
    pub static FD_TABLE: ResArc<FdTable> = ResArc::new();
}

/// Get file-like object from fd table by fd.
pub fn fd_lookup(fd: FileDescriptor) -> LinuxResult<Arc<dyn FileLike>> {
    FD_TABLE.get(fd)
}

/// Get fd flags by fd.
pub fn fd_get_flags(fd: FileDescriptor) -> LinuxResult<FdFlags> {
    let item = FD_TABLE.get_item(fd)?;
    Ok(item.flags)
}

/// Set fd flags.
pub fn fd_set_flags(fd: FileDescriptor, flags: FdFlags) -> LinuxResult<()> {
    FD_TABLE.add_at(fd, FD_TABLE.get(fd)?, flags)
}

/// Add a file-like object to fd table and return its fd.
pub fn fd_add(file_like: Arc<dyn FileLike>, flags: FdFlags) -> LinuxResult<FileDescriptor> {
    let limit = current_process_data()
        .resource_limits
        .lock()
        .get_soft(&ResourceLimitType::NOFILE);
    if FD_TABLE.count() >= limit as _ {
        // Too many open files
        return Err(LinuxError::EMFILE);
    }
    FD_TABLE.add(file_like, flags)
}

/// Add a file-like object to fd table at a specific fd.
pub fn fd_add_at(
    fd: FileDescriptor,
    file_like: Arc<dyn FileLike>,
    flags: FdFlags,
) -> LinuxResult<()> {
    let limit = current_process_data()
        .resource_limits
        .lock()
        .get_soft(&ResourceLimitType::NOFILE);
    if fd >= limit as _ {
        // Too many open files
        return Err(LinuxError::EMFILE);
    }
    FD_TABLE.add_at(fd, file_like, flags)
}

/// Remove a file-like object from fd table by fd.
pub fn fd_remove(fd: FileDescriptor) -> LinuxResult {
    FD_TABLE.remove(fd)
}

pub fn close_all_file_like() {
    let ref_count = FD_TABLE.ref_count();
    debug!("ref count for FD_TABLE is {}", ref_count);

    if ref_count == 1 {
        FD_TABLE.clear();
    }

    let res = FD_TABLE.deref();
    let res_ptr = FD_TABLE::as_ptr(res);
    unsafe {
        drop_in_place(res_ptr);
    }
}
