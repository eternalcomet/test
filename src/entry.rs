use alloc::{string::String, sync::Arc};
use axfs_ng::api::{FS_CONTEXT, resolve_path};
use axhal::arch::UspaceContext;
use axsignal::Signo;
use spin::Mutex;
use starry_api::core::fs::fd::FD_TABLE;
use starry_core::mm::{copy_from_kernel, load_user_app, map_trampoline, new_user_aspace_empty};
use starry_core::process::{ProcessData, create_thread_data};
use starry_core::task::{TaskExt, create_user_task};
use undefined_process::process::Process;

pub fn run_user_app(args: &[String], envs: &[String]) -> Option<i32> {
    // create user address space
    // to hold executable file and other data
    let mut uspace = new_user_aspace_empty()
        .and_then(|mut it| {
            copy_from_kernel(&mut it)?;
            // signal trampoline
            map_trampoline(&mut it)?;
            Ok(it)
        })
        .expect("Failed to create user address space");

    // set current directory
    let mut context = FS_CONTEXT.lock();
    let path =
        resolve_path(&context, &args[0], &mut 0, false).expect("Failed to resolve file path");
    context
        .change_dir(path.parent().unwrap())
        .expect("Failed to set current dir");
    drop(context);

    // load executable file
    let (entry_vaddr, ustack_top) = load_user_app(&mut uspace, args, envs)
        .unwrap_or_else(|e| panic!("Failed to load user app: {}", e));

    // create user context
    let uctx = UspaceContext::new(entry_vaddr.into(), ustack_top, 2333);
    // create user task for scheduler
    let mut user_task = create_user_task(args.join(" "), uctx);
    user_task
        .ctx_mut()
        .set_page_table_root(uspace.page_table_root());

    // create thread
    let process = Process::spawn_process();
    let thread = process.get_main_thread().unwrap();

    // init task extended data
    let process_data = ProcessData::new(
        args.to_vec(),
        Arc::new(Mutex::new(uspace)),
        Arc::default(),
        Some(Signo::SIGCHLD),
    );
    let thread_data = create_thread_data(Arc::new(process_data), thread.get_tid());

    FD_TABLE
        .deref_from(&thread_data.namespace)
        .init_new(FD_TABLE.copy_inner());
    FS_CONTEXT
        .deref_from(&thread_data.namespace)
        .init_new(FS_CONTEXT.copy_inner());

    user_task.init_task_ext(TaskExt::new(thread, thread_data));

    // spawn and wait the task
    let user_task = axtask::spawn_task(user_task);
    user_task.join()
}
