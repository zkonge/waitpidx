#[cfg(feature = "netlink")]
pub mod netlink;
pub mod pidfd;

use std::{io::Result, time::Duration};

use rustix::process::Pid;

pub(crate) trait Backend {
    fn waitpid(&self, pid: Pid, timeout: Option<Duration>) -> Result<()>;
}

#[cfg(feature = "async")]
pub(crate) trait AsyncBackend {
    async fn waitpid(&self, pid: Pid) -> Result<()>;
}
