use crate::core::fs::FsLocation;
use crate::core::fs::fd::FileDescriptor;
use crate::core::time::TimeSpec;
use crate::utils::path::{Resolve, ResolveFlags, resolve_path_at};
use axerrno::LinuxResult;
use core::time::Duration;
use undefined_vfs::types::Metadata;

pub fn sys_stat_impl(
    dir_fd: FileDescriptor,
    path: Option<&str>,
    flags: ResolveFlags,
) -> LinuxResult<Metadata> {
    match resolve_path_at(dir_fd, path, flags)? {
        Resolve::Location(location) => location.metadata(),
        Resolve::FileLike(file_like) => file_like.status(),
    }
}

pub fn sys_statfs_impl(location: &FsLocation, buf: &mut UserStatFs) -> LinuxResult<()> {
    let stat = location.filesystem().stat()?;

    buf.f_type = stat.fs_type as _;
    buf.f_bsize = stat.block_size as isize;
    buf.f_blocks = stat.blocks as usize;
    buf.f_bfree = stat.blocks_free as usize;
    buf.f_bavail = stat.blocks_available as usize;
    buf.f_files = stat.file_count as usize;
    buf.f_ffree = stat.free_file_count as usize;
    // TODO: fsid is incomplete
    buf.f_fsid.val = [0, location.mountpoint().device() as _];
    buf.f_namelen = stat.name_length as isize;
    buf.f_frsize = stat.fragment_size as isize;
    buf.f_flags = stat.mount_flags as isize;

    Ok(())
}

#[repr(C)]
#[derive(Debug, Default)]
pub struct FsStatxTimestamp {
    pub tv_sec: i64,
    pub tv_nsec: u32,
}

impl From<Duration> for FsStatxTimestamp {
    fn from(duration: Duration) -> Self {
        let seconds = duration.as_secs() as i64;
        let nanoseconds = duration.subsec_nanos();
        Self {
            tv_sec: seconds,
            tv_nsec: nanoseconds,
        }
    }
}

impl From<TimeSpec> for FsStatxTimestamp {
    fn from(ts: TimeSpec) -> Self {
        Self {
            tv_sec: ts.seconds,
            tv_nsec: ts.nanoseconds as u32,
        }
    }
}

/// statfs - get filesystem statistics
/// Standard C library (libc, -lc)
/// <https://man7.org/linux/man-pages/man2/statfs.2.html>
#[repr(C)]
#[derive(Debug, Default)]
pub struct UserStatFs {
    /// Type of filesystem (see below)
    pub f_type: FsWord,
    /// Optimal transfer block size
    pub f_bsize: FsWord,
    /// Total data blocks in filesystem
    pub f_blocks: FsBlkCnt,
    /// Free blocks in filesystem
    pub f_bfree: FsBlkCnt,
    /// Free blocks available to unprivileged user
    pub f_bavail: FsBlkCnt,
    /// Total inodes in filesystem
    pub f_files: FsFilCnt,
    /// Free inodes in filesystem
    pub f_ffree: FsFilCnt,
    /// Filesystem ID
    pub f_fsid: FsId,
    /// Maximum length of filenames
    pub f_namelen: FsWord,
    /// Fragment size (since Linux 2.6)
    pub f_frsize: FsWord,
    /// Mount flags of filesystem (since Linux 2.6.36)
    pub f_flags: FsWord,
    /// Padding bytes reserved for future use
    pub f_spare: [FsWord; 5],
}

/// Type of miscellaneous file system fields. (typedef long __fsword_t)
pub type FsWord = isize;

/// Type to count file system blocks. (typedef unsigned long __fsblkcnt_t)
pub type FsBlkCnt = usize;

/// Type to count file system nodes. (typedef unsigned long __fsfilcnt_t)
pub type FsFilCnt = usize;

/// Type of file system IDs.
#[repr(C)]
#[derive(Debug, Default)]
pub struct FsId {
    /// raw value of the ID
    pub val: [i32; 2],
}

pub struct FsType;

impl FsType {
    //const EXT4_SUPER_MAGIC: u32 = 0xEF53;
}
