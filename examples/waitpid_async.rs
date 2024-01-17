use std::env::args;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let pid = args()
        .nth(1)
        .expect("specify a PID(TGID) to wait")
        .parse::<u32>()
        .expect("parse PID failed");
    waitpidx::waitpid_async(pid).await.unwrap();
}
