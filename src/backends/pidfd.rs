use std::{
    io::{self, Error, ErrorKind},
    time::Duration,
};

use rustix::{
    event::{poll, PollFd, PollFlags},
    process::{pidfd_open, Pid, PidfdFlags},
};
use tokio::io::unix::AsyncFd;

use super::{AsyncBackend, Backend};

#[derive(Debug)]
pub struct PidFdBackend;

impl Backend for PidFdBackend {
    fn waitpid(&self, pid: Pid, timeout: Option<Duration>) -> io::Result<()> {
        let fd = pidfd_open(pid, PidfdFlags::empty())?;
        let timeout = match timeout {
            Some(dur) => dur.as_millis().try_into().unwrap_or(i32::MAX),
            None => -1, // Infinite.
        };

        let mut fds = [PollFd::new(&fd, PollFlags::IN)];

        match poll(&mut fds, timeout)? {
            0 => Err(Error::from(ErrorKind::TimedOut)),
            _ => Ok(()),
        }
    }
}

impl AsyncBackend for PidFdBackend {
    async fn async_waitpid(&self, pid: Pid) -> io::Result<()> {
        let fd = pidfd_open(pid, PidfdFlags::empty())?;

        let _ = AsyncFd::new(fd)?.readable().await?;

        Ok(())
    }
}
