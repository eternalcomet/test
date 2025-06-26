//! POSIX-compatible APIs for [ArceOS] modules
//!
//! [ArceOS]: https://github.com/arceos-org/arceos

#![cfg_attr(all(not(test), not(doc)), no_std)]
#![feature(doc_cfg)]
#![feature(doc_auto_cfg)]
#![allow(clippy::missing_safety_doc)]

#[cfg(feature = "alloc")]
extern crate alloc;
#[macro_use]
extern crate axlog;
extern crate axruntime;

#[macro_use]
mod utils;

mod imp;
pub use utils::char_ptr_to_str;

/// Platform-specific constants and parameters.
pub mod config {
    pub use axconfig::*;
}

/// POSIX C types.
#[rustfmt::skip]
#[path = "./ctypes_gen.rs"]
#[allow(dead_code, non_snake_case, non_camel_case_types, non_upper_case_globals, clippy::upper_case_acronyms, missing_docs)]
pub mod ctypes;

pub use imp::sys::*;
pub use imp::time::*;

#[cfg(feature = "net")]
pub use imp::net::*;
