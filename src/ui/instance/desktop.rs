#[cfg(target_os = "windows")]
use interprocess::local_socket::{
    GenericNamespaced, Listener as UnixListener, ListenerOptions, Stream as UnixStream, prelude::*,
};
#[cfg(target_os = "macos")]
use std::io::ErrorKind;
#[cfg(target_os = "macos")]
use std::os::unix::net::{UnixListener, UnixStream};

use std::io::Write;

/// Becomes the running instance, or asks the one already running to show its window.
pub enum Claim {
    Primary(Option<UnixListener>),
    Handed,
}

pub fn claim(name: &str, show: bool) -> Claim {
    #[cfg(target_os = "macos")]
    let path = {
        use std::os::unix::fs::DirBuilderExt;
        let Some(dirs) = directories::ProjectDirs::from("", "", "hermes") else {
            return Claim::Primary(None);
        };
        if std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(dirs.data_local_dir())
            .is_err()
        {
            return Claim::Primary(None);
        }
        dirs.data_local_dir().join(name)
    };
    #[cfg(target_os = "windows")]
    // Pipe names are machine-wide, so scope them to the user.
    let path = format!(
        "hermes-{}-{name}",
        std::env::var("USERNAME").unwrap_or_default()
    );
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
                let _ = stream.write_all(b"open");
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

pub fn serve(listener: UnixListener, open: async_channel::Sender<crate::ui::tray::Action>) {
    let _ = std::thread::Builder::new()
        .name("hermes-instance".into())
        .spawn(move || {
            for mut stream in listener.incoming().flatten() {
                let mut request = [0; 4];
                if std::io::Read::read_exact(&mut stream, &mut request).is_ok()
                    && open.try_send(crate::ui::tray::Action::Open).is_err()
                {
                    return;
                }
            }
        });
}
