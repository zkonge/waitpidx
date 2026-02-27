use std::{
    io::{ErrorKind, Result},
    os::fd::OwnedFd,
    time::Duration,
};

use rustix::{
    event::{poll, PollFd, PollFlags, Timespec},
    process::{pidfd_open, Pid, PidfdFlags},
};

#[derive(Debug)]
struct PidFdInner(OwnedFd);

impl PidFdInner {
    fn new(pid: Pid) -> Result<Self> {
        pidfd_open(pid, PidfdFlags::empty())
            .map(Self)
            .map_err(Into::into)
    }

    fn waitpid(&self, timeout: Option<Duration>) -> Result<()> {
        let timeout: Option<Timespec> = timeout
            .map(TryInto::try_into)
            .transpose()
            .map_err(|_| ErrorKind::InvalidInput)?;

        let mut fds = [PollFd::new(&self.0, PollFlags::IN)];

        match poll(&mut fds, timeout.as_ref())? {
            0 => Err(ErrorKind::TimedOut.into()),
            _ => Ok(()),
        }
    }

    #[inline]
    fn is_exited(&self) -> Result<bool> {
        match self.waitpid(Some(Duration::ZERO)) {
            Ok(()) => Ok(true),
            Err(e) if e.kind() == ErrorKind::TimedOut => Ok(false),
            Err(e) => Err(e),
        }
    }
}

/// A synchronous pidfd handle.
#[derive(Debug)]
pub struct PidFd(PidFdInner);

impl PidFd {
    /// Open a pidfd for `pid`.
    ///
    /// # Errors
    ///
    /// Returns an OS error when opening pidfd fails.
    pub fn new(pid: Pid) -> Result<Self> {
        PidFdInner::new(pid).map(Self)
    }

    /// Wait for process exit.
    ///
    /// If `timeout` is `Some`, returns `TimedOut` when deadline is reached.
    ///
    /// # Errors
    ///
    /// Returns `TimedOut` if timeout expires, and forwards OS errors from
    /// `poll(2)`.
    #[inline]
    pub fn wait(&self, timeout: Option<Duration>) -> Result<()> {
        self.0.waitpid(timeout)
    }

    /// Check whether the process has already exited.
    ///
    /// This is implemented as a non-blocking wait.
    ///
    /// # Errors
    ///
    /// Forwards errors produced while probing process state.
    #[inline]
    pub fn is_exited(&self) -> Result<bool> {
        self.0.is_exited()
    }
}
