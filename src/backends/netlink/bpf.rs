use std::{
    io::{Error, Result},
    mem::size_of,
};

use classic_bpf::*;
use linux_raw_sys::netlink::NLMSG_DONE;
use memoffset::offset_of;
use rustix::{
    fd::{AsRawFd, BorrowedFd},
    process::Pid,
};
use BPFFilter as B;

use super::binding::{
    cb_id, cn_msg, exit_proc_event, nlmsghdr, proc_cn_event, proc_event, CN_IDX_PROC, CN_VAL_PROC,
};

// cBPF modified from https://github.com/Parrot-Developers/fusion/blob/master/pidwatch/src/pidwatch.c
// with BSD-3-Clause license
fn assembly_filter(pids: &[Pid]) -> Vec<BPFFilter> {
    let mut filter = Vec::with_capacity(15 /* head */ + 1 /* tail */ + 3 /* pid asm */ * pids.len());

    filter.extend([
        /* check message's type is NLMSG_DONE */
        B::bpf_stmt(
            BPF_LD | BPF_H | BPF_ABS,
            offset_of!(nlmsghdr, nlmsg_type) as _,
        ),
        B::bpf_jump(
            BPF_JMP | BPF_JEQ | BPF_K,
            (NLMSG_DONE as u16).to_be() as u32,
            1,
            0,
        ),
        B::bpf_stmt(BPF_RET | BPF_K, 0x0), /* message is dropped */
        /* check message comes from the kernel */
        B::bpf_stmt(
            BPF_LD | BPF_W | BPF_ABS,
            offset_of!(nlmsghdr, nlmsg_pid) as _,
        ),
        B::bpf_jump(BPF_JMP | BPF_JEQ | BPF_K, 0, 1, 0),
        B::bpf_stmt(BPF_RET | BPF_K, 0x0), /* message is dropped */
        /* check it's a proc connector event part 1 */
        B::bpf_stmt(
            BPF_LD | BPF_W | BPF_ABS,
            (size_of::<nlmsghdr>() + offset_of!(cn_msg, id) + offset_of!(cb_id, idx)) as _,
        ),
        B::bpf_jump(BPF_JMP | BPF_JEQ | BPF_K, CN_IDX_PROC.to_be(), 1, 0),
        B::bpf_stmt(BPF_RET | BPF_K, 0x0), /* message is dropped */
        /* check it's a proc connector event part 2 */
        B::bpf_stmt(
            BPF_LD | BPF_W | BPF_ABS,
            (size_of::<nlmsghdr>() + offset_of!(cn_msg, id) + offset_of!(cb_id, val)) as _,
        ),
        B::bpf_jump(BPF_JMP | BPF_JEQ | BPF_K, CN_VAL_PROC.to_be(), 1, 0),
        B::bpf_stmt(BPF_RET | BPF_K, 0x0), /* message is dropped */
        /* check it's an exit message*/
        B::bpf_stmt(
            BPF_LD | BPF_W | BPF_ABS,
            (size_of::<nlmsghdr>() + size_of::<cn_msg>() + offset_of!(proc_event, what)) as _,
        ),
        B::bpf_jump(
            BPF_JMP | BPF_JEQ | BPF_K,
            (proc_cn_event::PROC_EVENT_EXIT as u32).to_be(),
            1,
            0,
        ),
        B::bpf_stmt(BPF_RET | BPF_K, 0x0), /* message is dropped */
    ]);

    for p in pids {
        filter.extend([
            /* check the pid matches */
            B::bpf_stmt(
                BPF_LD | BPF_W | BPF_ABS,
                (size_of::<nlmsghdr>()
                    + size_of::<cn_msg>()
                    + offset_of!(proc_event, event_data)
                    + offset_of!(exit_proc_event, process_tgid)) as _,
            ),
            /* here pid has been tested >= 1, so the cast is ok */
            B::bpf_jump(
                BPF_JMP | BPF_JEQ | BPF_K,
                p.as_raw_nonzero().get().to_be() as _,
                0,
                1,
            ),
            /* message is sent to user space */
            B::bpf_stmt(BPF_RET | BPF_K, 0xffffffff),
        ]);
    }

    filter.extend([
        B::bpf_stmt(BPF_RET | BPF_K, 0x0), /* message is dropped */
    ]);

    filter
}

pub fn apply_bpf_filter(fd: BorrowedFd, pid: &[Pid]) -> Result<()> {
    BPFFProg::new(&assembly_filter(pid))
        .attach_filter(fd.as_raw_fd())
        .map_err(Error::from_raw_os_error)
}

pub fn detach_bpf_filter(fd: BorrowedFd) -> Result<()> {
    detach_filter(fd.as_raw_fd()).map_err(Error::from_raw_os_error)
}
