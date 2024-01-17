use std::{
    io::{self, Error, ErrorKind},
    time::Duration,
};

use rustix::{
    event::{poll, PollFd, PollFlags},
    process::{pidfd_open, Pid, PidfdFlags},
};

use super::Backend;

#[derive(Debug)]
pub struct PidFdBackend;

impl Backend for PidFdBackend {
    fn waitpid(&self, pid: Pid, timeout: Option<Duration>) -> io::Result<()> {
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
    async fn waitpid_async(&self, pid: Pid) -> io::Result<()> {
        let fd = pidfd_open(pid, PidfdFlags::empty())?;

        let _ = tokio::io::unix::AsyncFd::new(fd)?.readable().await?;

        Ok(())
    }
}
