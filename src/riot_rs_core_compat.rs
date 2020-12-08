pub mod thread {
    pub use riot_rs_core::thread::{Pid, Thread};
    use riot_sys as raw;
    #[derive(Debug, PartialEq, Copy, Clone)]
    pub struct KernelPID(Pid);

    impl KernelPID {
        pub fn new(pid: raw::kernel_pid_t) -> Option<Self> {
            if Thread::pid_is_valid(pid) {
                Some(KernelPID { 0: pid })
            } else {
                None
            }
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
}
