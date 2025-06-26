use crate::core::fs::fd::FileLike;
use crate::core::fs::{ApiDir, FsLocation};
use alloc::sync::Arc;
use axerrno::{LinuxError, LinuxResult};
use axfs_ng::api::FileFlags;
use axio::PollState;
use axsync::Mutex;
use core::any::Any;
use undefined_vfs::types::Metadata;

pub struct Directory {
    inner: Mutex<ApiDir>,
}

impl Directory {
    pub fn new(inner: ApiDir) -> Self {
        Self {
            inner: Mutex::new(inner),
        }
    }

    pub fn inner(&self) -> axsync::MutexGuard<ApiDir> {
        self.inner.lock()
    }
}

impl FileLike for Directory {
    fn read(&self, _buf: &mut [u8]) -> LinuxResult<usize> {
        Err(LinuxError::EISDIR)
    }
    fn write(&self, _buf: &[u8]) -> LinuxResult<usize> {
        // Not open for writing.
        Err(LinuxError::EBADF)
    }

    fn status(&self) -> LinuxResult<Metadata> {
        self.inner.lock().location().metadata()
    }

    fn poll(&self) -> LinuxResult<PollState> {
        // A directory is always readable and writable.
        Ok(PollState {
            readable: true,
            writable: true,
        })
    }

    fn get_flags(&self) -> FileFlags {
        self.inner.lock().get_flags()
    }

    fn set_flags(&self, _flags: FileFlags) {
        todo!()
    }

    fn into_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync> {
        self
    }

    fn location(&self) -> Option<FsLocation> {
        Some(self.inner.lock().location().clone())
    }
}
