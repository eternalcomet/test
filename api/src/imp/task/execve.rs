use alloc::string::String;
use alloc::vec::Vec;
use axerrno::{AxError, LinuxResult};
use axhal::arch::{TrapFrame};
use axtask::current;
use core::default::Default;
use starry_core::mm;
use starry_core::mm::map_trampoline;
use starry_core::task::{current_process, current_process_data};

pub fn sys_execve_impl(
    tf: &mut TrapFrame,
    path: String,
    args: Vec<String>,
    envs: Vec<String>,
) -> LinuxResult<isize> {
    use axalloc::global_allocator;
    let allocator = global_allocator();
    error!(
        "memory statistic: used: [{} KiB, {} pages], available: [{} KiB, {} pages]",
        allocator.used_bytes() / 1024,
        allocator.used_pages(),
        allocator.available_bytes() / 1024,
        allocator.available_pages()
    );
    if current_process().get_threads().len() > 1 {
        // TODO: kill other threads except leader thread
        // because we need to unmap the address space
        error!("[sys_execve] execve is not supported in multi-threaded process");
    }

    debug!("[execve] args = {:?}, envs = {:?}", &args, &envs);

    let process_data = current_process_data();

    // clear address space
    let addr_space = &process_data.addr_space;
    let mut addr_space = addr_space.lock();
    addr_space.unmap_user_areas()?;
    // for signals
    map_trampoline(&mut addr_space)?;
    axhal::arch::flush_tlb(None);

    // load executable binary
    let (entry_point, user_stack_base) =
        mm::load_user_app(&mut addr_space, &args, &envs).map_err(|_| {
            error!("Failed to load app {}", path);
            AxError::NotFound
        })?;

    // set name and path
    current().set_name(&path);
    *process_data.command_line.lock() = args;

    // reset some process attributes
    // TODO: reset signal dispositions, mmap, shm, etc.
    *process_data.signal.actions.lock() = Default::default();
    process_data.shared_memory.lock().clear();

    // new user context
    tf.set_ip(entry_point.as_usize());
    tf.set_sp(user_stack_base.as_usize());
    
    Ok(0)
}
