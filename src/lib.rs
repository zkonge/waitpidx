mod backends;
mod util;

use std::{
    io::{ErrorKind, Result},
    time::Duration,
};

pub use rustix::process::Pid;

use crate::backends::*;
pub use crate::{backends::pidfd, util::process_exists};

pub fn waitpid(pid: u32, timeout: Option<Duration>) -> Result<()> {
    let pid = Pid::from_raw(pid as i32).ok_or(ErrorKind::InvalidInput)?;

    Backend::waitpid(&pidfd::PidFdBackend, pid, timeout)
}

#[cfg(feature = "async")]
pub async fn waitpid_async(pid: u32) -> Result<()> {
    use backends::AsyncBackend;

    let pid = Pid::from_raw(pid as i32).ok_or(ErrorKind::InvalidInput)?;

    AsyncBackend::waitpid(&pidfd::PidFdBackend, pid).await
}

#[cfg(not(target_os = "linux"))]
compile_error!("waitpidx only supports Linux");
