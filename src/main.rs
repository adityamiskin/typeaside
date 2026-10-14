mod ui;

fn main() {
    let started = std::time::Instant::now();
    let demo = std::env::args().any(|arg| arg == "--demo");
    if let Err(error) = ui::run(demo, started) {
        eprintln!("hermes: {error:#}");
        std::process::exit(1);
    }
}
