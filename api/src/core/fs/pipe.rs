use crate::core::fs::fd::FileLike;
use crate::core::random::random_u32;
use crate::utils::task::task_yield_interruptable;
use alloc::sync::Arc;
use axerrno::{LinuxError, LinuxResult};
use axfs_ng::api::FileFlags;
use axio::PollState;
use axsync::Mutex;
use core::any::Any;
use undefined_vfs::types::{Metadata, NodePermission, NodeType};

#[derive(Copy, Clone, PartialEq)]
enum RingBufferStatus {
    Full,
    Empty,
    Normal,
}

const RING_BUFFER_SIZE: usize = 1024;

pub struct PipeRingBuffer {
    arr: [u8; RING_BUFFER_SIZE],
    head: usize,
    tail: usize,
    status: RingBufferStatus,
}

impl PipeRingBuffer {
    pub const fn new() -> Self {
        Self {
            arr: [0; RING_BUFFER_SIZE],
            head: 0,
            tail: 0,
            status: RingBufferStatus::Empty,
        }
    }

    pub fn write_byte(&mut self, byte: u8) {
        self.status = RingBufferStatus::Normal;
        self.arr[self.tail] = byte;
        self.tail = (self.tail + 1) % RING_BUFFER_SIZE;
        if self.tail == self.head {
            self.status = RingBufferStatus::Full;
        }
    }

    pub fn read_byte(&mut self) -> u8 {
        self.status = RingBufferStatus::Normal;
        let c = self.arr[self.head];
        self.head = (self.head + 1) % RING_BUFFER_SIZE;
        if self.head == self.tail {
            self.status = RingBufferStatus::Empty;
        }
        c
    }

    /// Get the length of remaining data in the buffer
    pub const fn available_read(&self) -> usize {
        if matches!(self.status, RingBufferStatus::Empty) {
            0
        } else if self.tail > self.head {
            self.tail - self.head
        } else {
            self.tail + RING_BUFFER_SIZE - self.head
        }
    }

    /// Get the length of remaining space in the buffer
    pub const fn available_write(&self) -> usize {
        if matches!(self.status, RingBufferStatus::Full) {
            0
        } else {
            RING_BUFFER_SIZE - self.available_read()
        }
    }
}

pub struct Pipe {
    readable: bool,
    buffer: Arc<Mutex<PipeRingBuffer>>,
    inode: u64,
    // TODO: non-blocking support, flags support, etc.
    file_flags: FileFlags,
}

impl Pipe {
    pub fn new(file_flags: FileFlags) -> (Pipe, Pipe) {
        let buffer = Arc::new(Mutex::new(PipeRingBuffer::new()));
        let inode = random_u32();
        let read_end = Pipe {
            readable: true,
            buffer: buffer.clone(),
            inode: inode as _,
            file_flags,
        };
        let write_end = Pipe {
            readable: false,
            buffer,
            inode: inode as _,
            file_flags,
        };
        (read_end, write_end)
    }

    pub const fn readable(&self) -> bool {
        self.readable
    }

    pub const fn writable(&self) -> bool {
        !self.readable
    }

    pub fn write_end_close(&self) -> bool {
        Arc::strong_count(&self.buffer) == 1
    }

    pub fn non_blocking(&self) -> bool {
        self.file_flags.contains(FileFlags::NON_BLOCK)
    }
}

impl FileLike for Pipe {
    fn read(&self, buf: &mut [u8]) -> LinuxResult<usize> {
        if !self.readable() {
            return Err(LinuxError::EPERM);
        }
        let mut read_size = 0usize;
        let max_len = buf.len();
        loop {
            let mut ring_buffer = self.buffer.lock();
            let loop_read = ring_buffer.available_read();
            if loop_read == 0 {
                if self.write_end_close() || read_size > 0
                /* || non_block */
                {
                    return Ok(read_size);
                }
                drop(ring_buffer);
                // Data not ready, wait for write end
                // TODO: interruptable wait
                task_yield_interruptable()?;
                continue;
            }
            for _ in 0..loop_read {
                if read_size == max_len {
                    return Ok(read_size);
                }
                buf[read_size] = ring_buffer.read_byte();
                read_size += 1;
            }
        }
    }

    fn write(&self, buf: &[u8]) -> LinuxResult<usize> {
        if !self.writable() {
            return Err(LinuxError::EPERM);
        }
        let mut write_size = 0usize;
        let max_len = buf.len();
        loop {
            let mut ring_buffer = self.buffer.lock();
            let loop_write = ring_buffer.available_write();
            if loop_write == 0 {
                drop(ring_buffer);
                // Buffer is full, wait for read end to consume
                task_yield_interruptable()?;
                continue;
            }
            for _ in 0..loop_write {
                if write_size == max_len {
                    return Ok(write_size);
                }
                ring_buffer.write_byte(buf[write_size]);
                write_size += 1;
            }
        }
    }

    fn status(&self) -> LinuxResult<Metadata> {
        // TODO: uid, gid, etc.
        Ok(Metadata {
            inode: self.inode,
            n_link: 1,
            mode: NodePermission::OWNER_READ | NodePermission::OWNER_WRITE,
            node_type: NodeType::Fifo,
            uid: 1000,
            gid: 1000,
            block_size: 4096,
            ..Default::default()
        })
    }

    fn poll(&self) -> LinuxResult<PollState> {
        let buf = self.buffer.lock();
        Ok(PollState {
            readable: self.readable() && buf.available_read() > 0,
            writable: self.writable() && buf.available_write() > 0,
        })
    }

    fn get_flags(&self) -> FileFlags {
        self.file_flags
    }

    fn set_flags(&self, _flags: FileFlags) {
        todo!()
    }

    fn into_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync> {
        self
    }
}
