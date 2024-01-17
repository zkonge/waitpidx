#[cfg(feature = "async")]
mod async_;
mod binding;
mod bpf;
mod connection;
mod sync;

#[cfg(feature = "async")]
pub(crate) use async_::AsyncNetlinkBackend;
pub(crate) use sync::NetlinkBackend;
