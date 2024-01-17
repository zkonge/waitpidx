/// Sync netlink pid waiter
use std::{
    collections::HashMap,
    io::{Error, ErrorKind, Result},
    iter,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use rustix::{
    fd::{AsFd, AsRawFd},
    process::Pid,
};

use super::{binding::NL_CONNECTOR_MAX_MSG_SIZE, connection::NetlinkConnection};
use crate::{backends::Backend, utils};

type ExitNotifier = crossbeam_channel::Sender<()>;
type ExitReceiver = crossbeam_channel::Receiver<()>;

#[derive(Debug)]
struct NetlinkBackendInner {
    netlink: NetlinkConnection,
    interest: Mutex<HashMap<Pid, Vec<ExitNotifier>>>,
}

impl NetlinkBackendInner {
    fn new() -> Result<Arc<Self>> {
        let netlink = NetlinkConnection::new()?;
        netlink.interest(Some(&[]))?;
        netlink.start()?;

        let ret = Arc::new(Self {
            netlink,
            interest: Default::default(),
        });

        thread::spawn({
            let ret = ret.clone();
            move || {
                match ret.handle_events() {
                    Ok(_) => { /* connection closed */ }
                    Err(e) if e.raw_os_error() == Some(libc::EBADF) => { /* connection closed */ }
                    Err(e) => panic!("{e:?}"),
                }
            }
        });

        Ok(ret)
    }

    fn interest(&self, pid: Pid) -> Result<ExitReceiver> {
        let mut interest_group = self.interest.lock().unwrap();

        let keys = interest_group
            .keys()
            .copied()
            .chain(iter::once(pid))
            .collect::<Vec<_>>();

        self.netlink.interest(Some(keys.as_slice()))?;

        let (tx, rx) = crossbeam_channel::bounded(0);
        interest_group.entry(pid).or_default().push(tx);
        Ok(rx)
    }

    fn handle_events(&self) -> Result<()> {
        let mut buf = [0u8; NL_CONNECTOR_MAX_MSG_SIZE];

        loop {
            let pid = self.netlink.read_event(&mut buf, None)?;

            let mut interest_group = self.interest.lock().unwrap();
            if let Some(notifiers) = interest_group.remove(&pid) {
                for notifier in notifiers {
                    let _ = notifier.send(()); // don't care if the receiver is dropped
                }
            }

            let keys = interest_group.keys().copied().collect::<Vec<_>>();
            match self.netlink.interest(Some(&keys)) {
                Ok(_) => (),
                Err(_) => return Ok(()), // OwnedFd is dropped
            }
        }
    }
}

#[derive(Debug)]
pub struct NetlinkBackend(Arc<NetlinkBackendInner>);

impl NetlinkBackend {
    pub fn new() -> Result<Self> {
        Ok(Self(NetlinkBackendInner::new()?))
    }

    pub fn interest(&self, pid: Pid) -> Result<ExitReceiver> {
        self.0.interest(pid)
    }
}

impl Drop for NetlinkBackend {
    fn drop(&mut self) {
        let _ = self.0.netlink.stop();
        unsafe { rustix::io::close(self.0.netlink.as_fd().as_raw_fd()) }
    }
}

impl Backend for NetlinkBackend {
    fn waitpid(&self, pid: Pid, timeout: Option<Duration>) -> Result<()> {
        if !utils::process_exists(pid) {
            return Err(Error::from_raw_os_error(libc::ESRCH));
        }

        let rx = self.interest(pid)?;

        match timeout {
            Some(timeout) => match rx.recv_timeout(timeout) {
                Ok(_) => Ok(()),
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                    Err(ErrorKind::TimedOut.into())
                }
                Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                    Err(ErrorKind::BrokenPipe.into())
                }
            },
            None => rx.recv().map_err(|_| ErrorKind::BrokenPipe.into()),
        }
    }
}
