mod ui;

fn main() {
    if std::env::args().any(|arg| arg == "--local-worker") {
        if let Err(error) = typeaside::local::worker_main() {
            eprintln!("typeaside local: {error:#}");
            std::process::exit(1);
        }
        return;
    }
    if std::env::args().any(|arg| arg == "--update-now") {
        if std::env::args().any(|arg| arg == "--demo") {
            return;
        }
        let result = (|| -> anyhow::Result<()> {
            let release =
                typeaside::update::check()?.ok_or_else(|| anyhow::anyhow!("No newer release"))?;
            typeaside::update::install(&release, |progress| eprintln!("{progress:?}"))?;
            Ok(())
        })();
        match result {
            Ok(()) => typeaside::update::quit_after_install(),
            Err(error) => {
                eprintln!("Update failed: {error:#}");
                std::process::exit(1);
            }
        }
    }
    let started = std::time::Instant::now();
    #[cfg(target_os = "linux")]
    prefer_integrated_gpu();
    let demo = std::env::args().any(|arg| arg == "--demo");
    let hidden = std::env::args().any(|arg| arg == "--hidden");
    #[cfg(target_os = "linux")]
    let hidden = hidden && !typeaside::update::restarted_for_update();
    if let Err(error) = ui::run(demo, hidden, started) {
        eprintln!("typeaside: {error:#}");
        std::process::exit(1);
    }
}

/// On hybrid laptops, loading the NVIDIA Vulkan driver wakes the discrete GPU (about 2 s) and
/// maps ~50 MiB of driver libraries, for a UI the integrated GPU draws just as well.
#[cfg(target_os = "linux")]
fn prefer_integrated_gpu() {
    const NVIDIA: &str = "0x10de";
    if [
        "VK_LOADER_DRIVERS_DISABLE",
        "VK_LOADER_DRIVERS_SELECT",
        "VK_DRIVER_FILES",
        "VK_ICD_FILENAMES",
    ]
    .iter()
    .any(|name| std::env::var_os(name).is_some())
    {
        return;
    }
    let Ok(cards) = std::fs::read_dir("/sys/class/drm") else {
        return;
    };
    let vendors: Vec<String> = cards
        .flatten()
        .filter_map(|card| std::fs::read_to_string(card.path().join("device/vendor")).ok())
        .map(|vendor| vendor.trim().to_owned())
        .collect();
    if vendors.iter().any(|vendor| vendor == NVIDIA)
        && vendors.iter().any(|vendor| vendor != NVIDIA)
    {
        // SAFETY: runs first in main, before any other thread exists.
        unsafe { std::env::set_var("VK_LOADER_DRIVERS_DISABLE", "*nvidia*") };
    }
}
