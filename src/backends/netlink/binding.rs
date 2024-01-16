use std::{ffi::c_int, mem::size_of};

pub(super) use linux_raw_sys::netlink::{nlmsghdr, NLMSG_DONE};

use crate::utils::incomplete_array::IncompleteArray;

pub(super) const CN_IDX_PROC: u32 = 0x1;
pub(super) const CN_VAL_PROC: u32 = 0x1;

pub(super) const NL_MESSAGE_SIZE: usize =
    size_of::<nlmsghdr>() + size_of::<cn_msg>() + size_of::<c_int>();
pub(super) const CONNECTOR_MAX_MSG_SIZE: usize = 16384;

#[allow(non_camel_case_types)]
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub(super) struct cb_id {
    pub idx: u32,
    pub val: u32,
}

#[allow(non_camel_case_types, unused)]
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub(super) enum proc_cn_mcast_op {
    PROC_CN_MCAST_LISTEN = 1,
    PROC_CN_MCAST_IGNORE = 2,
}

#[allow(non_camel_case_types)]
#[repr(C)]
#[derive(Debug)]
pub(super) struct cn_msg {
    pub id: cb_id,

    pub seq: u32,
    pub ack: u32,

    pub len: u16, // Length of the following data
    pub flags: u16,
    pub data: IncompleteArray<u8>,
}

// cn_proc.h
#[allow(non_camel_case_types, unused)]
#[repr(C)]
#[derive(Debug, Copy, Clone)]
// TODO: UB no all coverage
pub(super) enum proc_cn_event {
    /* Use successive bits so the enums can be used to record
     * sets of events as well
     */
    PROC_EVENT_NONE = 0x00000000,
    PROC_EVENT_FORK = 0x00000001,
    PROC_EVENT_EXEC = 0x00000002,
    PROC_EVENT_UID = 0x00000004,
    PROC_EVENT_GID = 0x00000040,
    PROC_EVENT_SID = 0x00000080,
    PROC_EVENT_PTRACE = 0x00000100,
    PROC_EVENT_COMM = 0x00000200,
    /* "next" should be 0x00000400 */
    /* "last" is the last process event: exit,
     * while "next to last" is coredumping event
     * before that is report only if process dies
     * with non-zero exit status
     */
    PROC_EVENT_NONZERO_EXIT = 0x20000000,
    PROC_EVENT_COREDUMP = 0x40000000,
    // PROC_EVENT_EXIT = 0x80000000,
    PROC_EVENT_EXIT = i32::MIN as isize, // 32bit overflow, make clippy happy with negative literal
}

#[allow(non_camel_case_types)]
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub(super) struct exit_proc_event {
    pub process_pid: u32,
    pub process_tgid: u32,
    pub exit_code: u32,
    pub exit_signal: u32,
    pub parent_pid: u32,
    pub parent_tgid: u32,
}

#[allow(non_camel_case_types)]
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub(super) struct proc_event {
    pub what: proc_cn_event,
    pub cpu: u32,
    /// Number of nano seconds since system boot
    pub timestamp_ns: u64,
    // specially, exit_proc_event is the longest struct in proc_event union,
    // so it's safe to use it as the type of event_data
    pub event_data: exit_proc_event, /* must be last field of proc_event struct */
}
