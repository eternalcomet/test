use crate::core::fs::fd::FileLike;
use crate::core::fs::{ApiFile, FsLocation};
use alloc::sync::Arc;
use axerrno::LinuxResult;
use axfs_ng::api::FileFlags;
use axio::PollState;
use axsync::{Mutex, MutexGuard};
use core::any::Any;
use undefined_vfs::types::Metadata;

/// File-like wrapper for [axfs_ng::api::File].
pub struct File {
    inner: Mutex<ApiFile>,
}

impl File {
    pub fn new(inner: ApiFile) -> Self {
        Self {
            inner: Mutex::new(inner),
        }
    }

    pub fn inner(&self) -> MutexGuard<ApiFile> {
        self.inner.lock()
    }

    pub fn from_location(location: FsLocation, flags: FileFlags) -> Self {
        let file = ApiFile::new(location, flags);
        Self::new(file)
    }
}

impl FileLike for File {
    fn read(&self, buf: &mut [u8]) -> LinuxResult<usize> {
        Ok(self.inner().read(buf)?)
    }

    fn write(&self, buf: &[u8]) -> LinuxResult<usize> {
        self.inner().write(buf)
    }

    fn status(&self) -> LinuxResult<Metadata> {
        Ok(self.inner().metadata()?)
    }

    fn poll(&self) -> LinuxResult<PollState> {
        // Regular files are always readable and writable, regardless of file flags
        Ok(PollState {
            readable: true,
            writable: true,
        })
    }

    fn get_flags(&self) -> FileFlags {
        self.inner.lock().get_flags()
    }

    fn set_flags(&self, _flags: FileFlags) {
        warn!("set file flags is not implemented for regular files");
    }

    fn into_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync> {
        self
    }

    fn location(&self) -> Option<FsLocation> {
        Some(self.inner().location().clone())
    }
}
