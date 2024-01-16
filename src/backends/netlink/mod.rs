mod binding;
mod bpf;

use std::{
    collections::HashMap,
    ffi::c_int,
    io, iter,
    mem::{size_of, transmute},
    sync::Mutex,
    time::Duration,
};

use binding::*;
use libc::sockaddr;
use linux_raw_sys::netlink;
use rustix::{
    fd::{AsFd, AsRawFd, OwnedFd},
    net::{self, netlink as rustix_netlink, AddressFamily, RecvFlags, SendFlags, SocketType},
    process::{self, Pid},
};

use self::bpf::apply_bpf_filter;
use super::Backend;
use crate::utils::incomplete_array::IncompleteArray;

type ExitNotifier = crossbeam_channel::Sender<()>;
type ExitReceiver = crossbeam_channel::Receiver<()>;

fn parse_netlink_control_message(buf: &[u8; CONNECTOR_MAX_MSG_SIZE]) -> Option<Pid> {
    let nlh_ptr = buf.as_ptr();
    let nlh = unsafe { &*(nlh_ptr as *const netlink::nlmsghdr) };

    let cn_msg_ptr = unsafe { nlh_ptr.add(size_of::<netlink::nlmsghdr>()) };
    let cn_msg = unsafe { &*(cn_msg_ptr as *const cn_msg) };

    // if nlh.nlmsg_type == NLMSG_ERROR as u16 {
    //     return Ok(());
    // }
    if nlh.nlmsg_type != NLMSG_DONE as u16 {
        return None;
    }

    let event: &[u8; size_of::<proc_event>()] =
        match unsafe { cn_msg.data.as_slice(cn_msg.len as usize) }.try_into() {
            Ok(x) => x,
            Err(_) => return None,
        };
    let event: &proc_event = unsafe { transmute(event) };

    if !matches!(event.what, proc_cn_event::PROC_EVENT_EXIT) {
        return None;
    }

    let pid = Pid::from_raw(event.event_data.process_tgid as i32)
        .expect("kernel should not send invalid pid");

    Some(pid)
}

#[derive(Debug)]
pub struct NetlinkBackend {
    netlink_fd: OwnedFd,

    interest: Mutex<HashMap<Pid, Vec<ExitNotifier>>>,
}

impl NetlinkBackend {
    pub fn new() -> io::Result<Self> {
        let fd = net::socket(
            AddressFamily::NETLINK,
            SocketType::DGRAM,
            Some(rustix_netlink::CONNECTOR),
        )?;

        let pid = process::getpid().as_raw_nonzero().get();

        let sa_nl = netlink::sockaddr_nl {
            nl_family: AddressFamily::NETLINK.as_raw(),
            nl_pad: 0, // unspecified
            nl_pid: pid as u32,
            nl_groups: CN_IDX_PROC,
        };

        let bind_result = unsafe {
            libc::bind(
                fd.as_raw_fd(),
                (&sa_nl) as *const netlink::sockaddr_nl as *const sockaddr,
                size_of::<netlink::sockaddr_nl>() as _,
            )
        };

        if -1 == bind_result {
            return Err(io::Error::last_os_error());
        }

        let mut buf = [0u8; NL_MESSAGE_SIZE];

        // headers
        {
            let nlh_ptr = buf.as_mut_ptr();
            let nlh = unsafe { &mut *(nlh_ptr as *mut netlink::nlmsghdr) };
            *nlh = netlink::nlmsghdr {
                nlmsg_len: NL_MESSAGE_SIZE as u32,
                nlmsg_type: NLMSG_DONE as u16,
                nlmsg_flags: 0,
                nlmsg_seq: 0,
                nlmsg_pid: pid as u32,
            };
        }

        // msg
        {
            let msg_ptr = unsafe { buf.as_mut_ptr().add(size_of::<netlink::nlmsghdr>()) };
            let cn_msg = unsafe { &mut *(msg_ptr as *mut cn_msg) };
            *cn_msg = cn_msg {
                id: cb_id {
                    idx: CN_IDX_PROC,
                    val: CN_VAL_PROC,
                },
                seq: 0,
                ack: 0,
                len: size_of::<c_int>() as u16,
                flags: 0,
                data: IncompleteArray::new(),
            };

            let data_ptr = unsafe { msg_ptr.add(size_of::<cn_msg>()) };
            let data = unsafe { &mut *(data_ptr as *mut c_int) };
            *data = proc_cn_mcast_op::PROC_CN_MCAST_LISTEN as c_int;
        }

        net::send(&fd, &buf, SendFlags::empty())?;

        Ok(Self {
            netlink_fd: fd,
            interest: Default::default(),
        })
    }

    pub fn interest(&self, pid: Pid) -> io::Result<ExitReceiver> {
        let mut interest_group = self.interest.lock().unwrap();

        let keys = interest_group
            .keys()
            .copied()
            .chain(iter::once(pid))
            .collect::<Vec<_>>();
        apply_bpf_filter(self.netlink_fd.as_fd(), &keys)?;

        let (tx, rx) = crossbeam_channel::bounded(0);

        interest_group.entry(pid).or_default().push(tx);
        Ok(rx)
    }

    pub fn handle_events(&self) -> io::Result<()> {
        let mut buf = [0u8; CONNECTOR_MAX_MSG_SIZE];

        loop {
            let n = net::recv(&self.netlink_fd, &mut buf, RecvFlags::empty())?;
            if n == 0 {
                return Ok(());
            }

            let pid = match parse_netlink_control_message(&buf) {
                Some(pid) => pid,
                None => continue,
            };

            let mut interest_group = self.interest.lock().unwrap();
            if let Some(notifiers) = interest_group.remove(&pid) {
                for notifier in notifiers {
                    let _ = notifier.send(()); // don't care if the receiver is dropped
                }
            }

            let keys = interest_group.keys().copied().collect::<Vec<_>>();
            match apply_bpf_filter(self.netlink_fd.as_fd(), &keys) {
                Ok(_) => (),
                Err(_) => return Ok(()), // OwnedFd is dropped
            }
        }
    }
}

impl Backend for NetlinkBackend {
    fn waitpid(&self, pid: Pid, timeout: Option<Duration>) -> io::Result<()> {
        let rx = self.interest(pid)?;

        match timeout {
            Some(timeout) => {
                if rx.recv_timeout(timeout).is_err() {
                    return Err(io::ErrorKind::TimedOut.into());
                }
            }

            None => {
                if rx.recv().is_err() {
                    return Err(io::ErrorKind::Interrupted.into());
                }
            }
        };

        let _ = self.handle_events();

        Ok(())
    }
}
