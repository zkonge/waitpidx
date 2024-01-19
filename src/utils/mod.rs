use std::io::Error;

use rustix::process::Pid;

pub mod incomplete_array;

#[must_use]
pub fn process_exists(pid: Pid) -> bool {
    // SAFETY: kill with signal 0 does not affect anything
    let ret = unsafe { libc::kill(pid.as_raw_nonzero().get(), 0) };

    ret == 0 || Error::last_os_error().raw_os_error() != Some(libc::ESRCH)
}
