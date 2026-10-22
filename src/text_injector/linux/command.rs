use std::{
    io::{self, Read, Write},
    os::{fd::AsRawFd, unix::process::CommandExt},
    process::{Command, Output, Stdio},
    time::{Duration, Instant},
};

fn nonblocking(pipe: &impl AsRawFd) -> io::Result<()> {
    let fd = pipe.as_raw_fd();
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags == -1 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } == -1 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

pub(super) fn run(
    command: &mut Command,
    input: Option<&[u8]>,
    timeout: Duration,
    max_output: usize,
) -> io::Result<Output> {
    let deadline = Instant::now() + timeout;
    let mut child = command
        .process_group(0)
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(if max_output == 0 {
            Stdio::null()
        } else {
            Stdio::piped()
        })
        .stderr(Stdio::null())
        .spawn()?;
    let result = (|| {
        let mut stdin = child.stdin.take();
        let mut stdout = child.stdout.take();
        if let Some(stdout) = &stdout {
            nonblocking(stdout)?;
        }
        if let Some(stdin) = &stdin {
            nonblocking(stdin)?;
        }
        let mut remaining = input.unwrap_or_default();
        let mut bytes = Vec::new();
        let mut eof = stdout.is_none();
        let mut status = None;
        loop {
            if Instant::now() >= deadline {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "command deadline exceeded",
                ));
            }
            if remaining.is_empty() {
                stdin.take();
            }
            if let Some(stdin) = &mut stdin {
                match stdin.write(remaining) {
                    Ok(0) => return Err(io::ErrorKind::WriteZero.into()),
                    Ok(count) => remaining = &remaining[count..],
                    Err(error)
                        if matches!(
                            error.kind(),
                            io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                        ) => {}
                    Err(error) => return Err(error),
                }
            }
            // Bound work per iteration so a continuously writing child cannot evade the deadline.
            for _ in 0..64 {
                let Some(stdout) = &mut stdout else { break };
                let mut buffer = [0; 8192];
                match stdout.read(&mut buffer) {
                    Ok(0) => {
                        eof = true;
                        break;
                    }
                    Ok(count) => {
                        if count > max_output.saturating_sub(bytes.len()) {
                            return Err(io::Error::new(
                                io::ErrorKind::FileTooLarge,
                                "command output exceeds limit",
                            ));
                        }
                        bytes.extend_from_slice(&buffer[..count]);
                    }
                    Err(error)
                        if matches!(
                            error.kind(),
                            io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                        ) =>
                    {
                        break;
                    }
                    Err(error) => return Err(error),
                }
            }
            if status.is_none() {
                status = child.try_wait()?;
            }
            if let Some(status) = status.filter(|_| eof && stdin.is_none()) {
                return Ok(Output {
                    status,
                    stdout: bytes,
                    stderr: Vec::new(),
                });
            }
            let mut pipes = [
                libc::pollfd {
                    fd: if eof {
                        -1
                    } else {
                        stdout.as_ref().map_or(-1, AsRawFd::as_raw_fd)
                    },
                    events: libc::POLLIN,
                    revents: 0,
                },
                libc::pollfd {
                    fd: stdin.as_ref().map_or(-1, AsRawFd::as_raw_fd),
                    events: libc::POLLOUT,
                    revents: 0,
                },
            ];
            unsafe {
                libc::poll(pipes.as_mut_ptr(), pipes.len() as libc::nfds_t, 2);
            }
        }
    })();
    if result.is_err() {
        // wl-copy can fork before publishing; kill the whole group before returning failure.
        unsafe {
            libc::kill(-(child.id() as i32), libc::SIGKILL);
        }
        let _ = child.kill();
        let _ = child.wait();
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transfers_input_and_bounds_output() {
        let input = vec![b'x'; 256 * 1024];
        let output = run(
            &mut Command::new("cat"),
            Some(&input),
            Duration::from_secs(2),
            input.len(),
        )
        .unwrap();
        assert!(output.status.success());
        assert_eq!(output.stdout, input);
        assert_eq!(
            run(
                Command::new("printf").arg("12345"),
                None,
                Duration::from_secs(1),
                4
            )
            .unwrap_err()
            .kind(),
            io::ErrorKind::FileTooLarge
        );
    }

    #[test]
    fn kills_hung_commands_and_descendants_even_when_stdin_blocks() {
        assert_timeout_kills_group("sleep 10 & echo \"$$ $!\" > \"$1\"; wait", true);
    }

    #[test]
    fn kills_descendants_after_the_parent_exits() {
        assert_timeout_kills_group("sleep 10 & echo \"$$ $!\" > \"$1\"", false);
    }

    fn assert_timeout_kills_group(script: &str, blocked_stdin: bool) {
        let path = std::env::temp_dir().join(format!(
            "typeaside-command-test-{}-{}",
            std::process::id(),
            if blocked_stdin { "stdin" } else { "stdout" }
        ));
        let input = vec![0; 1024 * 1024];
        let started = Instant::now();
        let error = run(
            Command::new("sh").args(["-c", script, "sh"]).arg(&path),
            blocked_stdin.then_some(input.as_slice()),
            Duration::from_millis(150),
            1024,
        )
        .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::TimedOut);
        assert!(started.elapsed() < Duration::from_secs(2));
        let pids = std::fs::read_to_string(&path).unwrap();
        std::fs::remove_file(path).unwrap();
        for pid in pids.split_whitespace() {
            let stopped = || {
                std::fs::read_to_string(format!("/proc/{pid}/status")).map_or(true, |status| {
                    status
                        .lines()
                        .any(|line| line.starts_with("State:") && line.contains("Z (zombie)"))
                })
            };
            let deadline = Instant::now() + Duration::from_millis(200);
            while !stopped() && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(2));
            }
            assert!(stopped(), "child {pid} survived the command deadline");
        }
    }
}
