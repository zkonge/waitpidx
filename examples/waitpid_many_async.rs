use std::env::args;

use futures_util::future::select_all;
use waitpidx::{pidfd::AsyncPidFd, Pid};

async fn amain() {
    let mut pids: Vec<Pid> = args()
        .skip(1)
        .map(|x| x.parse().expect("parse PID failed"))
        .map(|p| Pid::from_raw(p).expect("invalid PID"))
        .collect();

    if pids.is_empty() {
        eprintln!("Usage: {} <pid> [<pid> ...]", args().next().unwrap());
        return;
    }

    let mut pidfds: Vec<AsyncPidFd> = pids
        .iter()
        .map(|&p| AsyncPidFd::new(p).expect("watch PID failed"))
        .collect();

    while !pidfds.is_empty() {
        let (r, idx, _) = select_all(pidfds.iter().map(|x| x.wait())).await;
        let pid = pids.swap_remove(idx);
        pidfds.swap_remove(idx);

        println!("one process exit: {r:?} {pid:?}");
    }
}

fn main() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(amain())
}
