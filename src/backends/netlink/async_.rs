/// Async netlink waiter
use std::{
    collections::HashMap,
    io::{Error, ErrorKind, Result},
    iter,
    sync::Arc,
};

use rustix::process::Pid;

use super::{binding::NL_CONNECTOR_MAX_MSG_SIZE, connection::NetlinkConnection};
use crate::{backends::AsyncBackend, utils};

type AsyncExitNotifier = tokio::sync::oneshot::Sender<()>;
type AsyncExitReceiver = tokio::sync::oneshot::Receiver<()>;

#[derive(Debug)]
struct AsyncNetlinkBackendInner {
    netlink: NetlinkConnection,
    interest: tokio::sync::Mutex<HashMap<Pid, Vec<AsyncExitNotifier>>>,
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
        let mut interest_group = self.interest.lock().await;

        let keys = interest_group
            .keys()
            .copied()
            .chain(iter::once(pid))
            .collect::<Vec<_>>();

        self.netlink.interest(Some(keys.as_slice()))?;

        let (tx, rx) = tokio::sync::oneshot::channel();
        interest_group.entry(pid).or_default().push(tx);
        Ok(rx)
    }

    async fn handle_events(&self) -> Result<()> {
        let mut buf = [0u8; NL_CONNECTOR_MAX_MSG_SIZE];

        loop {
            let pid = self.netlink.read_event_async(&mut buf).await?;

            let mut interest_group = self.interest.lock().await;
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
                    Ok(_) => { /* connection closed */ }
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
        let _ = self.inner.netlink.stop();
        self.aborter.abort();
    }
}

impl AsyncBackend for AsyncNetlinkBackend {
    async fn waitpid_async(&self, pid: Pid) -> Result<()> {
        if !utils::process_exists(pid) {
            return Err(Error::from_raw_os_error(libc::ESRCH));
        }

        let rx = self.interest(pid).await?;
        match rx.await {
            Ok(_) => Ok(()),
            Err(_) => Err(ErrorKind::BrokenPipe.into()),
        }
    }
}
