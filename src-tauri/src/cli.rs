//! One-shot CLI client for the backend's control socket (see `backend/ipc.rs`).

/// Sends one line to the running instance and prints the reply. Returns the
/// process exit code.
pub fn run(cmd: &str) -> i32 {
    match send(cmd) {
        Ok(reply) => {
            println!("{reply}");
            i32::from(reply.starts_with("error"))
        }
        Err(e) => {
            eprintln!("firmium: {e} (is the app running?)");
            1
        }
    }
}

#[cfg(unix)]
fn send(cmd: &str) -> std::io::Result<String> {
    use std::io::{BufRead, BufReader, Write};
    let mut stream = std::os::unix::net::UnixStream::connect(firmium_backend::ipc::socket_path())?;
    writeln!(stream, "{cmd}")?;
    let mut reply = String::new();
    BufReader::new(stream).read_line(&mut reply)?;
    Ok(reply.trim_end().to_string())
}

#[cfg(windows)]
fn send(cmd: &str) -> std::io::Result<String> {
    use std::io::{BufRead, BufReader, Write};
    let mut pipe = std::fs::OpenOptions::new().read(true).write(true).open(firmium_backend::ipc::PIPE_NAME)?;
    writeln!(pipe, "{cmd}")?;
    let mut reply = String::new();
    BufReader::new(pipe).read_line(&mut reply)?;
    Ok(reply.trim_end().to_string())
}
