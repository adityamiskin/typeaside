use std::{
    io::ErrorKind,
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
                let _ = super::send_command(&mut stream, super::Command::Open);
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

pub fn send(name: &str, command: super::Command) -> std::io::Result<()> {
    let dir = std::env::var_os("XDG_RUNTIME_DIR")
        .ok_or_else(|| std::io::Error::other("XDG_RUNTIME_DIR is not set"))?;
    let mut stream = UnixStream::connect(PathBuf::from(dir).join(name))?;
    stream.set_read_timeout(Some(std::time::Duration::from_secs(2)))?;
    stream.set_write_timeout(Some(std::time::Duration::from_secs(2)))?;
    super::send_command(&mut stream, command)
}

pub fn serve(listener: UnixListener, actions: async_channel::Sender<crate::ui::tray::Action>) {
    let _ = std::thread::Builder::new()
        .name("typeaside-instance".into())
        .spawn(move || {
            for mut stream in listener.incoming().flatten() {
                let timeout = Some(std::time::Duration::from_secs(2));
                let _ = stream.set_read_timeout(timeout);
                let _ = stream.set_write_timeout(timeout);
                super::receive_command(&mut stream, &actions);
            }
        });
}
