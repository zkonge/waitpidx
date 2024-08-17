use std::{
    future::Future,
    io,
    os::fd::OwnedFd,
    pin::Pin,
    sync::atomic::{AtomicBool, Ordering},
    task::{Context, Poll},
};

use rustix::process::{pidfd_open, Pid, PidfdFlags};
use tokio::io::{unix::AsyncFd, Interest};

struct MaybeExitedPidFd {
    fd: AsyncFd<OwnedFd>,
    exited: AtomicBool,
}

impl MaybeExitedPidFd {
    fn new(pid: Pid) -> io::Result<Self> {
        Ok(Self {
            fd: AsyncFd::with_interest(pidfd_open(pid, PidfdFlags::empty())?, Interest::READABLE)?,
            exited: false.into(),
        })
    }

    fn poll_exit(&self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        if self.exited.load(Ordering::Acquire) {
            return Poll::Ready(Ok(()));
        }

        let r = self.fd.poll_read_ready(cx).map_ok(|_| ());

        if let Poll::Ready(Ok(_)) = r {
            self.exited.store(true, Ordering::Release);
        }

        r
    }
}

pub struct AsyncPidFdWait<'a> {
    pidfd: &'a MaybeExitedPidFd,
}

impl Future for AsyncPidFdWait<'_> {
    type Output = io::Result<()>;

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.pidfd.poll_exit(cx)
    }
}

pub struct AsyncPidFdExited<'a> {
    pidfd: &'a MaybeExitedPidFd,
}

impl Future for AsyncPidFdExited<'_> {
    type Output = io::Result<bool>;

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.pidfd.poll_exit(cx) {
            Poll::Ready(x) => Poll::Ready(x.map(|_| true)),
            Poll::Pending => Poll::Ready(Ok(false)),
        }
    }
}

pub struct AsyncPidFd(MaybeExitedPidFd);

impl AsyncPidFd {
    #[inline]
    pub fn new(pid: Pid) -> io::Result<Self> {
        MaybeExitedPidFd::new(pid).map(Self)
    }

    #[inline]
    pub fn wait(&self) -> AsyncPidFdWait {
        AsyncPidFdWait { pidfd: &self.0 }
    }

    #[inline]
    pub fn is_exited(&self) -> AsyncPidFdExited {
        AsyncPidFdExited { pidfd: &self.0 }
    }
}

impl Future for AsyncPidFd {
    type Output = io::Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.0.poll_exit(cx)
    }
}
