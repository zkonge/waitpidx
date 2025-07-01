#[cfg(feature = "async-netlink")]
mod async_;
mod binding;
mod bpf;
mod connection;
mod sync;

#[cfg(feature = "async-netlink")]
pub(crate) use self::async_::AsyncNetlinkBackend;
pub(crate) use self::sync::NetlinkBackend;
