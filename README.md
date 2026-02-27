# waitpidx

**waitpid** e**x**ternally for Linux

Wait for a process to terminate, not only available for child processes.

Supports `sync` and `async` (with tokio) mode.

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
+ ~~netlink with cBPF (Linux 3.19+)~~ (It existed in early versions, but I believe it is not production ready, so I removed it for now. It may be added back in the future.)

# Feature

Default features:

+ `async`

# License

Apache-2.0
