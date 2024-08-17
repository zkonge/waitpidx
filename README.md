# waitpidx

**waitpid** e**x**ternally for Linux

Wait for a process to terminate, not only available for child processes.

Supports `sync` and `async` (with tokio) mode.

⚠️ **WARNING**: This crate is still in development, `netlink` backend may not work as your expectation.

## Usage

```rust
use waitpidx::waitpid;

fn main() {
    waitpid(1234u32, Some(Duration::from_secs(1))).unwrap();
}
```

or

```rust
use waitpidx::waitpid_async;

#[tokio::main]
async fn main() {
    waitpid_async(1234u32).await.unwrap();
}
```

# Waiter backends

+ pidfd_open (Linux 5.3+, default)
+ netlink with cBPF (Linux 3.19+)

# Feature

Default features:

+ `async`

Following features are disabled by default:

+ `netlink`

# Advanced Usage

## wait many PIDs

TBD

# License

Apache-2.0
