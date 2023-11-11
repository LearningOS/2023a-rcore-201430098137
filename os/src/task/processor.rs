//!Implementation of [`Processor`] and Intersection of control flow
//!
//! Here, the continuous operation of user apps in CPU is maintained,
//! the current running state of CPU is recorded,
//! and the replacement and transfer of control flow of different applications are executed.

use super::__switch;
use super::{fetch_task, TaskStatus};
use super::{TaskContext, TaskControlBlock};
use crate::sync::UPSafeCell;
use crate::trap::TrapContext;
use alloc::sync::Arc;
use lazy_static::*;
use crate::mm::{MapPermission, VirtAddr};
use crate::task::task::TaskInfo;
use crate::timer;

/// Processor management structure
pub struct Processor {
    ///The task currently executing on the current processor
    current: Option<Arc<TaskControlBlock>>,

    ///The basic control flow of each core, helping to select and switch process
    idle_task_cx: TaskContext,
}

impl Processor {
    ///Create an empty Processor
    pub fn new() -> Self {
        Self {
            current: None,
            idle_task_cx: TaskContext::zero_init(),
        }
    }

    ///Get mutable reference to `idle_task_cx`
    fn get_idle_task_cx_ptr(&mut self) -> *mut TaskContext {
        &mut self.idle_task_cx as *mut _
    }

    ///Get current task in moving semanteme
    pub fn take_current(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.current.take()
    }

    ///Get current task in cloning semanteme
    pub fn current(&self) -> Option<Arc<TaskControlBlock>> {
        self.current.as_ref().map(Arc::clone)
    }

    /// count numbers of syscall called by task
    pub fn count_numbers_of_syscall(&self, syscall_id: usize) {
        let task = self.current().unwrap();
        task.inner_exclusive_access().syscall_times[syscall_id] += 1;
    }

    /// get current `Running` task info
    pub fn get_current_task(&self) -> TaskInfo {
        let task = self.current().unwrap();
        let inner = task.inner_exclusive_access();
        //print!("start_time:{}, time:{}\n", task.start_time, timer::get_time_us()/1000);
        TaskInfo{
            status: inner.task_status,
            syscall_times: inner.syscall_times.clone(),
            time: timer::get_time_ms() - inner.start_time,
        }
    }

    ///map memory of current task
    pub fn mmap(&self, _start: usize, _len: usize, _port: usize) -> isize {
        let start = VirtAddr::from(_start);
        if start.page_offset() != 0 {
            return -1;
        }

        let mut map_perm = MapPermission::from(_port);
        if map_perm & MapPermission::U != MapPermission::empty() {
            return -1;
        }

        if map_perm & (MapPermission::X | MapPermission::W | MapPermission::R) == MapPermission::empty() {
            return -1;
        }

        map_perm = map_perm | MapPermission::U;

        let end= VirtAddr::from(_start + _len);
        let task = self.current().unwrap();


        //print!("_port:{}", _port as u8);
        if !task.insert_framed_area(start, end, map_perm) {
            return -1;
        }


        0
    }

    ///uumap
    pub fn munmap(&self, _start: usize, _len: usize) -> isize {
        let start = VirtAddr::from(_start);
        if start.page_offset() != 0 {
            return -1;
        }
        let end= VirtAddr::from(_start + _len);
        let task = self.current().unwrap();

        if !task.free_framed_area(start, end) {
            return -1;
        }

        0
    }
}

lazy_static! {
    pub static ref PROCESSOR: UPSafeCell<Processor> = unsafe { UPSafeCell::new(Processor::new()) };
}

///The main part of process execution and scheduling
///Loop `fetch_task` to get the process that needs to run, and switch the process through `__switch`
pub fn run_tasks() {
    loop {
        let mut processor = PROCESSOR.exclusive_access();
        if let Some(task) = fetch_task() {
            let idle_task_cx_ptr = processor.get_idle_task_cx_ptr();
            // access coming task TCB exclusively
            let mut task_inner = task.inner_exclusive_access();
            let next_task_cx_ptr = &task_inner.task_cx as *const TaskContext;
            task_inner.task_status = TaskStatus::Running;
            // release coming task_inner manually
            drop(task_inner);
            // release coming task TCB manually
            processor.current = Some(task);
            // release processor manually
            drop(processor);
            unsafe {
                __switch(idle_task_cx_ptr, next_task_cx_ptr);
            }
        } else {
            warn!("no tasks available in run_tasks");
        }
    }
}

/// Get current task through take, leaving a None in its place
pub fn take_current_task() -> Option<Arc<TaskControlBlock>> {
    PROCESSOR.exclusive_access().take_current()
}

/// Get a copy of the current task
pub fn current_task() -> Option<Arc<TaskControlBlock>> {
    PROCESSOR.exclusive_access().current()
}

/// Get the current user token(addr of page table)
pub fn current_user_token() -> usize {
    let task = current_task().unwrap();
    task.get_user_token()
}

///Get the mutable reference to trap context of current task
pub fn current_trap_cx() -> &'static mut TrapContext {
    current_task()
        .unwrap()
        .inner_exclusive_access()
        .get_trap_cx()
}

///Return to idle control flow for new scheduling
pub fn schedule(switched_task_cx_ptr: *mut TaskContext) {
    let mut processor = PROCESSOR.exclusive_access();
    let idle_task_cx_ptr = processor.get_idle_task_cx_ptr();
    drop(processor);
    unsafe {
        __switch(switched_task_cx_ptr, idle_task_cx_ptr);
    }
}

/// count the number of syscall of the current task
pub fn count_numbers_of_syscall(syscall_id: usize) {
    PROCESSOR.exclusive_access().count_numbers_of_syscall(syscall_id);
}

/// get current `Running` task info
pub fn get_current_task() -> TaskInfo {
    PROCESSOR.readonly_access().get_current_task()
}


///map memory of current task
pub fn mmap(_start: usize, _len: usize, _port: usize) -> isize {
    PROCESSOR.exclusive_access().mmap(_start, _len, _port)
}

///map memory of current task
pub fn munmap(_start: usize, _len: usize) -> isize {
    PROCESSOR.exclusive_access().munmap(_start, _len)
}
