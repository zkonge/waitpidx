/// Async netlink waiter
use std::{
    collections::HashMap,
    io::Result,
    iter,
    sync::{Arc, Mutex},
};

use rustix::{io::Errno, process::Pid};
use tokio::sync::watch;

use super::{binding::NL_CONNECTOR_MAX_MSG_SIZE, connection::NetlinkConnection};
use crate::{backends::AsyncBackend, utils};

type AsyncExitNotifier = watch::Sender<()>;
type AsyncExitReceiver = watch::Receiver<()>;

#[derive(Debug)]
struct AsyncNetlinkBackendInner {
    netlink: NetlinkConnection,
    interest: Mutex<HashMap<Pid, AsyncExitNotifier>>,
}

impl AsyncNetlinkBackendInner {
    fn new() -> Result<Arc<Self>> {
        let netlink = NetlinkConnection::new()?;
        netlink.interest(Some(&[]))?;
        netlink.start()?;

        Ok(Arc::new(Self {
            netlink,
            interest: Default::default(),
        }))
    }

    async fn interest(&self, pid: Pid) -> Result<AsyncExitReceiver> {
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

        // cleanup invalid entries in order to avoid memory leaks
        let entry = interest_group.entry(pid).or_insert(watch::channel(()).0);

        Ok(entry.subscribe())
    }

    async fn handle_events(&self) -> Result<()> {
        let mut buf = [0u8; NL_CONNECTOR_MAX_MSG_SIZE];

        loop {
            let pid = self.netlink.read_event_async(&mut buf).await?;

            let mut interest_group = self.interest.lock().unwrap_or_else(|x| x.into_inner());

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
pub struct AsyncNetlinkBackend {
    inner: Arc<AsyncNetlinkBackendInner>,
    aborter: tokio::task::AbortHandle,
}

impl AsyncNetlinkBackend {
    pub fn new() -> Result<Self> {
        let inner = AsyncNetlinkBackendInner::new()?;

        let h = tokio::spawn({
            let inner = inner.clone();
            async move {
                match inner.handle_events().await {
                    Ok(()) => { /* connection closed */ }
                    Err(e) => panic!("{e:?}"),
                }
            }
        });
        let aborter = h.abort_handle();

        Ok(Self { inner, aborter })
    }

    pub async fn interest(&self, pid: Pid) -> Result<AsyncExitReceiver> {
        self.inner.interest(pid).await
    }
}

impl Drop for AsyncNetlinkBackend {
    fn drop(&mut self) {
        _ = self.inner.netlink.stop();
        self.aborter.abort();
    }
}

impl AsyncBackend for AsyncNetlinkBackend {
    async fn waitpid(&self, pid: Pid) -> Result<()> {
        _ = self.interest(pid).await?.changed().await;

        Ok(())
    }
}
