pub(crate) mod incomplete_array;

use rustix::{
    io::Errno,
    process::{test_kill_process, Pid},
};

#[must_use]
pub fn process_exists(pid: Pid) -> bool {
    test_kill_process(pid) != Err(Errno::SRCH)
}
