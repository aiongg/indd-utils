//! Reader for INDD files, the native document format of Adobe® InDesign®.
//!
//! The format is undocumented. Everything this crate knows about it is
//! recorded in `docs/format/`; see `CLEANROOM.md` for how it was learned.

pub mod header;

pub use header::{ByteOrder, Header, Version};

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    /// The file does not start with the INDD signature.
    NotIndd,
    Truncated {
        needed: usize,
        got: usize,
    },
    UnknownByteOrder(u8),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(e) => write!(f, "{e}"),
            Error::NotIndd => write!(f, "not an INDD file"),
            Error::Truncated { needed, got } => {
                write!(f, "file too short: need {needed} bytes, got {got}")
            }
            Error::UnknownByteOrder(b) => write!(f, "unknown byte-order flag {b:#04x}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

/// Read only the header of the file at `path`.
pub fn read_header(path: impl AsRef<std::path::Path>) -> Result<Header, Error> {
    use std::io::Read;
    let mut buf = [0u8; header::HEADER_LEN];
    let mut f = std::fs::File::open(path)?;
    let mut got = 0;
    while got < buf.len() {
        match f.read(&mut buf[got..])? {
            0 => break,
            n => got += n,
        }
    }
    Header::parse(&buf[..got])
}
