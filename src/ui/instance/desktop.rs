#[cfg(target_os = "windows")]
use interprocess::local_socket::{
    GenericNamespaced, Listener as UnixListener, ListenerOptions, Stream as UnixStream, prelude::*,
};
#[cfg(target_os = "macos")]
use std::io::ErrorKind;
#[cfg(target_os = "macos")]
use std::os::unix::net::{UnixListener, UnixStream};

/// Becomes the running instance, or asks the one already running to show its window.
pub enum Claim {
    Primary(Option<UnixListener>),
    Handed,
}

pub fn claim(name: &str, show: bool) -> Claim {
    let Ok(path) = address(name) else {
        return Claim::Primary(None);
    };
    #[cfg(target_os = "macos")]
    let connected = UnixStream::connect(&path);
    #[cfg(target_os = "windows")]
    let connected = path
        .as_str()
        .to_ns_name::<GenericNamespaced>()
        .and_then(UnixStream::connect);
    match connected {
        Ok(mut stream) => {
            if show {
                let _ = super::send_command(&mut stream, super::Command::Open);
            }
            Claim::Handed
        }
        Err(error) => {
            #[cfg(target_os = "macos")]
            if matches!(error.kind(), ErrorKind::ConnectionRefused) {
                let _ = std::fs::remove_file(&path);
            }
            #[cfg(target_os = "windows")]
            let _ = error;
            #[cfg(target_os = "macos")]
            let listener = UnixListener::bind(&path);
            #[cfg(target_os = "windows")]
            let listener = path
                .to_ns_name::<GenericNamespaced>()
                .and_then(|name| ListenerOptions::new().name(name).create_sync());
            Claim::Primary(listener.ok())
        }
    }
}

#[cfg(target_os = "macos")]
fn address(name: &str) -> std::io::Result<std::path::PathBuf> {
    use std::os::unix::fs::DirBuilderExt;
    let paths = typeaside::paths::AppPaths::discover().map_err(std::io::Error::other)?;
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&paths.data_dir)?;
    Ok(paths.data_dir.join(name))
}

#[cfg(target_os = "windows")]
fn address(name: &str) -> std::io::Result<String> {
    Ok(format!(
        "typeaside-{}-{name}",
        std::env::var("USERNAME").unwrap_or_default()
    ))
}

pub fn send(name: &str, command: super::Command) -> std::io::Result<()> {
    let path = address(name)?;
    #[cfg(target_os = "macos")]
    let mut stream = UnixStream::connect(path)?;
    #[cfg(target_os = "windows")]
    let mut stream = UnixStream::connect(path.to_ns_name::<GenericNamespaced>()?)?;
    #[cfg(target_os = "macos")]
    {
        stream.set_read_timeout(Some(std::time::Duration::from_secs(2)))?;
        stream.set_write_timeout(Some(std::time::Duration::from_secs(2)))?;
    }
    super::send_command(&mut stream, command)
}

pub fn serve(listener: UnixListener, actions: async_channel::Sender<crate::ui::tray::Action>) {
    let _ = std::thread::Builder::new()
        .name("typeaside-instance".into())
        .spawn(move || {
            for mut stream in listener.incoming().flatten() {
                #[cfg(target_os = "macos")]
                {
                    let timeout = Some(std::time::Duration::from_secs(2));
                    let _ = stream.set_read_timeout(timeout);
                    let _ = stream.set_write_timeout(timeout);
                }
                super::receive_command(&mut stream, &actions);
            }
        });
}
