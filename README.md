# waitpidx

[![crates.io](https://img.shields.io/crates/v/waitpidx.svg)](https://crates.io/crates/waitpidx)
[![docs.rs](https://docs.rs/waitpidx/badge.svg)](https://docs.rs/waitpidx)
[![license](https://img.shields.io/crates/l/waitpidx.svg)](LICENSE)

`waitpidx` provides Linux process waiting by PID (not limited to child processes).

It uses `pidfd_open(2)` + `poll(2)` under the hood and supports both:

- synchronous waiting
- asynchronous waiting with Tokio

## Requirements

- Linux only
- Kernel `>= 5.3` (`pidfd_open`)
- Rust stable (Edition 2024)

## Install

```toml
[dependencies]
waitpidx = "0.0.1"
```

Disable default async feature when you only need sync API:

```toml
[dependencies]
waitpidx = { version = "0.0.1", default-features = false }
```

## Quick start

### Sync API

```rust
use std::{process::Command, time::Duration};

fn main() -> std::io::Result<()> {
    let mut child = Command::new("sleep").arg("0.1").spawn()?;
    waitpidx::waitpid(child.id(), Some(Duration::from_secs(5)))?;
    child.wait()?;
    Ok(())
}
```

### Async API (Tokio)

```rust
use std::process::Command;

#[tokio::main(flavor = "current_thread")]
async fn main() -> std::io::Result<()> {
    let mut child = Command::new("sleep").arg("0.1").spawn()?;
    waitpidx::waitpid_async(child.id()).await?;
    child.wait()?;
    Ok(())
}
```

## Public API

- `waitpid(pid, timeout)`
    - Wait for process exit with optional timeout.
    - Returns `TimedOut` when timeout expires.
- `waitpid_async(pid)`
    - Async wait for process exit.
- `pidfd::PidFd` / `pidfd::AsyncPidFd`
    - Low-level wrappers if you need reusable wait handles.
- `process_exists(pid)`
    - Cheap existence check based on signal 0 semantics.

## Error behavior

Common `std::io::ErrorKind` values:

- `InvalidInput`: invalid PID (for example `0`), or invalid timeout conversion
- `TimedOut`: timeout reached in sync wait
- `NotFound` / `PermissionDenied` / others: forwarded from OS (`pidfd_open`, `poll`, etc.)

## Features

- `async` (default): enables Tokio-based async API

## Examples

Run included examples:

```bash
cargo run --example waitpid -- <pid>
cargo run --example waitpid_async --features async -- <pid>
cargo run --example waitpid_many_async --features async -- <pid1> <pid2>
```

## License

Apache-2.0
