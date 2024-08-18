use std::{
    io::{Error, ErrorKind, Result},
    os::fd::OwnedFd,
    time::Duration,
};

use rustix::{
    event::{poll, PollFd, PollFlags},
    process::{pidfd_open, Pid, PidfdFlags},
};

struct PidFdInner(OwnedFd);

impl PidFdInner {
    fn new(pid: Pid) -> Result<Self> {
        pidfd_open(pid, PidfdFlags::empty())
            .map(Self)
            .map_err(Into::into)
    }

    fn waitpid(&self, timeout: Option<Duration>) -> Result<()> {
        let timeout = match timeout {
            Some(dur) => dur.as_millis().try_into().unwrap_or(i32::MAX),
            None => -1, // infinity
        };

        let mut fds = [PollFd::new(&self.0, PollFlags::IN)];
        match poll(&mut fds, timeout)? {
            0 => Err(Error::from(ErrorKind::TimedOut)),
            _ => Ok(()),
        }
    }

    #[inline]
    fn is_exited(&self) -> Result<bool> {
        match self.waitpid(Some(Duration::ZERO)) {
            Ok(_) => Ok(true),
            Err(e) if e.kind() == ErrorKind::TimedOut => Ok(false),
            Err(e) => Err(e),
        }
    }
}

pub struct PidFd(PidFdInner);

impl PidFd {
    pub fn new(pid: Pid) -> Result<Self> {
        PidFdInner::new(pid).map(Self)
    }

    #[inline]
    pub fn wait(&self, timeout: Option<Duration>) -> Result<()> {
        self.0.waitpid(timeout)
    }

    #[inline]
    pub fn is_exited(&self) -> Result<bool> {
        self.0.is_exited()
    }
}
