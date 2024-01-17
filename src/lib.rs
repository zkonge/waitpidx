use std::{io, time::Duration};

use backends::*;
use rustix::process::Pid;

mod backends;
mod utils;

pub use utils::process_exists;

#[allow(unreachable_code)] // while netlink feature disabled
pub fn waitpid(pid: u32, timeout: Option<Duration>) -> io::Result<()> {
    let pid = Pid::from_raw(pid as i32).ok_or(io::ErrorKind::InvalidInput)?;

    // 1. try pidfd
    match PidFdBackend.waitpid(pid, timeout) {
        // kernel 5.2- doesn't support pidfd_open, try netlink
        #[cfg(feature = "netlink")]
        Err(e) if e.kind() == io::ErrorKind::Unsupported => (),
        r => return r,
    }

    // 2. try netlink
    #[cfg(feature = "netlink")]
    {
        let netlink = NetlinkBackend::new()?;
        netlink.waitpid(pid, timeout)?;
    }

    Ok(())
}

#[cfg(feature = "async")]
#[allow(unreachable_code)]
pub async fn waitpid_async(pid: u32) -> io::Result<()> {
    let pid = Pid::from_raw(pid as i32).ok_or(io::ErrorKind::InvalidInput)?;

    // 1. try pidfd
    match PidFdBackend.waitpid_async(pid).await {
        // kernel 5.2- doesn't support pidfd_open, try netlink
        #[cfg(feature = "netlink")]
        Err(e) if e.kind() == io::ErrorKind::Unsupported => (),
        r => return r,
    }

    // 2. try netlink
    #[cfg(feature = "netlink")]
    {
        let netlink = AsyncNetlinkBackend::new()?;
        netlink.waitpid_async(pid).await?;
    }

    Ok(())
}

#[cfg(not(target_os = "linux"))]
compile_error!("waitpidx only supports Linux");
