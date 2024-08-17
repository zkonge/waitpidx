use std::{
    io::{self, Error, ErrorKind},
    os::fd::OwnedFd,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

use rustix::{
    event::{poll, PollFd, PollFlags},
    process::{pidfd_open, Pid, PidfdFlags},
};

struct MaybeExitedPidFd {
    fd: OwnedFd,
    exited: AtomicBool,
}

impl MaybeExitedPidFd {
    fn new(pid: Pid) -> io::Result<Self> {
        Ok(Self {
            fd: pidfd_open(pid, PidfdFlags::empty())?,
            exited: false.into(),
        })
    }

    fn waitpid(&self, timeout: Option<Duration>) -> io::Result<()> {
        if self.exited.load(Ordering::Acquire) {
            return Ok(());
        };

        let timeout = match timeout {
            Some(dur) => dur.as_millis().try_into().unwrap_or(i32::MAX),
            None => -1, // infinity
        };

        let mut fds = [PollFd::new(&self.fd, PollFlags::IN)];
        match poll(&mut fds, timeout)? {
            0 => Err(Error::from(ErrorKind::TimedOut)),
            _ => {
                self.exited.store(true, Ordering::Release);
                Ok(())
            }
        }
    }

    #[inline]
    fn is_exited(&self) -> io::Result<bool> {
        match self.waitpid(Some(Duration::ZERO)) {
            Ok(_) => Ok(true),
            Err(e) if e.kind() == ErrorKind::TimedOut => Ok(false),
            Err(e) => Err(e),
        }
    }
}

pub struct PidFd(MaybeExitedPidFd);

impl PidFd {
    pub fn new(pid: Pid) -> io::Result<Self> {
        MaybeExitedPidFd::new(pid).map(Self)
    }

    #[inline]
    pub fn wait(&self, timeout: Option<Duration>) -> io::Result<()> {
        self.0.waitpid(timeout)
    }

    #[inline]
    pub fn is_exited(&self) -> io::Result<bool> {
        self.0.is_exited()
    }
}
