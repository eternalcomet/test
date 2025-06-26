#![no_std]

extern crate alloc;

pub mod fs;
pub mod mount;
pub mod node;
pub mod path;
pub mod types;

pub type VfsError = axerrno::LinuxError;
pub type VfsResult<T> = Result<T, VfsError>;
