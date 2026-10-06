//! Reader for INDD files, the native document format of Adobe® InDesign®.
//!
//! The format is undocumented. Everything this crate knows about it is
//! recorded in `docs/format/`; see `CLEANROOM.md` for how it was learned.

pub mod container;
pub mod database;
pub mod header;
pub mod idml;
pub mod model;
pub mod object;

pub use container::{Container, ContigObject, MasterPage};
pub use database::{Database, Entry};
pub use header::{ByteOrder, Header, Version};
pub use object::{Chunk, Cursor, Object};

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
    /// A contiguous object's trailer does not match its header.
    BadContigObject {
        offset: usize,
    },
    /// The database structures are inconsistent.
    Corrupt(String),
    /// A valid file using a feature this crate does not read yet.
    Unsupported(&'static str),
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
            Error::BadContigObject { offset } => {
                write!(
                    f,
                    "contiguous object at {offset:#x} has no matching trailer"
                )
            }
            Error::Corrupt(msg) => write!(f, "corrupt database: {msg}"),
            Error::Unsupported(what) => write!(f, "not supported yet: {what}"),
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

/// Convert INDD bytes to an IDML package written to `out`. `name` is the
/// document name recorded in the package (normally the INDD file name).
pub fn convert(indd: &[u8], name: &str, out: impl std::io::Write) -> Result<(), Error> {
    let container = Container::parse(indd)?;
    let db = container.database()?;
    let doc = model::Reader::new(&db).document(container.header.version)?;
    idml::write(&doc, name, out)?;
    Ok(())
}
