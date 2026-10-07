//! What a conversion reads, for `indd audit`.
//!
//! While a [`Recorder`] is active on a thread, the code that decodes
//! objects reports which objects, chunks and attributes it reads, and which
//! values it found but could not convert. [`audit`] converts a document
//! with a recorder active and compares the record with everything the
//! document contains. Anything present but never read is what the
//! converter does not understand yet. Without a recorder, reporting does
//! nothing.
//!
//! "Read" means the converter looked the item up, not that it converted
//! every value in it correctly.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};

use crate::{Container, Error, object};

/// Kinds of attribute lists, by where the converter parses them. IDs mean
/// different things in different kinds, so they are counted per kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub enum List {
    /// Not parsed from a document (an empty list).
    #[default]
    None,
    /// A page item's attribute list.
    Item,
    /// An object style's page item attributes.
    ObjectStyle,
    /// An object style's frame fitting attributes.
    ObjectStyleFitting,
    /// A paragraph or character style's text attributes.
    Style,
    /// The text attributes of a run of text.
    Text,
    /// A table's, row's, column's, or table or cell style's attributes.
    Table,
    /// A cell's attributes.
    Cell,
}

impl List {
    pub fn name(self) -> &'static str {
        match self {
            List::None => "none",
            List::Item => "page item",
            List::ObjectStyle => "object style",
            List::ObjectStyleFitting => "object style fitting",
            List::Style => "style",
            List::Text => "text",
            List::Table => "table",
            List::Cell => "cell",
        }
    }
}

#[derive(Debug, Default)]
struct Log {
    objects: HashSet<u32>,
    chunks: HashSet<(u32, u32)>,
    attrs_present: HashMap<(List, u32), usize>,
    attrs_read: HashSet<(List, u32)>,
    codes: HashMap<(List, u32, u32), usize>,
    strand_kinds: HashMap<u32, usize>,
}

thread_local! {
    static LOG: RefCell<Option<Log>> = const { RefCell::new(None) };
}

fn with(f: impl FnOnce(&mut Log)) {
    LOG.with_borrow_mut(|log| {
        if let Some(log) = log {
            f(log);
        }
    });
}

/// Records what the converter reads on this thread until dropped.
pub struct Recorder(());

impl Recorder {
    pub fn start() -> Recorder {
        LOG.set(Some(Log::default()));
        Recorder(())
    }

    fn take(self) -> Log {
        LOG.take().unwrap_or_default()
    }
}

impl Drop for Recorder {
    fn drop(&mut self) {
        LOG.set(None);
    }
}

/// The converter read object `uid`.
pub fn object_read(uid: u32) {
    with(|l| {
        l.objects.insert(uid);
    });
}

/// The converter looked up chunk `id` of object `uid`.
pub fn chunk_read(uid: u32, id: u32) {
    with(|l| {
        l.objects.insert(uid);
        l.chunks.insert((uid, id));
    });
}

/// An attribute list of kind `list` with these attribute IDs was parsed.
pub fn attrs_parsed(list: List, ids: impl Iterator<Item = u32>) {
    with(|l| {
        for id in ids {
            *l.attrs_present.entry((list, id)).or_default() += 1;
        }
    });
}

/// The converter looked up attribute `id` in a list of kind `list` that
/// has it.
pub fn attr_read(list: List, id: u32) {
    with(|l| {
        l.attrs_read.insert((list, id));
    });
}

/// Attribute `id` of a list of kind `list` has a code the converter does
/// not know.
pub fn unknown_code(list: List, id: u32, code: u32) {
    with(|l| *l.codes.entry((list, id, code)).or_default() += 1);
}

/// A strand holds run data of a kind the converter does not read.
pub fn unknown_strand_kind(kind: u32) {
    with(|l| *l.strand_kinds.entry(kind).or_default() += 1);
}

/// Objects of one class.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct ClassCount {
    pub objects: usize,
    /// Objects the converter read.
    pub read: usize,
}

/// What the converter does not understand in one document.
#[derive(Debug, Default)]
pub struct Audit {
    /// Every class: objects with data, and how many were read.
    pub classes: BTreeMap<Option<u32>, ClassCount>,
    /// Objects without chunks (byte streams) that were not read, by class.
    pub unread_streams: BTreeMap<Option<u32>, usize>,
    /// (class, chunk ID): objects that have the chunk, and objects in
    /// which the converter read it.
    pub chunks: BTreeMap<(Option<u32>, u32), (usize, usize)>,
    /// (list kind, attribute ID): occurrences, and whether the converter
    /// looked the attribute up.
    pub attrs: BTreeMap<(List, u32), (usize, bool)>,
    /// (list kind, attribute ID, code): occurrences of unknown codes.
    pub unknown_codes: BTreeMap<(List, u32, u32), usize>,
    /// Strand run data kinds that are not read: occurrences.
    pub unknown_strand_kinds: BTreeMap<u32, usize>,
    /// The conversion's warnings.
    pub warnings: Vec<String>,
    /// The error that stopped the conversion, if any.
    pub error: Option<String>,
}

/// Convert `indd` (output discarded) and report what was not read.
pub fn audit(indd: &[u8], name: &str) -> Result<Audit, Error> {
    let recorder = Recorder::start();
    let result = crate::convert(indd, name, std::io::sink());
    let log = recorder.take();

    let container = Container::parse(indd)?;
    let _order = object::use_byte_order(container.header.byte_order);
    let db = container.database()?;
    let mut out = Audit::default();
    match result {
        Ok(w) => out.warnings = w,
        Err(e) => out.error = Some(e.to_string()),
    }
    for uid in db.uids() {
        let Some(obj) = db.get(uid)? else { continue };
        let class = obj.class;
        let read = log.objects.contains(&uid);
        let c = out.classes.entry(class).or_default();
        c.objects += 1;
        c.read += usize::from(read);
        match obj.chunks() {
            Some(list) if !list.is_empty() => {
                let mut ids: Vec<u32> = list.iter().map(|c| c.id).collect();
                ids.sort_unstable();
                ids.dedup();
                for id in ids {
                    let e = out.chunks.entry((class, id)).or_default();
                    e.0 += 1;
                    e.1 += usize::from(log.chunks.contains(&(uid, id)));
                }
            }
            _ if !read && !obj.bytes.is_empty() => {
                *out.unread_streams.entry(class).or_default() += 1
            }
            _ => {}
        }
    }
    for (&key, &n) in &log.attrs_present {
        out.attrs.insert(key, (n, log.attrs_read.contains(&key)));
    }
    out.unknown_codes = log.codes.into_iter().collect();
    out.unknown_strand_kinds = log.strand_kinds.into_iter().collect();
    Ok(out)
}

impl Audit {
    /// Chunks of classes the converter reads that it read in no object:
    /// (class, chunk ID) and the objects that have the chunk.
    pub fn unread_chunks(&self) -> impl Iterator<Item = ((Option<u32>, u32), usize)> + '_ {
        self.chunks
            .iter()
            .filter(|((class, _), (_, read))| *read == 0 && self.classes[class].read > 0)
            .map(|(&k, &(n, _))| (k, n))
    }

    /// Attributes never looked up: (list kind, ID) and occurrences.
    pub fn unread_attrs(&self) -> impl Iterator<Item = ((List, u32), usize)> + '_ {
        self.attrs
            .iter()
            .filter(|(_, (_, read))| !read)
            .map(|(&k, &(n, _))| (k, n))
    }
}
