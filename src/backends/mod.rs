mod netlink;
mod pidfd;

use std::{io, time::Duration};

pub(crate) use netlink::NetlinkBackend;
pub(crate) use pidfd::PidFdBackend;
use rustix::process::Pid;

pub(crate) trait Backend {
    fn waitpid(&self, pid: Pid, timeout: Option<Duration>) -> io::Result<()>;
}

pub(crate) trait AsyncBackend {
    async fn async_waitpid(&self, pid: Pid) -> io::Result<()>;
}
