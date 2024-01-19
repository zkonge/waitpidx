use std::env::args;

async fn amain() {
    let pid = args()
        .nth(1)
        .expect("specify a PID(TGID) to wait")
        .parse::<u32>()
        .expect("parse PID failed");
    waitpidx::waitpid_async(pid).await.unwrap();
}

fn main() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(amain())
}
