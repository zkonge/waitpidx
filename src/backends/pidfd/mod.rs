#[cfg(feature = "async")]
mod async_fd;
mod sync_fd;

use std::{
    io::{Error, ErrorKind, Result},
    time::Duration,
};

use rustix::{
    event::{poll, PollFd, PollFlags},
    process::{pidfd_open, Pid, PidfdFlags},
};

#[cfg(feature = "async")]
pub use self::async_fd::{AsyncPidFd, AsyncPidFdExited, AsyncPidFdWait};
pub use self::sync_fd::PidFd;
use super::Backend;

#[derive(Debug)]
pub(crate) struct PidFdBackend;

impl Backend for PidFdBackend {
    fn waitpid(&self, pid: Pid, timeout: Option<Duration>) -> Result<()> {
        let fd = pidfd_open(pid, PidfdFlags::empty())?;
        let timeout = match timeout {
            Some(dur) => dur.as_millis().try_into().unwrap_or(i32::MAX),
            None => -1, // infinity
        };

        let mut fds = [PollFd::new(&fd, PollFlags::IN)];

        match poll(&mut fds, timeout)? {
            0 => Err(Error::from(ErrorKind::TimedOut)),
            _ => Ok(()),
        }
    }
}

#[cfg(feature = "async")]
impl super::AsyncBackend for PidFdBackend {
    async fn waitpid(&self, pid: Pid) -> Result<()> {
        AsyncPidFd::new(pid)?.await
    }
}
