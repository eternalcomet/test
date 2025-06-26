use crate::core::time::TimeSpec;
use crate::ptr::{UserConstPtr, UserInPtr, UserOutPtr, UserPtr};
use crate::utils::task::{task_sleep_interruptable, task_yield};
use axerrno::{LinuxError, LinuxResult};
use axtask::{current, AxCpuMask};
use syscall_trace::syscall_trace;

#[syscall_trace]
pub fn sys_sched_yield() -> LinuxResult<isize> {
    task_yield();
    Ok(0)
}

#[syscall_trace]
pub fn sys_nanosleep(
    requested: UserInPtr<TimeSpec>,
    remain: UserOutPtr<TimeSpec>,
) -> LinuxResult<isize> {
    let duration = requested.get_as_ref()?.to_duration()?;
    let now = axhal::time::monotonic_time();
    if let Err(LinuxError::EINTR) = task_sleep_interruptable(duration) {
        if let Ok(remain) = remain.get_as_mut_ref() {
            let after = axhal::time::monotonic_time();
            let elapsed = after - now;
            *remain = elapsed.into();
        }
        return Err(LinuxError::EINTR);
    }
    Ok(0)
}
pub fn sys_sched_getaffinity
(
    pid: i32,
    cpusetsize: usize,
    user_mask: UserPtr<u8>,
) -> LinuxResult<isize> {
    if cpusetsize * 8 < axconfig::SMP {
        return Err(LinuxError::EINVAL);
    }

    // TODO: support other threads
    if pid != 0 {
        return Err(LinuxError::EPERM);
    }

    let mask = current().cpumask();
    let mask_bytes = mask.as_bytes();
    user_mask
        .get_as_mut_slice(mask_bytes.len())?
        .copy_from_slice(mask_bytes);

    Ok(0)
}

pub fn sys_sched_setaffinity(
    pid: i32,
    cpusetsize: usize,
    user_mask: UserConstPtr<u8>,
) -> LinuxResult<isize> {
    let size = cpusetsize.min(axconfig::SMP.div_ceil(8));
    let user_mask = user_mask.get_as_slice(size)?;
    let mut cpu_mask = AxCpuMask::new();

    for i in 0..(size * 8).min(axconfig::SMP) {
        if user_mask[i / 8] & (1 << (i % 8)) != 0 {
            cpu_mask.set(i, true);
        }
    }

    // TODO: support other threads
    if pid != 0 {
        return Err(LinuxError::EPERM);
    }
    axtask::set_current_affinity(cpu_mask);

    Ok(0)
}