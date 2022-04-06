//! Create, inspect or modify RIOT processes ("threads")
//!
//! ## Tokens
//!
//! Some thread creation mechanisms (currently only [riot_main_with_tokens] and not those in here)
//! are "with tokens". With these, the zero-sized type [StartToken] is used to pass along the
//! information that the execution is currently happening in a thread, and more importantly that
//! some operations doable only once per thread (eg. setting up a message queue) have not yet
//! happed.
//!
//! When threads are created that way, they need to return an [EndToken] which ensures that
//! no operations that preclude the termination of a thread have happened.
//!
//! This has multiple implementations:
//! - one wrapping the RIOT (C) core/ API
//! - one wrapping the RIOT-rs (Rust) riot-rs-core API
//!
//! The right implementation is selected with help from the build system, similar to how std's
//! platform dependent backends are selected.

#[cfg(not(marker_riot_rs))]
mod riot_c;
#[cfg(not(marker_riot_rs))]
pub use riot_c::*;

#[cfg(marker_riot_rs)]
mod riot_rs;
#[cfg(marker_riot_rs)]
pub use riot_rs::*;

mod tokenparts;
#[cfg(doc)]
pub use tokenparts::TokenParts;
pub use tokenparts::{EndToken, InIsr, InThread, StartToken, TerminationToken, ValueInThread};

mod stack_stats;
pub use stack_stats::{StackStats, StackStatsError};

/// Error returned by PID methods when no thread with that PID exists
#[derive(Debug)]
pub struct NoSuchThread;

pub(crate) mod pid_converted {
    //! Converting the raw constants into consistently typed ones
    use riot_sys as raw;

    pub const KERNEL_PID_UNDEF: raw::kernel_pid_t = raw::KERNEL_PID_UNDEF as _;
    pub const KERNEL_PID_FIRST: raw::kernel_pid_t = raw::KERNEL_PID_FIRST as _;
    pub const KERNEL_PID_LAST: raw::kernel_pid_t = raw::KERNEL_PID_LAST as _;
    pub const KERNEL_PID_ISR: raw::kernel_pid_t = raw::KERNEL_PID_ISR as _;
}
