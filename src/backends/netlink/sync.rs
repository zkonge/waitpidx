/// Sync netlink pid waiter
use std::{
    collections::HashMap,
    io::{ErrorKind, Result},
    iter,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use crossbeam_channel::RecvTimeoutError;
use rustix::{
    fd::{AsFd, BorrowedFd, OwnedFd},
    io::{write, Errno},
    pipe::{pipe_with, PipeFlags},
    process::Pid,
};

use super::{binding::NL_CONNECTOR_MAX_MSG_SIZE, connection::NetlinkConnection};
use crate::{backends::Backend, utils};

type ExitNotifier = crossbeam_channel::Sender<()>;
type ExitReceiver = crossbeam_channel::Receiver<()>;

#[derive(Debug)]
struct NetlinkBackendInner {
    netlink: NetlinkConnection,
    interest: Mutex<HashMap<Pid, (ExitNotifier, ExitReceiver)>>,
}

impl NetlinkBackendInner {
    fn new() -> Result<Arc<Self>> {
        let netlink = NetlinkConnection::new()?;
        netlink.interest(Some(&[]))?;
        netlink.start()?;

        Ok(Arc::new(Self {
            netlink,
            interest: Default::default(),
        }))
    }

    fn interest(&self, pid: Pid) -> Result<ExitReceiver> {
        let mut interest_group = self.interest.lock().unwrap();

        // the process existence checking must in the lock scope or notify event would be dropped
        if !utils::process_exists(pid) {
            return Err(Errno::SRCH.into());
        }

        let keys = interest_group
            .keys()
            .copied()
            .chain(iter::once(pid))
            .collect::<Vec<_>>();

        self.netlink.interest(Some(keys.as_slice()))?;

        let (_, rx) = interest_group
            .entry(pid)
            .or_insert(crossbeam_channel::bounded(0));

        Ok(rx.clone())
    }

    fn handle_events(&self, timeout: Option<Duration>, aborter: BorrowedFd<'_>) -> Result<()> {
        let mut buf = [0u8; NL_CONNECTOR_MAX_MSG_SIZE];

        loop {
            let pid = self.netlink.read_event(&mut buf, timeout, aborter)?;

            let mut interest_group = self.interest.lock().unwrap();

            // notify receivers by drop senders
            interest_group.remove(&pid);

            let keys = interest_group.keys().copied().collect::<Vec<_>>();
            match self.netlink.interest(Some(&keys)) {
                Ok(()) => (),
                Err(_) => return Ok(()), // OwnedFd is dropped
            }
        }
    }
}

#[derive(Debug)]
pub struct NetlinkBackend {
    inner: Arc<NetlinkBackendInner>,
    aborter: OwnedFd,
}

impl NetlinkBackend {
    pub fn new() -> Result<Self> {
        let inner = NetlinkBackendInner::new()?;
        let (rx, tx) = pipe_with(PipeFlags::DIRECT | PipeFlags::CLOEXEC)?;

        thread::spawn({
            let inner = inner.clone();
            move || {
                match inner.handle_events(None, rx.as_fd()) {
                    Ok(()) => { /* connection closed */ }
                    Err(e) if e.kind() == ErrorKind::ConnectionAborted => {
                        /* connection closed */
                    }
                    Err(e) => panic!("{e:?}"),
                }
            }
        });

        Ok(Self { inner, aborter: tx })
    }

    pub fn interest(&self, pid: Pid) -> Result<ExitReceiver> {
        self.inner.interest(pid)
    }
}

impl Drop for NetlinkBackend {
    fn drop(&mut self) {
        _ = self.inner.netlink.stop();
        _ = write(self.aborter.as_fd(), &[0u8]);
    }
}

impl Backend for NetlinkBackend {
    fn waitpid(&self, pid: Pid, timeout: Option<Duration>) -> Result<()> {
        let rx = self.interest(pid)?;

        match timeout {
            Some(timeout) => match rx.recv_timeout(timeout) {
                Ok(()) | Err(RecvTimeoutError::Disconnected) => Ok(()),
                Err(RecvTimeoutError::Timeout) => Err(ErrorKind::TimedOut.into()),
            },
            None => {
                _ = rx.recv();
                Ok(())
            }
        }
    }
}
