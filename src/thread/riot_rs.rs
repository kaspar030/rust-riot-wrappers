use crate::helpers::PointerToCStr;

use riot_sys as raw;

pub use riot_sys::riot_rs_core::thread::c::{
    thread_get_status,
    thread_measure_stack_free,
    thread_status_t as Status,
};
pub use riot_sys::riot_rs_core::thread::{Pid, Thread, THREADS_NUMOF};

use super::stack_stats::{StackStats, StackStatsError};

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct KernelPID(pub Pid);

impl KernelPID {
    pub fn new(pid: raw::kernel_pid_t) -> Option<Self> {
        if Thread::pid_is_valid(pid) {
            Some(KernelPID { 0: pid })
        } else {
            None
        }
    }
    pub fn get_name(&self) -> Option<&str> {
        if Thread::pid_is_valid(self.0) {
            // safety: pid checked right above.
            // it might be that this thread has ended already, though...
            unsafe { Thread::get(self.0) }.name().map_or(None, |ptr| {
                Some(
                    // safety: name() either returns a Some(valid pointer) or None
                    unsafe { ptr.to_lifetimed_cstr()? }.to_str().ok(),
                )
            })
        } else {
            None
        }
    }
    pub fn status(&self) -> Result<Status, ()> {
        if Thread::pid_is_valid(self.0) {
            // safety: pid checked right above.
            Ok(unsafe { thread_get_status(Thread::get(self.0)) })
        } else {
            Err(())
        }
    }

    fn thread(&self) -> Option<&Thread> {
        if Thread::pid_is_valid(self.0) {
            Some(unsafe { Thread::get(self.0) })
        } else {
            None
        }
    }

    pub fn stack_stats(&self) -> Result<StackStats, StackStatsError> {
        let thread = self.thread().ok_or(StackStatsError::NoSuchThread)?;
        return Ok(StackStats {
            // This cast is relevant because different platforms (eg. native and arm) disagree on
            // whether that's an i8 or u8 pointer. Could have made it c_char, but a) don't want to
            // alter the signatures and b) it's easier to use on the Rust side with a clear type.
            start: thread.stack_bottom() as _,
            size: thread.stack_size() as _,
            free: unsafe { thread_measure_stack_free(thread.stack_bottom() as _) },
        });
        // TODO: handle case where riot-rs-core doesn't have the info.
        //return Err(StackStatsError::InformationUnavailable);
    }
}

impl KernelPID {
    pub fn all_pids() -> impl Iterator<Item = KernelPID> {
        // Not constructing the KernelPID manually but going through new serves as a convenient
        // validation of the construction (all_pids will panic if the rules of pid_is_valid change,
        // and then this function *should* be reevaluated). As pid_is_valid is static inline, the
        // compiler should be able to see through the calls down to there that the bounds checked
        // for there are the very bounds used in the construction here.
        (0..(THREADS_NUMOF))
            .map(|i| KernelPID::new(i as Pid).expect("Should be valid by construction"))
    }
}

impl Into<raw::kernel_pid_t> for &KernelPID {
    fn into(self) -> raw::kernel_pid_t {
        self.0
    }
}

impl Into<raw::kernel_pid_t> for KernelPID {
    fn into(self) -> raw::kernel_pid_t {
        self.0
    }
}

impl Into<i16> for KernelPID {
    fn into(self) -> i16 {
        self.0 as i16
    }
}

pub fn sleep() {
    Thread::sleep();
}

pub fn get_pid() -> KernelPID {
    KernelPID(Thread::current_pid())
}
