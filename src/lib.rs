use std::{io, sync::Arc, time::Duration};

use backends::{Backend, NetlinkBackend, PidFdBackend};
use rustix::process::Pid;

mod backends;
mod utils;

pub fn waitpid(pid: u32, timeout: Option<Duration>) -> io::Result<()> {
    let pid = Pid::from_raw(pid as i32).ok_or(io::ErrorKind::InvalidInput)?;

    // 1. try pidfd
    match PidFdBackend.waitpid(pid, timeout) {
        Err(e) if e.kind() == io::ErrorKind::Unsupported => (),
        r => return r,
    }

    // 2. try netlink
    let nl = Arc::new(NetlinkBackend::new()?);

    let nlc = nl.clone();
    std::thread::spawn(move || {
        nl.handle_events().unwrap();
    });

    let c = nlc.interest(pid)?;

    match timeout {
        Some(timeout) => {
            if c.recv_timeout(timeout).is_err() {
                return Err(io::ErrorKind::TimedOut.into());
            }
        }
        None => {
            c.recv().unwrap();
        }
    }

    Ok(())
}
