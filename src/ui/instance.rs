#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::*;
#[cfg(not(target_os = "linux"))]
mod desktop;
#[cfg(not(target_os = "linux"))]
pub use desktop::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Open,
    Start,
    Stop,
    Toggle,
    Cancel,
    PasteLast,
}

impl Command {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Start => "dictation start",
            Self::Stop => "dictation stop",
            Self::Toggle => "dictation toggle",
            Self::Cancel => "dictation cancel",
            Self::PasteLast => "paste-last",
        }
    }

    pub fn engine(self) -> Option<typeaside::engine::Command> {
        use typeaside::engine::Command as Engine;
        match self {
            Self::Open => None,
            Self::Start => Some(Engine::Start),
            Self::Stop => Some(Engine::Stop),
            Self::Toggle => Some(Engine::Toggle),
            Self::Cancel => Some(Engine::Cancel),
            Self::PasteLast => Some(Engine::PasteLast),
        }
    }
}

impl std::str::FromStr for Command {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "open" => Ok(Self::Open),
            "dictation start" => Ok(Self::Start),
            "dictation stop" => Ok(Self::Stop),
            "dictation toggle" => Ok(Self::Toggle),
            "dictation cancel" => Ok(Self::Cancel),
            "paste-last" => Ok(Self::PasteLast),
            _ => Err("unknown command"),
        }
    }
}

fn send_command(
    stream: &mut (impl std::io::Read + std::io::Write),
    command: Command,
) -> std::io::Result<()> {
    use std::io::BufRead;
    writeln!(stream, "{}", command.as_str())?;
    stream.flush()?;
    let mut response = String::new();
    std::io::BufReader::new(std::io::Read::take(stream, 64)).read_line(&mut response)?;
    if response == "ok\n" {
        Ok(())
    } else {
        Err(std::io::Error::other(format!(
            "running instance rejected command: {}",
            response.trim()
        )))
    }
}

fn receive_command(
    stream: &mut (impl std::io::Read + std::io::Write),
    actions: &async_channel::Sender<crate::ui::tray::Action>,
) {
    use std::io::BufRead;
    let mut request = String::new();
    let command = std::io::BufReader::new(std::io::Read::take(&mut *stream, 64))
        .read_line(&mut request)
        .ok()
        .and_then(|_| request.strip_suffix('\n'))
        .and_then(|line| line.parse::<Command>().ok());
    let response = match command {
        Some(command) => {
            if std::env::var("TYPEASIDE_TRACE").as_deref() == Ok("1") {
                eprintln!("typeaside instance: {}", command.as_str());
            }
            let action = match command {
                Command::Open => crate::ui::tray::Action::Open,
                _ => crate::ui::tray::Action::Command(command),
            };
            if actions.try_send(action).is_ok() {
                "ok"
            } else {
                "error: instance is shutting down"
            }
        }
        None => "error: unknown command",
    };
    let _ = writeln!(stream, "{response}");
}

#[cfg(test)]
mod tests {
    use super::Command;

    #[test]
    fn commands_round_trip() {
        for command in [
            Command::Open,
            Command::Start,
            Command::Stop,
            Command::Toggle,
            Command::Cancel,
            Command::PasteLast,
        ] {
            assert_eq!(command.as_str().parse(), Ok(command));
        }
    }

    #[test]
    fn rejects_unknown_or_malformed_commands() {
        for request in [
            "",
            "xxxx",
            "start",
            "open extra",
            "open\npaste-last",
            "OPEN",
        ] {
            assert!(request.parse::<Command>().is_err(), "{request:?}");
        }
    }
}
