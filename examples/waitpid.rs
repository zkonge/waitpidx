use std::env::args;

fn main() {
    let pid = args()
        .nth(1)
        .expect("specify a PID(TGID) to wait")
        .parse::<u32>()
        .expect("parse PID failed");
    waitpidx::waitpid(pid, None).unwrap();
}
