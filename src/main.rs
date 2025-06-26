#![no_std]
#![no_main]
#![doc = include_str!("../README.md")]

extern crate alloc;
#[macro_use]
extern crate axlog;

pub mod entry;
mod fs;
mod mm;
mod syscall;

use crate::fs::mount::mount_all;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use axfs_ng::api::FS_CONTEXT;
use entry::run_user_app;
use starry_api::core::fs::fd::{FD_TABLE, FdTable};

#[unsafe(no_mangle)]
fn main() {
    let root_dir = axfs_ng::api::FS_CONTEXT.lock().root_dir.clone();
    FS_CONTEXT.lock().change_root(root_dir).unwrap();
    FD_TABLE.init_new(FdTable::new());
    mount_all().expect("Mounting all filesystems failed");

    let testcases = option_env!("AX_TESTCASES_LIST")
        .unwrap_or_else(|| "Please specify the testcases list by making user_apps")
        .split(',')
        .filter(|&x| !x.is_empty());

    let command = testcases.collect::<Vec<_>>().join("\n");
    let args = vec!["/musl/busybox", "sh", "-c", &command];
    let args: Vec<String> = args.into_iter().map(String::from).collect();

    let envs = vec![
        "PATH=/bin".to_string(),
        // "LD_LIBRARY_PATH=/lib:/lib64".to_string(),
        // "LD_DEBUG=all".to_string(),
    ];

    let exit_code = run_user_app(&args, &envs);
    info!("[task manager] Shell exited with code: {:?}", exit_code);
}
