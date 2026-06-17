//! A Read+Seek wrapper over a streaming HTTP response body, used as a
//! `symphonia::core::io::MediaSource` for decoding while keeping the HTTP
//! connection open for the duration of playback.
//!
//! Bytes are read from the response on demand and buffered locally.
//! This keeps the HTTP connection open for as long as audio is playing,
//! which allows Navidrome (and other OpenSubsonic servers) to maintain the
//! "Now Playing" status for the full track duration rather than just the
//! brief moment the file is being downloaded.
//!
//! Backward seeks are supported via the in-memory buffer. Forward seeks
//! past the buffered position drain bytes from the live HTTP connection.

use parking_lot::Mutex;
use std::io::{self, Cursor, Read, Seek, SeekFrom};
use std::sync::Arc;
use symphonia::core::io::MediaSource;

pub struct StreamingReader {
    response: reqwest::blocking::Response,
    /// Shared buffer so the seek fallback can rebuild the decoder from buffered bytes.
    buffer: Arc<Mutex<Vec<u8>>>,
    pos: usize,
    /// Total stream size from the `Content-Length` header, if present. Lets
    /// symphonia's bisection-based seek (used by OGG/MP4/etc.) compute byte
    /// offsets from timestamps for forward seeks instead of failing with EOF.
    total_len: Option<u64>,
}

impl StreamingReader {
    pub fn new(response: reqwest::blocking::Response) -> (Self, Arc<Mutex<Vec<u8>>>) {
        let buffer = Arc::new(Mutex::new(Vec::new()));
        let total_len = response.content_length();
        let reader = Self { response, buffer: Arc::clone(&buffer), pos: 0, total_len };
        (reader, buffer)
    }

    pub fn fill_to(&mut self, target: usize) -> io::Result<()> {
        {
            let buf = self.buffer.lock();
            if target <= buf.len() {
                return Ok(());
            }
        }
        // Read from the network without holding the lock to avoid blocking other readers.
        let needed = target - self.buffer.lock().len();
        let mut tmp = vec![0u8; needed];
        let mut filled = 0;
        while filled < needed {
            let n = self.response.read(&mut tmp[filled..])?;
            if n == 0 { break; }
            filled += n;
        }
        self.buffer.lock().extend_from_slice(&tmp[..filled]);
        Ok(())
    }
}

impl Read for StreamingReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let buffered = self.buffer.lock();
        if self.pos < buffered.len() {
            let n = buf.len().min(buffered.len() - self.pos);
            buf[..n].copy_from_slice(&buffered[self.pos..self.pos + n]);
            drop(buffered);
            self.pos += n;
            return Ok(n);
        }
        drop(buffered);
        // Read new bytes from the HTTP connection and buffer them.
        let n = self.response.read(buf)?;
        self.buffer.lock().extend_from_slice(&buf[..n]);
        self.pos += n;
        Ok(n)
    }
}

impl Seek for StreamingReader {
    fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
        let new_pos = match from {
            SeekFrom::Start(n) => n as usize,
            SeekFrom::Current(n) => (self.pos as i64).saturating_add(n).max(0) as usize,
            SeekFrom::End(n) => {
                let mut rest = Vec::new();
                self.response.read_to_end(&mut rest)?;
                let mut buf = self.buffer.lock();
                buf.extend_from_slice(&rest);
                (buf.len() as i64).saturating_add(n).max(0) as usize
            }
        };

        {
            let buf = self.buffer.lock();
            if new_pos > buf.len() {
                drop(buf);
                self.fill_to(new_pos)?;
            }
        }

        let buf = self.buffer.lock();
        self.pos = new_pos.min(buf.len());
        Ok(self.pos as u64)
    }
}

// Safety: reqwest's blocking Response is Send (it wraps a sync I/O handle).
unsafe impl Send for StreamingReader {}
unsafe impl Sync for StreamingReader {}

impl MediaSource for StreamingReader {
    fn is_seekable(&self) -> bool {
        // Backed by an in-memory buffer that grows on demand — backward seeks
        // use the buffer directly, forward seeks fetch from the network.
        true
    }

    fn byte_len(&self) -> Option<u64> {
        self.total_len
    }
}

/// A fully-buffered, seekable in-memory source used to rebuild a decoder from
/// previously-buffered bytes (seek fallback for forward-only formats).
pub struct VecSource(Cursor<Vec<u8>>);

impl VecSource {
    pub fn new(data: Vec<u8>) -> Self {
        Self(Cursor::new(data))
    }
}

impl Read for VecSource {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }
}

impl Seek for VecSource {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.0.seek(pos)
    }
}

impl MediaSource for VecSource {
    fn is_seekable(&self) -> bool {
        true
    }

    fn byte_len(&self) -> Option<u64> {
        Some(self.0.get_ref().len() as u64)
    }
}

/// A seekable wrapper over `BufReader<File>` for local-file playback.
pub struct FileSource {
    reader: std::io::BufReader<std::fs::File>,
    len: Option<u64>,
}

impl FileSource {
    pub fn new(reader: std::io::BufReader<std::fs::File>, len: Option<u64>) -> Self {
        Self { reader, len }
    }
}

impl Read for FileSource {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.reader.read(buf)
    }
}

impl Seek for FileSource {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.reader.seek(pos)
    }
}

impl MediaSource for FileSource {
    fn is_seekable(&self) -> bool {
        true
    }

    fn byte_len(&self) -> Option<u64> {
        self.len
    }
}
