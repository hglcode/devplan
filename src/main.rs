#[cfg(unix)]
fn signal_reset_pipe() {
    unsafe extern "C" {
        fn signal(sig: i32, handler: usize) -> usize;
    }
    unsafe {
        signal(13 /* SIGPIPE */, 0 /* SIG_DFL */);
    }
}

fn main() -> anyhow::Result<()> {
    #[cfg(unix)]
    signal_reset_pipe();
    devplan::run()
}
