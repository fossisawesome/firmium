use std::fs::{File, OpenOptions};
use std::os::windows::io::AsRawHandle;

#[link(name = "kernel32")]
extern "system" {
    fn SetStdHandle(kind: u32, handle: *mut std::ffi::c_void) -> i32;
}

pub fn init() -> std::io::Result<File> {
    let path = firmium_backend::paths::data_dir().join("firmium.log");
    let file = OpenOptions::new().create(true).append(true).open(path)?;
    if unsafe { SetStdHandle(-12_i32 as u32, file.as_raw_handle()) } == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(file)
}