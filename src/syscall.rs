use axerrno::{LinuxError, LinuxResult};
use axhal::{
    arch::TrapFrame,
    trap::{SYSCALL, register_trap_handler},
};
use starry_api::*;
use starry_core::task::{time_stat_from_kernel_to_user, time_stat_from_user_to_kernel};
use syscalls::Sysno;

#[register_trap_handler(SYSCALL)]
fn handle_syscall(tf: &TrapFrame, syscall_num: usize) -> isize {
    info!("[syscall] <{:?}> begin", Sysno::from(syscall_num as u32));
    time_stat_from_user_to_kernel();
    let result: LinuxResult<isize> = match Sysno::from(syscall_num as u32) {
        #[cfg(target_arch = "x86_64")]
        Sysno::access => sys_access(tf.arg0().into(), tf.arg1() as _),
        Sysno::kill => sys_kill(tf.arg0() as _, tf.arg1() as _),
        Sysno::faccessat => sys_faccessat(
            tf.arg0() as _,
            tf.arg1().into(),
            tf.arg2() as _,
            tf.arg3() as _,
        ),
        Sysno::read => sys_read(tf.arg0() as _, tf.arg1().into(), tf.arg2() as _),
        Sysno::write => sys_write(tf.arg0() as _, tf.arg1().into(), tf.arg2() as _),
        Sysno::mmap => sys_mmap(
            tf.arg0().into(),
            tf.arg1() as _,
            tf.arg2() as _,
            tf.arg3() as _,
            tf.arg4() as _,
            tf.arg5() as _,
        ),
        Sysno::ioctl => sys_ioctl(tf.arg0() as _, tf.arg1() as _, tf.arg2().into()),
        Sysno::writev => sys_writev(tf.arg0() as _, tf.arg1().into(), tf.arg2() as _),
        Sysno::sched_yield => sys_sched_yield(),
        Sysno::nanosleep => stub_bypass(syscall_num as _),
        Sysno::getpid => sys_getpid(),
        Sysno::getppid => sys_getppid(),
        Sysno::exit => sys_exit(tf.arg0() as _),
        Sysno::gettimeofday => sys_get_time_of_day(tf.arg0().into()),
        Sysno::getcwd => sys_getcwd(tf.arg0().into(), tf.arg1() as _),
        Sysno::dup => sys_dup(tf.arg0() as _),
        Sysno::dup3 => sys_dup3(tf.arg0() as _, tf.arg1() as _),
        Sysno::fcntl => sys_fcntl(tf.arg0() as _, tf.arg1() as _, tf.arg2() as _),
        Sysno::clone => sys_clone(
            tf.arg0() as _,
            tf.arg1() as _,
            tf.arg2() as _,
            tf.arg3() as _,
            tf.arg4() as _,
        ),
        Sysno::wait4 => sys_wait4(tf.arg0() as _, tf.arg1().into(), tf.arg2() as _),
        Sysno::pipe2 => sys_pipe2(tf.arg0().into(), tf.arg1() as _),
        Sysno::close => sys_close(tf.arg0() as _),
        Sysno::chdir => sys_chdir(tf.arg0().into()),
        Sysno::mkdirat => sys_mkdirat(tf.arg0() as _, tf.arg1().into(), tf.arg2() as _),
        #[cfg(target_arch = "x86_64")]
        Sysno::mkdir => sys_mkdir(tf.arg0().into(), tf.arg1() as _),
        Sysno::execve => sys_execve(tf.arg0().into(), tf.arg1().into(), tf.arg2().into()),
        Sysno::openat => sys_openat(
            tf.arg0() as _,
            tf.arg1().into(),
            tf.arg2() as _,
            tf.arg3() as _,
        ),
        #[cfg(target_arch = "x86_64")]
        Sysno::open => sys_open(tf.arg0().into(), tf.arg1() as _, tf.arg2() as _),
        Sysno::getdents64 => sys_getdents64(tf.arg0() as _, tf.arg1().into(), tf.arg2() as _),
        Sysno::linkat => sys_linkat(
            tf.arg0() as _,
            tf.arg1().into(),
            tf.arg2() as _,
            tf.arg3().into(),
            tf.arg4() as _,
        ),
        Sysno::unlinkat => sys_unlinkat(tf.arg0() as _, tf.arg1().into(), tf.arg2() as _),
        Sysno::uname => sys_uname(tf.arg0().into()),
        Sysno::fstat => interface::fs::sys_fstat(tf.arg0() as _, tf.arg1().into()),
        Sysno::mount => sys_mount(
            tf.arg0().into(),
            tf.arg1().into(),
            tf.arg2().into(),
            tf.arg3() as _,
            tf.arg4().into(),
        ) as _,
        Sysno::umount2 => sys_umount2(tf.arg0().into(), tf.arg1() as _) as _,
        #[cfg(target_arch = "x86_64")]
        Sysno::newfstatat => interface::fs::sys_fstatat(
            tf.arg0() as _,
            tf.arg1().into(),
            tf.arg2().into(),
            tf.arg3() as _,
        ),
        #[cfg(not(target_arch = "x86_64"))]
        Sysno::fstatat => interface::fs::sys_fstatat(
            tf.arg0() as _,
            tf.arg1().into(),
            tf.arg2().into(),
            tf.arg3() as _,
        ),
        Sysno::statx => interface::fs::sys_statx(
            tf.arg0() as _,
            tf.arg1().into(),
            tf.arg2() as _,
            tf.arg3() as _,
            tf.arg4().into(),
        ),
        Sysno::munmap => sys_munmap(tf.arg0().into(), tf.arg1() as _),
        Sysno::mprotect => sys_mprotect(tf.arg0().into(), tf.arg1() as _, tf.arg2() as _),
        Sysno::times => sys_times(tf.arg0().into()),
        Sysno::brk => sys_brk(tf.arg0() as _),
        #[cfg(target_arch = "x86_64")]
        Sysno::arch_prctl => sys_arch_prctl(tf.arg0() as _, tf.arg1().into()),
        Sysno::set_tid_address => sys_set_tid_address(tf.arg0().into()),
        Sysno::clock_gettime => sys_clock_gettime(tf.arg0() as _, tf.arg1().into()),
        Sysno::exit_group => sys_exit_group(tf.arg0() as _),
        Sysno::getuid => sys_getuid(),
        Sysno::rt_sigprocmask => sys_rt_sigprocmask(
            tf.arg0() as _,
            tf.arg1().into(),
            tf.arg2().into(),
            tf.arg3() as _,
        ),
        Sysno::rt_sigaction => sys_rt_sigaction(
            tf.arg0() as _,
            tf.arg1().into(),
            tf.arg2().into(),
            tf.arg3() as _,
        ),
        #[cfg(target_arch = "x86_64")]
        Sysno::dup2 => sys_dup3(tf.arg0() as _, tf.arg1() as _),
        #[cfg(target_arch = "x86_64")]
        Sysno::fork => sys_fork(),
        Sysno::gettid => sys_gettid(),
        Sysno::lseek => sys_lseek(tf.arg0() as _, tf.arg1() as _, tf.arg2() as _),
        #[cfg(target_arch = "x86_64")]
        Sysno::lstat => interface::fs::sys_lstat(tf.arg0().into(), tf.arg1().into()),
        #[cfg(target_arch = "x86_64")]
        Sysno::pipe => sys_pipe(tf.arg0().into()),
        #[cfg(target_arch = "x86_64")]
        Sysno::poll => sys_poll(tf.arg0().into(), tf.arg1() as _, tf.arg2() as _),
        Sysno::ppoll => sys_ppoll(
            tf.arg0().into(),
            tf.arg1() as _,
            tf.arg2().into(),
            tf.arg3().into(),
        ),
        Sysno::pread64 => sys_pread64(
            tf.arg0() as _,
            tf.arg1().into(),
            tf.arg2() as _,
            tf.arg3() as _,
        ),
        Sysno::prlimit64 => sys_prlimit64(
            tf.arg0() as _,
            tf.arg1() as _,
            tf.arg2().into(),
            tf.arg3().into(),
        ),
        Sysno::readv => sys_readv(tf.arg0() as _, tf.arg1().into(), tf.arg2() as _),
        Sysno::rt_sigtimedwait => sys_rt_sigtimedwait(
            tf.arg0() as _,
            tf.arg1().into(),
            tf.arg2().into(),
            tf.arg3() as _,
        ),
        Sysno::sendfile => sys_sendfile(
            tf.arg0() as _,
            tf.arg1() as _,
            tf.arg2().into(),
            tf.arg3() as _,
        ),
        //Sysno::sysinfo=> sys_sysinfo(tf.arg0().into()),
        Sysno::syslog => sys_syslog(
            tf.arg0() as _,
            tf.arg1().into(),
            tf.arg2() as _,
        ),
        #[cfg(target_arch = "x86_64")]
        Sysno::stat => interface::fs::sys_stat(tf.arg0().into(), tf.arg1().into()),
        Sysno::statfs => sys_statfs(tf.arg0().into(), tf.arg1().into()),
        #[cfg(target_arch = "x86_64")]
        Sysno::unlink => sys_unlink(tf.arg0().into()),
        Sysno::utimensat => sys_utimensat(
            tf.arg0() as _,
            tf.arg1().into(),
            tf.arg2().into(),
            tf.arg3() as _,
        ),
        #[cfg(target_arch = "x86_64")]
        Sysno::access => stub_bypass(syscall_num),
        Sysno::faccessat => stub_bypass(syscall_num),
        Sysno::kill => stub_bypass(syscall_num),
        Sysno::sysinfo => stub_unimplemented(syscall_num),
        _ => stub_kill(syscall_num),
    };
    let ans = result.unwrap_or_else(|err| -err.code() as _);
    time_stat_from_kernel_to_user();
    info!(
        "[syscall] <{:?}> return {}",
        Sysno::from(syscall_num as u32),
        ans
    );
    ans
}

fn stub_unimplemented(syscall_num: usize) -> Result<isize, LinuxError> {
    warn!(
        "Unimplemented syscall: {:?}, ENOSYS",
        Sysno::from(syscall_num as u32)
    );
    Err(LinuxError::ENOSYS)
}

fn stub_bypass(syscall_num: usize) -> Result<isize, LinuxError> {
    warn!(
        "Unimplemented syscall: {:?}, bypassed",
        Sysno::from(syscall_num as u32)
    );
    Ok(0)
}

fn stub_kill(syscall_num: usize) -> Result<isize, LinuxError> {
    warn!(
        "Unimplemented syscall: {:?}, killed",
        Sysno::from(syscall_num as u32)
    );
    axtask::exit(LinuxError::ENOSYS as _)
}
