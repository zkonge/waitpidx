mod backends;
mod utils;

use std::{
    io::{ErrorKind, Result},
    time::Duration,
};

pub use rustix::process::Pid;

use crate::backends::*;
pub use crate::{backends::pidfd, utils::process_exists};

#[allow(unreachable_code)] // while netlink feature disabled
pub fn waitpid(pid: u32, timeout: Option<Duration>) -> Result<()> {
    let pid = Pid::from_raw(pid as i32).ok_or(ErrorKind::InvalidInput)?;

    // 1. try pidfd
    match Backend::waitpid(&pidfd::PidFdBackend, pid, timeout) {
        // kernel 5.2- doesn't support pidfd_open, try netlink
        #[cfg(feature = "netlink")]
        Err(e) if e.kind() == ErrorKind::Unsupported => (),
        r => return r,
    }

    // 2. try netlink
    #[cfg(feature = "netlink")]
    netlink::NetlinkBackend::new()?.waitpid(pid, timeout)?;

    Ok(())
}

#[cfg(feature = "async")]
#[allow(unreachable_code)]
pub async fn waitpid_async(pid: u32) -> Result<()> {
    use backends::AsyncBackend;

    let pid = Pid::from_raw(pid as i32).ok_or(ErrorKind::InvalidInput)?;

    // 1. try pidfd
    match AsyncBackend::waitpid(&pidfd::PidFdBackend, pid).await {
        // kernel 5.2- doesn't support pidfd_open, try netlink
        #[cfg(feature = "netlink")]
        Err(e) if e.kind() == ErrorKind::Unsupported => (),
        r => return r,
    }

    // 2. try netlink
    #[cfg(feature = "netlink")]
    netlink::AsyncNetlinkBackend::new()?.waitpid(pid).await?;

    Ok(())
}

#[cfg(not(target_os = "linux"))]
compile_error!("waitpidx only supports Linux");
