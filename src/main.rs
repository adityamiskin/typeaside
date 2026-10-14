mod ui;

fn main() {
    let demo = std::env::args().any(|arg| arg == "--demo");
    if let Err(error) = ui::run(demo) {
        eprintln!("hermes: {error:#}");
        std::process::exit(1);
    }
}
