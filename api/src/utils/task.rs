use axerrno::LinuxResult;
use core::time::Duration;

pub fn task_yield() {
    axtask::yield_now();
}

/// Provides a signal-interruptible yield function for tasks.
pub fn task_yield_interruptable() -> LinuxResult {
    axtask::yield_now();
    // TODO: check signals, if any are pending, return LinuxError::EINTR
    Ok(())
}
pub fn task_sleep(duration: Duration) {
    axtask::sleep(duration);
}

pub fn task_sleep_interruptable(duration: Duration) -> LinuxResult {
    axtask::sleep(duration);
    // TODO: check signals, if any are pending, return LinuxError::EINTR
    Ok(())
}
