use crate::core::fs::fd::FileLike;
use crate::utils::task::task_yield_interruptable;
use alloc::sync::Arc;
use alloc::vec;
use axerrno::{LinuxError, LinuxResult};
use axfs_ng::api::FileFlags;
use axio::PollState;
use axsync::Mutex;
use core::any::Any;
use undefined_vfs::types::{Metadata, NodePermission, NodeType};

// TODO: 添加驱动程序，以拦截ctrl+c等控制字符
fn console_read_bytes(buf: &mut [u8]) -> LinuxResult<usize> {
    // we must make sure the buffer is in kernel memory
    let mut kernel_buf = vec![0u8; buf.len()];
    let len = axhal::console::read_bytes(&mut kernel_buf);
    buf.copy_from_slice(&kernel_buf);
    for c in &mut buf[..len] {
        if *c == b'\r' {
            *c = b'\n';
        }
    }
    Ok(len)
}

fn console_write_bytes(buf: &[u8]) -> LinuxResult<usize> {
    axhal::console::write_bytes(buf);
    Ok(buf.len())
}

struct StdinRaw;
struct StdoutRaw;

impl StdinRaw {
    // Non-blocking read, returns number of bytes read.
    fn read(&mut self, buf: &mut [u8]) -> LinuxResult<usize> {
        let mut read_len = 0;
        while read_len < buf.len() {
            let len = console_read_bytes(buf[read_len..].as_mut())?;
            if len == 0 {
                break;
            }
            read_len += len;
        }
        Ok(read_len)
    }
}

impl StdoutRaw {
    fn write(&mut self, buf: &[u8]) -> LinuxResult<usize> {
        console_write_bytes(buf)
    }

    fn flush(&mut self) -> LinuxResult {
        Ok(())
    }
}

#[derive(Default)]
struct StdinBuffer {
    buffer: [u8; 1],
    available: bool,
}

pub struct Stdin {
    inner: &'static Mutex<StdinRaw>,
    buffer: Mutex<StdinBuffer>,
}

impl Stdin {
    // Block until at least one byte is read.
    fn read_blocked(&self, buf: &mut [u8]) -> LinuxResult<usize> {
        // make sure buf[0] is valid
        if buf.is_empty() {
            return Ok(0);
        }
        let mut read_len = 0;
        let mut stdin_buffer = self.buffer.lock();
        let buf = if stdin_buffer.available {
            buf[0] = stdin_buffer.buffer[0];
            read_len += 1;
            stdin_buffer.available = false;
            &mut buf[1..]
        } else {
            buf
        };
        drop(stdin_buffer);
        read_len += self.inner.lock().read(buf)?;
        if read_len > 0 {
            return Ok(read_len);
        }
        // read_len == 0, try again until we get something
        loop {
            let read_len = self.inner.lock().read(buf)?;
            if read_len > 0 {
                return Ok(read_len);
            }
            // TODO: 直接打断是否合理？
            task_yield_interruptable()?;
        }
    }
}

impl Stdin {
    fn read(&mut self, buf: &mut [u8]) -> LinuxResult<usize> {
        self.read_blocked(buf)
    }
}

pub struct Stdout {
    inner: &'static Mutex<StdoutRaw>,
}

impl Stdout {
    fn write(&mut self, buf: &[u8]) -> LinuxResult<usize> {
        self.inner.lock().write(buf)
    }

    fn flush(&mut self) -> LinuxResult {
        self.inner.lock().flush()
    }
}

/// Constructs a new handle to the standard input of the current process.
pub fn stdin() -> Stdin {
    static INSTANCE: Mutex<StdinRaw> = Mutex::new(StdinRaw);
    Stdin {
        inner: &INSTANCE,
        buffer: Default::default(),
    }
}

/// Constructs a new handle to the standard output of the current process.
pub fn stdout() -> Stdout {
    static INSTANCE: Mutex<StdoutRaw> = Mutex::new(StdoutRaw);
    Stdout { inner: &INSTANCE }
}

// TODO: impl get/set file flags

impl FileLike for Stdin {
    fn read(&self, buf: &mut [u8]) -> LinuxResult<usize> {
        Ok(self.read_blocked(buf)?)
    }

    fn write(&self, _buf: &[u8]) -> LinuxResult<usize> {
        Err(LinuxError::EPERM)
    }

    fn status(&self) -> LinuxResult<Metadata> {
        // TODO: add more fields to Metadata as needed
        let mode = NodePermission::from_bits_truncate(0o620);
        Ok(Metadata {
            device: 60,
            inode: 6,
            n_link: 1,
            mode,
            node_type: NodeType::CharacterDevice,
            uid: 1000,
            ..Default::default()
        })
    }

    fn poll(&self) -> LinuxResult<PollState> {
        // try unblocking read
        let mut buf = [0u8; 1];
        let read_len = self.inner.lock().read(&mut buf)?;
        let readable = read_len > 0;
        if readable {
            // if we read something, we should store it in the buffer
            let mut stdin_buffer = self.buffer.lock();
            stdin_buffer.buffer[0] = buf[0];
            stdin_buffer.available = true;
        }
        Ok(PollState {
            readable,
            writable: true,
        })
    }

    fn get_flags(&self) -> FileFlags {
        warn!("get flags for stdin is not implemented");
        FileFlags::empty()
    }

    fn set_flags(&self, _flags: FileFlags) {
        warn!("set flags for stdin is not implemented");
    }

    fn into_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync> {
        self
    }
}

impl FileLike for Stdout {
    fn read(&self, _buf: &mut [u8]) -> LinuxResult<usize> {
        Err(LinuxError::EPERM)
    }

    fn write(&self, buf: &[u8]) -> LinuxResult<usize> {
        Ok(self.inner.lock().write(buf)?)
    }

    fn status(&self) -> LinuxResult<Metadata> {
        // TODO: add more fields to Metadata as needed
        let mode = NodePermission::from_bits_truncate(0o620);
        Ok(Metadata {
            device: 60,
            inode: 6,
            n_link: 1,
            mode,
            node_type: NodeType::CharacterDevice,
            uid: 1000,
            ..Default::default()
        })
    }

    fn poll(&self) -> LinuxResult<PollState> {
        Ok(PollState {
            readable: false,
            writable: true,
        })
    }

    fn get_flags(&self) -> FileFlags {
        warn!("get flags for stdout is not implemented");
        FileFlags::empty()
    }

    fn set_flags(&self, _flags: FileFlags) {
        warn!("set flags for stdout is not implemented");
    }

    fn into_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync> {
        self
    }
}
