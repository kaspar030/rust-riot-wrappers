use crate::helpers::PointerToCStr;

use riot_sys as raw;

pub use riot_sys::riot_rs_core::c::thread::{
    thread_get_status,
    thread_measure_stack_free,
    thread_status_t as Status,
};

pub use riot_sys::riot_rs_core::thread::{
    self,
    is_valid_pid,
    Thread,
    ThreadId as Pid,
    THREADS_NUMOF,
};

use super::stack_stats::{StackStats, StackStatsError};

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct KernelPID(pub Pid);

impl KernelPID {
    pub fn new(pid: raw::kernel_pid_t) -> Option<Self> {
        if is_valid_pid(pid) {
            Some(KernelPID { 0: pid })
        } else {
            None
        }
    }

    fn is_valid(&self) -> bool {
        is_valid_pid(self.0)
    }

    pub fn get_name(&self) -> Option<&str> {
        if self.is_valid() {
            // unimplemented in RIOT-rs
            None

            // safety: pid checked right above.
            // it might be that this thread has ended already, though...
            // unsafe { Thread::get(self.0) }.name().map_or(None, |ptr| {
            //     let ptr = ptr as *const u8;
            //     // safety: name() either returns a Some(valid pointer) or None
            //     unsafe { ptr.to_lifetimed_cstr()? }.to_str().ok()
            // })
        } else {
            None
        }
    }

    pub fn status(&self) -> Result<Status, ()> {
        thread::get_state(self.0).map_or(Err(()), |status| Ok(status.into()))
    }

    pub fn stack_stats(&self) -> Result<StackStats, StackStatsError> {
        return Err(StackStatsError::InformationUnavailable);
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
    thread::sleep();
}

pub fn get_pid() -> KernelPID {
    KernelPID(thread::current_pid().unwrap())
}
