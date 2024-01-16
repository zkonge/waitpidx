use std::env::args;

fn main() {
    waitpidx::waitpid(args().nth(1).unwrap().parse().unwrap(), None).unwrap();
}
