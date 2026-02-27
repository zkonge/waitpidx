#![forbid(unsafe_code)]
#![warn(missing_docs, rust_2018_idioms, unreachable_pub)]
#![warn(clippy::all, clippy::pedantic, clippy::dbg_macro, clippy::todo)]
#![allow(clippy::module_name_repetitions)]

//! Wait for Linux processes by PID, even if they are not child processes.
//!
//! This crate uses `pidfd_open(2)` and exposes both synchronous and asynchronous
//! waiting APIs.
//!
//! # Platform
//!
//! - Linux only
//! - Kernel 5.3+ (`pidfd_open`)
//!
//! # Features
//!
//! - `async` (default): enables Tokio-based async APIs
//!
//! # Examples
//!
//! Synchronous wait with timeout:
//!
//! ```
//! use std::{
//!     process::Command,
//!     time::Duration,
//! };
//!
//! let mut child = Command::new("sleep").arg("0.1").spawn().unwrap();
//! waitpidx::waitpid(child.id(), Some(Duration::from_secs(3))).unwrap();
//! child.wait().unwrap();
//! ```
//!
//! Asynchronous wait (requires `async` feature):
//!
//! ```
//! # #[cfg(feature = "async")]
//! # {
//! # tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
//! use std::process::Command;
//!
//! let mut child = Command::new("sleep").arg("0.1").spawn().unwrap();
//! waitpidx::waitpid_async(child.id()).await.unwrap();
//! child.wait().unwrap();
//! # });
//! # }
//! ```
mod backends;
mod util;

use std::{
    io::{ErrorKind, Result},
    time::Duration,
};

pub use rustix::process::Pid;

use crate::backends::Backend;
pub use crate::{backends::pidfd, util::process_exists};

/// Wait for a process to exit.
///
/// `pid` can refer to any process visible to the current user namespace and is
/// not restricted to child processes.
///
/// When `timeout` is `Some`, this function returns `TimedOut` if the process
/// does not exit before the deadline. When `None`, it waits indefinitely.
///
/// # Errors
///
/// Returns `InvalidInput` when `pid` is not a valid positive PID.
pub fn waitpid(pid: u32, timeout: Option<Duration>) -> Result<()> {
    let pid = i32::try_from(pid)
        .ok()
        .and_then(Pid::from_raw)
        .ok_or(ErrorKind::InvalidInput)?;

    Backend::waitpid(&pidfd::PidFdBackend, pid, timeout)
}

/// Wait asynchronously for a process to exit.
///
/// This function requires the `async` feature and a running Tokio runtime.
///
/// # Errors
///
/// Returns `InvalidInput` when `pid` is not a valid positive PID.
#[cfg(feature = "async")]
pub async fn waitpid_async(pid: u32) -> Result<()> {
    use backends::AsyncBackend;

    let pid = i32::try_from(pid)
        .ok()
        .and_then(Pid::from_raw)
        .ok_or(ErrorKind::InvalidInput)?;

    AsyncBackend::waitpid(&pidfd::PidFdBackend, pid).await
}

#[cfg(not(target_os = "linux"))]
compile_error!("waitpidx only supports Linux");
