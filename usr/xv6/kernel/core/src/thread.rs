use alloc::boxed::Box;
use alloc::collections::VecDeque;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use lazy_static::lazy_static;
use spin::Mutex;

use libsyscalls::sync::CondVar;
use libsyscalls::syscalls::sys_create_thread;
use usr_interface::xv6::Thread;
use usr_interface::vfs::VFSPtr;


lazy_static! {
    static ref thread_queue: Mutex<VecDeque<ThreadContext>> = Default::default();
}

pub struct ThreadHandleInternal {
    finished: Mutex<bool>,
    cv: CondVar,
}

impl ThreadHandleInternal {
    fn new() -> Self {
        Self {
            finished: Mutex::new(false),
            cv: CondVar::new(),
        }
    }
}

pub struct ThreadHandle(Arc<ThreadHandleInternal>);

impl ThreadHandle {
    fn new() -> Self {
        Self(Arc::new(ThreadHandleInternal::new()))
    }
}

impl Thread for ThreadHandle {
    fn join(&self) {
        let pred = |finished: &mut bool| *finished;
        self.cv.sleep_until(&self.finished, pred);
    }
}

impl core::ops::Deref for ThreadHandle {
    type Target = ThreadHandleInternal;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}


impl core::clone::Clone for ThreadHandle {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

struct ThreadContext {
    fs: VFSPtr,
    name: String,
    entry: Box<dyn FnOnce() + Send>,
    handle: ThreadHandle,
}

impl ThreadContext {
    fn new(fs: VFSPtr, name: String, entry: Box<dyn FnOnce() + Send>, handle: ThreadHandle) -> Self {
        Self {
            fs,
            name,
            entry,
            handle,
        }
    }
}

extern fn thread_entry() {
    let context = thread_queue.lock().pop_front().unwrap();
    (context.entry)();
    context.fs.sys_thread_exit();
    *context.handle.finished.lock() = true;
    context.handle.cv.wakeup();
    console::println!("Thread {} exits", context.name);
}

pub fn spawn_thread(fs: VFSPtr, name: &str, func: Box<dyn FnOnce() + Send>) -> Box<dyn Thread> {
    let handle = ThreadHandle::new();
    thread_queue.lock().push_back(ThreadContext::new(fs, name.to_string(), func, handle.clone()));
    sys_create_thread(name, thread_entry);
    box handle
}
