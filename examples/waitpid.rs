use std::{env::args, sync::Arc, time::Duration};

use waitpidx::Pid;

fn main() {
    let pid = args()
        .nth(1)
        .expect("specify a PID(TGID) to wait")
        .parse::<u32>()
        .expect("parse PID failed");
    // waitpidx::waitpid(pid, None).unwrap();

    let pfd = Arc::new(waitpidx::pidfd::PidFd::new(Pid::from_raw(pid as _).unwrap()).unwrap());

    std::thread::spawn({
        let pfd = pfd.clone();
        move || loop {
            match pfd.is_exited() {
                Ok(x) => {
                    println!("thread: pid is exited: {}", x);
                    std::thread::sleep(Duration::from_secs(1));
                }
                Err(e) => {
                    println!("thread: error: {e:?}");
                    return;
                }
            }
        }
    });

    match pfd.wait(None) {
        Ok(_) => {
            println!("main: pid is exited");
        }
        Err(e) => {
            println!("main: error: {e:?}");
        }
    }

    std::thread::sleep(Duration::from_secs(10));
}
