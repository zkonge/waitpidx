use std::{
    io::{Error, ErrorKind, Result},
    mem::size_of,
    ptr::addr_of,
    time::Duration,
};

use libc::sockaddr;
use linux_raw_sys::netlink;
use rustix::{
    event,
    fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd},
    net::{self, netlink as rustix_netlink, AddressFamily, RecvFlags, SendFlags, SocketType},
    process::{self, Pid},
};

use super::{binding::*, bpf};
use crate::utils::incomplete_array::IncompleteArray;

#[derive(Debug)]
pub(super) struct NetlinkConnection {
    fd: OwnedFd,
}

impl NetlinkConnection {
    pub(super) fn new() -> Result<Self> {
        let fd = net::socket(
            AddressFamily::NETLINK,
            SocketType::DGRAM,
            Some(rustix_netlink::CONNECTOR),
        )?;

        let self_pid = process::getpid().as_raw_nonzero().get();

        let sa_nl = netlink::sockaddr_nl {
            nl_family: AddressFamily::NETLINK.as_raw(),
            nl_pad: 0, // unspecified
            nl_pid: self_pid as u32,
            nl_groups: CN_IDX_PROC,
        };

        // SAFETY: sockaddr_nl can be safely passed as a valid sockaddr pointer
        let bind_ret = unsafe {
            libc::bind(
                fd.as_raw_fd(),
                addr_of!(sa_nl) as *const sockaddr,
                size_of::<netlink::sockaddr_nl>() as _,
            )
        };

        if bind_ret == -1 {
            return Err(Error::last_os_error());
        }

        Ok(Self { fd })
    }

    pub(super) fn start(&self) -> Result<()> {
        let buf = make_netlink_control_message(proc_cn_mcast_op::PROC_CN_MCAST_LISTEN);

        net::send(&self.fd, &buf, SendFlags::empty())?;

        Ok(())
    }

    pub(super) fn stop(&self) -> Result<()> {
        let buf = make_netlink_control_message(proc_cn_mcast_op::PROC_CN_MCAST_IGNORE);

        net::send(&self.fd, &buf, SendFlags::empty())?;

        Ok(())
    }

    pub(super) fn interest(&self, pids: Option<&[Pid]>) -> Result<()> {
        match pids {
            Some(pids) => bpf::apply_bpf_filter(self.fd.as_fd(), pids),
            None => bpf::detach_bpf_filter(self.fd.as_fd()),
        }
    }

    // WARNING: multiple reader in the same time may cause unwanted behavior.
    // current polling implementation is atomic, althrough it's thread-safe in Rust semantics,
    pub(super) fn read_event(
        &self,
        buf: &mut [u8; NL_CONNECTOR_MAX_MSG_SIZE],
        timeout: Option<Duration>,
    ) -> Result<Pid> {
        let n = match timeout {
            Some(timeout) => read_with_timeout(self.fd.as_fd(), buf, timeout)?,
            None => net::recv(&self.fd, buf, RecvFlags::empty())?, // infinity
        };
        if n == 0 {
            return Err(ErrorKind::UnexpectedEof.into());
        }

        parse_netlink_event_message(buf).ok_or(ErrorKind::InvalidData.into())
    }

    // WARNING: multiple reader in the same time may cause unwanted behavior.
    // current polling implementation is atomic, althrough it's thread-safe in Rust semantics,
    #[cfg(feature = "async")]
    pub(super) async fn read_event_async(
        &self,
        buf: &mut [u8; NL_CONNECTOR_MAX_MSG_SIZE],
    ) -> Result<Pid> {
        let fd = tokio::io::unix::AsyncFd::new(self.fd.as_fd())?;

        loop {
            let mut guard = fd.readable().await?;

            match guard.try_io(|inner| {
                net::recv(inner.get_ref(), buf, RecvFlags::empty()).map_err(Into::into)
            }) {
                Ok(Ok(n)) => {
                    if n == 0 {
                        return Err(ErrorKind::UnexpectedEof.into());
                    }

                    return parse_netlink_event_message(buf).ok_or(ErrorKind::InvalidData.into());
                }
                Ok(Err(e)) => return Err(e),
                // TODO: it accutally impossible to reach this branch, didn't set NON_BLOCK flag
                Err(_would_block) => continue,
            }
        }
    }
}

impl AsFd for NetlinkConnection {
    fn as_fd(&self) -> BorrowedFd {
        self.fd.as_fd()
    }
}

fn read_with_timeout(fd: BorrowedFd, buf: &mut [u8], timeout: Duration) -> Result<usize> {
    let timeout = timeout.as_millis().try_into().unwrap_or(i32::MAX);

    let mut fds = [event::PollFd::new(&fd, event::PollFlags::IN)];

    if event::poll(&mut fds, timeout)? == 0 {
        return Err(ErrorKind::TimedOut.into());
    }

    net::recv(fd, buf, RecvFlags::empty()).map_err(Into::into)
}

fn make_netlink_control_message(control_op: proc_cn_mcast_op) -> [u8; NL_MESSAGE_MCAST_SIZE] {
    let self_pid = process::getpid().as_raw_nonzero().get();

    // send call needn't alignment, stack array is fine
    let mut buf = [0u8; NL_MESSAGE_MCAST_SIZE];

    // headers
    {
        let nlh_ptr = buf.as_mut_ptr() as *mut netlink::nlmsghdr;

        // SAFETY: nlh_ptr is a valid pointer to a netlink::nlmsghdr
        // and it's length is known and suitable for writing
        unsafe {
            nlh_ptr.write_unaligned(netlink::nlmsghdr {
                nlmsg_len: NL_MESSAGE_MCAST_SIZE as u32,
                nlmsg_type: NLMSG_DONE as u16,
                nlmsg_flags: 0,
                nlmsg_seq: 0,
                nlmsg_pid: self_pid as u32,
            });
        }
    }

    // msg
    {
        // SAFETY: structure layout is known and suitable for writing, no overflow
        let msg_ptr = unsafe { buf.as_mut_ptr().add(NLMSGHDR_SIZE) } as *mut cn_msg;

        // SAFETY: msg_ptr is a valid pointer to a cn_msg
        // and it's length is known and suitable for writing
        unsafe {
            msg_ptr.write_unaligned(cn_msg {
                id: cb_id {
                    idx: CN_IDX_PROC,
                    val: CN_VAL_PROC,
                },
                seq: 0,
                ack: 0,
                len: MCAST_OP_SIZE as u16,
                flags: 0,
                data: IncompleteArray::new(),
            })
        };
    }

    // msg data
    {
        // SAFETY: structure layout is known and suitable for writing, no overflow
        let data_ptr =
            unsafe { buf.as_mut_ptr().add(NL_MESSAGE_BASE_SIZE) } as *mut proc_cn_mcast_op;

        // SAFETY: data_ptr is a valid pointer to a c_int
        unsafe { data_ptr.write_unaligned(control_op as proc_cn_mcast_op) };
    }

    buf
}

fn parse_netlink_event_message(buf: &[u8; NL_CONNECTOR_MAX_MSG_SIZE]) -> Option<Pid> {
    let nlh_ptr = buf.as_ptr();
    // SAFETY: structure layout is known and suitable for writing, no overflow
    let cn_msg_ptr = unsafe { nlh_ptr.add(NLMSGHDR_SIZE) };
    let proc_event_ptr = unsafe { cn_msg_ptr.add(CN_MSG_SIZE) };

    // SAFETY: nlh_ptr is a valid pointer to a nlmsghdr
    // and it's length is known and suitable for reading
    let nlh = unsafe { (nlh_ptr as *const nlmsghdr).read_unaligned() };
    // // do we need to check nlh.nlmsg_type == NLMSG_ERROR and return Result?
    // if nlh.nlmsg_type == NLMSG_ERROR as u16 {
    //     return Ok(());
    // }
    if nlh.nlmsg_type != NLMSG_DONE as u16 {
        return None;
    }

    // SAFETY: cn_msg_ptr is a valid pointer to a cn_msg
    // and it's length is known and suitable for reading
    let cn_msg = unsafe { (cn_msg_ptr as *const cn_msg).read_unaligned() };
    if cn_msg.len as usize != PROC_EVENT_SIZE {
        return None;
    }

    // SAFETY: event is a valid pointer to a proc_event
    // and it's length is known and suitable for reading
    // DO NOT read cn_msg.data, it's not a valid stack array after copy
    let proc_event = unsafe { (proc_event_ptr as *const proc_event).read_unaligned() };
    if !matches!(proc_event.what, proc_cn_event::PROC_EVENT_EXIT) {
        return None;
    }

    Pid::from_raw(proc_event.event_data.process_tgid as i32)
}
