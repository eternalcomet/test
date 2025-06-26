use crate::ptr::{PtrWrapper, UserInPtr};
use axerrno::{LinuxError, LinuxResult};
use core::sync::atomic::Ordering;
use num_enum::TryFromPrimitive;
use starry_core::task::{current_process, current_thread, current_thread_data};
use syscall_trace::syscall_trace;
use undefined_process::process::get_process;

/// ARCH_PRCTL codes
///
/// It is only avaliable on x86_64, and is not convenient
/// to generate automatically via c_to_rust binding.
#[derive(Debug, Eq, PartialEq, TryFromPrimitive)]
#[repr(i32)]
enum ArchPrctlCode {
    /// Set the GS segment base
    SetGs = 0x1001,
    /// Set the FS segment base
    SetFs = 0x1002,
    /// Get the FS segment base
    GetFs = 0x1003,
    /// Get the GS segment base
    GetGs = 0x1004,
    /// The setting of the flag manipulated by ARCH_SET_CPUID
    GetCpuid = 0x1011,
    /// Enable (addr != 0) or disable (addr == 0) the cpuid instruction for the calling thread.
    SetCpuid = 0x1012,
}

pub fn sys_setpgid(pid: u32, pgid: u32) -> LinuxResult<isize> {
    let process = if pid == 0 {
        current_process()
    } else {
        if pid == current_process().get_pid(){
            current_process()
        }else{
            current_process().get_child(pid).ok_or(LinuxError::ESRCH)?.clone()
        }
    };
    if pgid == 0 {
        process.create_group();
    } else if pgid < 0 || pgid > 4194304 {
        return Err(LinuxError::EINVAL);
    } else {
        if !process.move_to_group(pgid) {
            return Err(LinuxError::EPERM);
        }
    }
    Ok(0)
}
pub fn sys_getpgid(pid: u32) -> LinuxResult<isize> {
    let process = if pid == 0 {
        current_process()
    } else {
        get_process(pid).ok_or(LinuxError::ESRCH)?
    };
    Ok(process.get_group().get_pgid() as _)
}

#[syscall_trace]
pub fn sys_getpid() -> LinuxResult<isize> {
    Ok(current_process().get_pid() as _)
}

#[syscall_trace]
pub fn sys_getppid() -> LinuxResult<isize> {
    Ok(match current_process().get_parent() {
        Some(p) => p.get_pid() as _,
        None => 0,
    })
}

#[syscall_trace]
pub fn sys_gettid() -> LinuxResult<isize> {
    Ok(current_thread().get_tid() as _)
}

/// To set the clear_child_tid field in the task extended data.
///
/// The set_tid_address() always succeeds
#[syscall_trace]
pub fn sys_set_tid_address(tid_ptd: UserInPtr<i32>) -> LinuxResult<isize> {
    let addr = &current_thread_data().addr_clear_child_tid;
    addr.store(tid_ptd.address().as_ptr() as _, Ordering::Relaxed);
    Ok(current_thread().get_tid() as _)
}

#[cfg(target_arch = "x86_64")]
use axhal::arch::TrapFrame;
#[cfg(target_arch = "x86_64")]
pub fn sys_arch_prctl(
    code: i32,
    addr: UserInOutPtr<u64>,
    tf: &mut TrapFrame,
) -> LinuxResult<isize> {
    use axerrno::LinuxError;
    debug!(
        "arch_prctl: code = {:?}, addr = {:#x}",
        ArchPrctlCode::try_from(code),
        addr.address().as_usize()
    );
    match ArchPrctlCode::try_from(code).map_err(|_| LinuxError::EINVAL)? {
        // According to Linux implementation, SetFs & SetGs does not return
        // error at all
        ArchPrctlCode::SetFs => {
            tf.set_tls(addr.address().as_usize());
            Ok(0)
        }
        ArchPrctlCode::SetGs => {
            unsafe {
                x86::msr::wrmsr(x86::msr::IA32_KERNEL_GSBASE, addr.address().as_usize() as _);
            }
            Ok(0)
        }
        ArchPrctlCode::GetFs => {
            unsafe {
                // *addr.get()? = axhal::arch::read_thread_pointer() as u64;
                *addr.get()? = tf.tls() as u64;
            }
            Ok(0)
        }

        ArchPrctlCode::GetGs => {
            unsafe {
                *addr.get()? = x86::msr::rdmsr(x86::msr::IA32_KERNEL_GSBASE);
            }
            Ok(0)
        }
        ArchPrctlCode::GetCpuid => Ok(0),
        ArchPrctlCode::SetCpuid => Err(LinuxError::ENODEV),
    }
}
