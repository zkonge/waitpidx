#[cfg(feature = "async")]
mod async_fd;
mod sync_fd;

use std::{io::Result, time::Duration};

use rustix::process::Pid;

#[cfg(feature = "async")]
pub use self::async_fd::{AsyncPidFd, AsyncPidFdExited, AsyncPidFdWait};
pub use self::sync_fd::PidFd;
use super::Backend;

#[derive(Debug)]
pub(crate) struct PidFdBackend;

impl Backend for PidFdBackend {
    fn waitpid(&self, pid: Pid, timeout: Option<Duration>) -> Result<()> {
        PidFd::new(pid)?.wait(timeout)
    }
}

#[cfg(feature = "async")]
impl super::AsyncBackend for PidFdBackend {
    async fn waitpid(&self, pid: Pid) -> Result<()> {
        AsyncPidFd::new(pid)?.await
    }
}
