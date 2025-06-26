use axerrno::LinuxResult;

use crate::ptr::{PtrWrapper, UserPtr};

const OS_NAME: &str = "UndefinedOS";
const OS_VERSION: &str = "10.0.0";

#[repr(C)]
pub struct UtsName {
    /// sysname
    pub sysname: [u8; 65],
    /// nodename
    pub nodename: [u8; 65],
    /// release
    pub release: [u8; 65],
    /// version
    pub version: [u8; 65],
    /// machine
    pub machine: [u8; 65],
    /// domainname
    pub domainname: [u8; 65],
}

impl Default for UtsName {
    fn default() -> Self {
        Self {
            sysname: Self::from_str(OS_NAME),
            nodename: Self::from_str("UndefinedOS-DESKTOP"),
            release: Self::from_str(OS_VERSION),
            version: Self::from_str(OS_VERSION),
            machine: Self::from_str(option_env!("ARCH").unwrap_or("riscv64")),
            domainname: Self::from_str("localdomain"),
        }
    }
}

impl UtsName {
    fn from_str(info: &str) -> [u8; 65] {
        let mut data: [u8; 65] = [0; 65];
        data[..info.len()].copy_from_slice(info.as_bytes());
        data
    }
}

pub fn sys_uname(name: UserPtr<UtsName>) -> LinuxResult<isize> {
    unsafe { *name.get()? = UtsName::default() };
    Ok(0)
}
