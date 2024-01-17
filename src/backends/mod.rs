#[cfg(feature = "netlink")]
mod netlink;
mod pidfd;

use std::{io, time::Duration};

#[cfg(all(feature = "netlink", feature = "async"))]
pub(crate) use netlink::AsyncNetlinkBackend;
#[cfg(feature = "netlink")]
pub(crate) use netlink::NetlinkBackend;
pub(crate) use pidfd::PidFdBackend;
use rustix::process::Pid;

pub(crate) trait Backend {
    fn waitpid(&self, pid: Pid, timeout: Option<Duration>) -> io::Result<()>;
}

#[cfg(feature = "async")]
pub(crate) trait AsyncBackend {
    async fn waitpid_async(&self, pid: Pid) -> io::Result<()>;
}
