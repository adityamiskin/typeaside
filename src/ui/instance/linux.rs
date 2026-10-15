use std::{
    io::{ErrorKind, Write},
    os::unix::net::{UnixListener, UnixStream},
    path::PathBuf,
};

/// Becomes the running instance, or asks the one already running to show its window.
pub enum Claim {
    Primary(Option<UnixListener>),
    Handed,
}

pub fn claim(name: &str, show: bool) -> Claim {
    let Some(path) = std::env::var_os("XDG_RUNTIME_DIR").map(|dir| PathBuf::from(dir).join(name))
    else {
        return Claim::Primary(None);
    };
    match UnixStream::connect(&path) {
        Ok(mut stream) => {
            if show {
                let _ = stream.write_all(b"open");
            }
            Claim::Handed
        }
        Err(error) => {
            if matches!(error.kind(), ErrorKind::ConnectionRefused) {
                let _ = std::fs::remove_file(&path);
            }
            Claim::Primary(UnixListener::bind(&path).ok())
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
