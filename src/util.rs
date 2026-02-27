use rustix::{
    io::Errno,
    process::{test_kill_process, Pid},
};

/// Return whether a process currently exists.
///
/// This uses signal-0 style probing (`kill(pid, 0)` semantics).
///
/// Note that this is inherently race-prone: a process may exit immediately
/// after this function returns `true`.
#[must_use]
pub fn process_exists(pid: Pid) -> bool {
    test_kill_process(pid) != Err(Errno::SRCH)
}
