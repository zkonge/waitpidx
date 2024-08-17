mod backends;
mod utils;

use std::{io, time::Duration};

use backends::*;
use rustix::process::Pid;

pub use crate::{backends::pidfd, utils::process_exists};

#[allow(unreachable_code)] // while netlink feature disabled
pub fn waitpid(pid: u32, timeout: Option<Duration>) -> io::Result<()> {
    let pid = Pid::from_raw(pid as i32).ok_or(io::ErrorKind::InvalidInput)?;

    // 1. try pidfd
    match Backend::waitpid(&PidFdBackend, pid, timeout) {
        // kernel 5.2- doesn't support pidfd_open, try netlink
        #[cfg(feature = "netlink")]
        Err(e) if e.kind() == io::ErrorKind::Unsupported => (),
        r => return r,
    }

    // 2. try netlink
    #[cfg(feature = "netlink")]
    NetlinkBackend::new()?.waitpid(pid, timeout)?;

    Ok(())
}

#[cfg(feature = "async")]
#[allow(unreachable_code)]
pub async fn waitpid_async(pid: u32) -> io::Result<()> {
    let pid = Pid::from_raw(pid as i32).ok_or(io::ErrorKind::InvalidInput)?;

    // 1. try pidfd
    match AsyncBackend::waitpid(&PidFdBackend, pid).await {
        // kernel 5.2- doesn't support pidfd_open, try netlink
        #[cfg(feature = "netlink")]
        Err(e) if e.kind() == io::ErrorKind::Unsupported => (),
        r => return r,
    }

    // 2. try netlink
    #[cfg(feature = "netlink")]
    AsyncNetlinkBackend::new()?.waitpid(pid).await?;

    Ok(())
}

#[cfg(not(target_os = "linux"))]
compile_error!("waitpidx only supports Linux");
