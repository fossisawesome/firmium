/// Sends one command to a running instance and prints its reply.
pub fn run_cli(cmd: &str) -> i32 {
    match cli_send(cmd) {
        Ok(reply) => {
            println!("{reply}");
            if reply.starts_with("error") { 1 } else { 0 }
        }
        Err(error) => {
            eprintln!("firmium: {error} (is the app running?)");
            1
        }
    }
}

#[cfg(unix)]
fn cli_send(cmd: &str) -> std::io::Result<String> {
    use std::io::{BufRead, BufReader, Write};
    let mut stream = std::os::unix::net::UnixStream::connect(firmium_backend::ipc::socket_path())?;
    writeln!(stream, "{cmd}")?;
    let mut reply = String::new();
    BufReader::new(stream).read_line(&mut reply)?;
    Ok(reply.trim_end().to_string())
}

#[cfg(windows)]
fn cli_send(cmd: &str) -> std::io::Result<String> {
    use std::io::{BufRead, BufReader, Write};
    let mut pipe = std::fs::OpenOptions::new().read(true).write(true).open(firmium_backend::ipc::PIPE_NAME)?;
    writeln!(pipe, "{cmd}")?;
    let mut reply = String::new();
    BufReader::new(pipe).read_line(&mut reply)?;
    Ok(reply.trim_end().to_string())
}