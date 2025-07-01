#[cfg(feature = "async")]
mod async_fd;
mod sync_fd;

use std::{
    io::{Error, ErrorKind, Result},
    time::Duration,
};

use rustix::{
    event::{poll, PollFd, PollFlags, Timespec},
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

        let timeout: Option<Timespec> = timeout
            .map(TryInto::try_into)
            .transpose()
            .map_err(|_| ErrorKind::InvalidInput)?;

        let mut fds = [PollFd::new(&fd, PollFlags::IN)];

        match poll(&mut fds, timeout.as_ref())? {
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
