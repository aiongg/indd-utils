//! The object database stored in the database pages. See
//! `docs/format/database.md`.
//!
//! Every object has a UID and a byte stream. The streams are found through
//! a B+ tree keyed by (UID, segment). Little-endian files only.

use std::collections::BTreeMap;

use crate::container::PAGE_SIZE;
use crate::{ByteOrder, Container, Error};

/// Page types, from the u32 at 0xFF4 of every database page.
pub mod page_type {
    pub const ALLOC_DIR: u32 = 2;
    pub const ALLOC_BITMAP: u32 = 3;
    pub const LOGICAL_DIR: u32 = 4;
    pub const LOGICAL_TABLE: u32 = 5;
    pub const TREE_LEAF: u32 = 6;
    pub const TREE_INTERIOR: u32 = 7;
    pub const DATA: u32 = 8;
    pub const SLOTTED: u32 = 9;
}

const TRAILER_TYPE: usize = 0xFF4;
const TABLE_START: usize = 0x80;
const TABLE_ENTRIES: usize = 989;
const SLOT_FOOTER: usize = 0xFDC;

// Fields of the active master page.
const MASTER_LOGICAL_DIR: usize = 0x3A8;
/// Three u32 tree roots (logical pages): objects, classes, unclassed UIDs.
const MASTER_TREE_ROOTS: usize = 0xB7C;
/// Three u64 entry counts, in the same order as the roots.
const MASTER_TREE_COUNTS: usize = 0xB88;

/// Largest segment stored on a data page.
pub const SEGMENT_MAX: u32 = 0xF70;

const SLOT_CONTINUES: u16 = 0x8000;

/// One leaf entry of the object tree: part `segment` (1-based) of object `uid`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry {
    pub segment: u32,
    pub uid: u32,
    /// Either a byte count (segment on a data page), or `slot << 16 | count`
    /// (segment in a slotted page).
    pub length: u32,
    /// Physical page for data-page segments, logical page for slotted ones.
    pub page: u32,
}

impl Entry {
    pub fn is_slotted(&self) -> bool {
        self.length >> 16 != 0
    }

    pub fn len(&self) -> usize {
        if self.is_slotted() {
            (self.length & 0xFFFF) as usize
        } else {
            self.length as usize
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

pub struct Database<'a> {
    bytes: &'a [u8],
    logical: BTreeMap<u32, u32>,
    entries: Vec<Entry>,
    /// (UID, class ID), sorted by UID.
    classes: Vec<(u32, u32)>,
    /// UIDs listed in the third tree. They have no class entry.
    unclassed: Vec<u32>,
}

fn corrupt(msg: impl Into<String>) -> Error {
    Error::Corrupt(msg.into())
}

impl<'a> Database<'a> {
    pub fn open(container: &Container<'a>) -> Result<Database<'a>, Error> {
        if container.header.byte_order != ByteOrder::Little {
            return Err(Error::Unsupported("big-endian database"));
        }
        let master = container.active * PAGE_SIZE;
        let mut db = Database {
            bytes: container.bytes,
            logical: BTreeMap::new(),
            entries: Vec::new(),
            classes: Vec::new(),
            unclassed: Vec::new(),
        };
        db.read_logical_table(db.u32(master + MASTER_LOGICAL_DIR)?)?;
        let mut trees: [Vec<Entry>; 3] = Default::default();
        for (i, tree) in trees.iter_mut().enumerate() {
            let root = db.u32(master + MASTER_TREE_ROOTS + 4 * i)?;
            let expected = db.u32(master + MASTER_TREE_COUNTS + 8 * i)? as usize;
            db.walk(root, 0, tree)?;
            if tree.len() != expected {
                return Err(corrupt(format!(
                    "tree {i} has {} entries, master page says {expected}",
                    tree.len()
                )));
            }
        }
        let [entries, classes, unclassed] = trees;
        db.entries = entries;
        db.classes = classes.iter().map(|e| (e.uid, e.length)).collect();
        db.unclassed = unclassed.iter().map(|e| e.uid).collect();
        Ok(db)
    }

    fn u16(&self, offset: usize) -> Result<u16, Error> {
        self.bytes
            .get(offset..offset + 2)
            .map(|b| u16::from_le_bytes(b.try_into().unwrap()))
            .ok_or_else(|| corrupt(format!("read past end of file at {offset:#x}")))
    }

    fn u32(&self, offset: usize) -> Result<u32, Error> {
        self.bytes
            .get(offset..offset + 4)
            .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
            .ok_or_else(|| corrupt(format!("read past end of file at {offset:#x}")))
    }

    pub fn page_type(&self, page: u32) -> Result<u32, Error> {
        self.u32(page as usize * PAGE_SIZE + TRAILER_TYPE)
    }

    fn expect_type(&self, page: u32, expected: u32) -> Result<usize, Error> {
        let actual = self.page_type(page)?;
        if actual != expected {
            return Err(corrupt(format!(
                "page {page} has type {actual}, expected {expected}"
            )));
        }
        Ok(page as usize * PAGE_SIZE)
    }

    fn read_logical_table(&mut self, dir_page: u32) -> Result<(), Error> {
        let dir = self.expect_type(dir_page, page_type::LOGICAL_DIR)?;
        for j in 0..TABLE_ENTRIES {
            let table_page = self.u32(dir + TABLE_START + 4 * j)?;
            if table_page == 0 {
                continue;
            }
            let table = self.expect_type(table_page, page_type::LOGICAL_TABLE)?;
            for i in 0..TABLE_ENTRIES {
                let physical = self.u32(table + TABLE_START + 4 * i)?;
                if physical != 0 {
                    self.logical
                        .insert((j * TABLE_ENTRIES + i) as u32, physical);
                }
            }
        }
        Ok(())
    }

    /// Physical page for a logical page number.
    pub fn physical(&self, logical: u32) -> Result<u32, Error> {
        self.logical
            .get(&logical)
            .copied()
            .ok_or_else(|| corrupt(format!("logical page {logical} is not mapped")))
    }

    fn walk(&self, logical: u32, depth: usize, out: &mut Vec<Entry>) -> Result<(), Error> {
        if depth > 32 {
            return Err(corrupt("object tree is too deep"));
        }
        let page = self.physical(logical)?;
        let base = page as usize * PAGE_SIZE;
        let count = self.u32(base)? as usize;
        match self.page_type(page)? {
            page_type::TREE_LEAF => {
                for i in 0..count {
                    let o = base + 4 + 16 * i;
                    out.push(Entry {
                        segment: self.u32(o)?,
                        uid: self.u32(o + 4)?,
                        length: self.u32(o + 8)?,
                        page: self.u32(o + 12)?,
                    });
                }
            }
            page_type::TREE_INTERIOR => {
                // child, then (segment, uid) key and child, repeated.
                let mut children = vec![self.u32(base + 4)?];
                for i in 1..count {
                    children.push(self.u32(base + 4 + 12 * i)?);
                }
                for child in children {
                    self.walk(child, depth + 1, out)?;
                }
            }
            other => {
                return Err(corrupt(format!("tree page {page} has type {other}")));
            }
        }
        Ok(())
    }

    /// Leaf entries in (UID, segment) order.
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// Class ID of object `uid`.
    pub fn class_of(&self, uid: u32) -> Option<u32> {
        self.classes
            .binary_search_by_key(&uid, |&(u, _)| u)
            .ok()
            .map(|i| self.classes[i].1)
    }

    /// All (UID, class ID) pairs, sorted by UID.
    pub fn classes(&self) -> &[(u32, u32)] {
        &self.classes
    }

    /// UIDs that have no class (listed in the third tree).
    pub fn unclassed(&self) -> &[u32] {
        &self.unclassed
    }

    fn read_slotted(
        &self,
        logical: u32,
        slot: u16,
        len: usize,
        out: &mut Vec<u8>,
    ) -> Result<(), Error> {
        let base = self.expect_type(self.physical(logical)?, page_type::SLOTTED)?;
        let offset = base + self.u32(base + SLOT_FOOTER - 4 * slot as usize)? as usize;
        let record_len = self.u16(offset)? as usize;
        let id = self.u16(offset + 2)?;
        if id & !SLOT_CONTINUES != slot {
            return Err(corrupt(format!(
                "slot {slot} of logical page {logical} holds record {id:#x}"
            )));
        }
        let record = self
            .bytes
            .get(offset + 4..offset + record_len)
            .ok_or_else(|| corrupt(format!("record at {offset:#x} runs past end of file")))?;
        if id & SLOT_CONTINUES == 0 {
            let data = record
                .get(..len)
                .ok_or_else(|| corrupt(format!("record at {offset:#x} is shorter than {len}")))?;
            out.extend_from_slice(data);
            return Ok(());
        }
        // A continued record starts with a pointer to the rest:
        // `slot << 16` and a logical page, as in a leaf entry.
        if record.len() < 8 {
            return Err(corrupt(format!(
                "continued record at {offset:#x} is too short"
            )));
        }
        let next = u32::from_le_bytes(record[..4].try_into().unwrap());
        let next_page = u32::from_le_bytes(record[4..8].try_into().unwrap());
        let here = &record[8..];
        if here.len() >= len || next & 0xFFFF != 0 {
            return Err(corrupt(format!("bad continuation at {offset:#x}")));
        }
        out.extend_from_slice(here);
        self.read_slotted(next_page, (next >> 16) as u16, len - here.len(), out)
    }

    fn read_entry(&self, e: &Entry, out: &mut Vec<u8>) -> Result<(), Error> {
        if e.is_slotted() {
            return self.read_slotted(e.page, (e.length >> 16) as u16, e.len(), out);
        }
        if e.length > SEGMENT_MAX {
            return Err(corrupt(format!(
                "segment of {} bytes on a data page",
                e.length
            )));
        }
        let base = self.expect_type(e.page, page_type::DATA)?;
        out.extend_from_slice(&self.bytes[base..base + e.len()]);
        Ok(())
    }

    /// The UIDs of all objects, in ascending order.
    pub fn uids(&self) -> impl Iterator<Item = u32> + '_ {
        self.entries
            .iter()
            .enumerate()
            .filter(|(i, e)| *i == 0 || self.entries[i - 1].uid != e.uid)
            .map(|(_, e)| e.uid)
    }

    /// Object `uid` with its class, or `None` if it has no data.
    pub fn get(&self, uid: u32) -> Result<Option<crate::Object>, Error> {
        Ok(self.object(uid)?.map(|bytes| crate::Object {
            uid,
            class: self.class_of(uid),
            bytes,
        }))
    }

    /// The complete byte stream of object `uid`, or `None` if it doesn't exist.
    pub fn object(&self, uid: u32) -> Result<Option<Vec<u8>>, Error> {
        let start = self.entries.partition_point(|e| e.uid < uid);
        let parts = self.entries[start..].iter().take_while(|e| e.uid == uid);
        let mut out = Vec::new();
        let mut found = false;
        for (i, e) in parts.enumerate() {
            if e.segment as usize != i + 1 {
                return Err(corrupt(format!(
                    "object {uid} is missing segment {}",
                    i + 1
                )));
            }
            self.read_entry(e, &mut out)?;
            found = true;
        }
        Ok(found.then_some(out))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_lengths() {
        let data = Entry {
            segment: 1,
            uid: 5,
            length: 0xF70,
            page: 214,
        };
        assert!(!data.is_slotted());
        assert_eq!(data.len(), 0xF70);
        let slotted = Entry {
            segment: 1,
            uid: 2,
            length: 0x0002_0010,
            page: 4,
        };
        assert!(slotted.is_slotted());
        assert_eq!(slotted.len(), 0x10);
    }
}
