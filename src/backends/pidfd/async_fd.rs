use std::{
    future::Future,
    io::Result,
    os::fd::OwnedFd,
    pin::Pin,
    task::{Context, Poll},
};

use rustix::process::{pidfd_open, Pid, PidfdFlags};
use tokio::io::{unix::AsyncFd, Interest};

#[derive(Debug)]
struct PidFdInner(AsyncFd<OwnedFd>);

impl PidFdInner {
    fn new(pid: Pid) -> Result<Self> {
        AsyncFd::with_interest(pidfd_open(pid, PidfdFlags::empty())?, Interest::READABLE).map(Self)
    }

    fn poll_exit(&self, cx: &mut Context<'_>) -> Poll<Result<()>> {
        self.0.poll_read_ready(cx).map_ok(|_| ())
    }
}

/// Future returned by [`AsyncPidFd::wait`].
pub struct AsyncPidFdWait<'a> {
    pidfd: &'a PidFdInner,
}

impl Future for AsyncPidFdWait<'_> {
    type Output = Result<()>;

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.pidfd.poll_exit(cx)
    }
}

/// Future returned by [`AsyncPidFd::is_exited`].
pub struct AsyncPidFdExited<'a> {
    pidfd: &'a PidFdInner,
}

impl Future for AsyncPidFdExited<'_> {
    type Output = Result<bool>;

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.pidfd.poll_exit(cx) {
            Poll::Ready(x) => Poll::Ready(x.map(|()| true)),
            Poll::Pending => Poll::Ready(Ok(false)),
        }
    }
}

#[derive(Debug)]
/// An asynchronous pidfd handle.
///
/// This type can be awaited directly, or used via [`AsyncPidFd::wait`] and
/// [`AsyncPidFd::is_exited`].
pub struct AsyncPidFd(PidFdInner);

impl AsyncPidFd {
    /// Open an async pidfd for `pid`.
    ///
    /// # Errors
    ///
    /// Returns an OS error when opening pidfd fails.
    #[inline]
    pub fn new(pid: Pid) -> Result<Self> {
        PidFdInner::new(pid).map(Self)
    }

    /// Return a future that resolves when the process exits.
    #[inline]
    #[must_use]
    pub fn wait(&self) -> AsyncPidFdWait<'_> {
        AsyncPidFdWait { pidfd: &self.0 }
    }

    /// Return a future that performs a non-blocking exit probe.
    ///
    /// The returned future always resolves immediately with `Ok(true)` when the
    /// process is exited, or `Ok(false)` otherwise.
    #[inline]
    #[must_use]
    pub fn is_exited(&self) -> AsyncPidFdExited<'_> {
        AsyncPidFdExited { pidfd: &self.0 }
    }
}

impl Future for AsyncPidFd {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.0.poll_exit(cx)
    }
}
