//! Document-level settings and lists: document preferences, bullets,
//! colour group order, index groups, constant shade, named grids, document
//! users and languages.
//!
//! Evidence: `docs/format/objects.md` and `preferences.md`.

use super::*;

/// A numbering list (class 0x1A483).
#[derive(Debug, Clone, PartialEq)]
pub struct NumberingList {
    pub uid: u32,
    /// The name as IDML writes it.
    pub name: String,
    pub across_stories: bool,
    pub across_documents: bool,
}

/// A pasted smooth shade with a constant shading. See
/// `docs/format/objects.md`, pasted smooth shades.
#[derive(Debug, Clone, PartialEq)]
pub struct ConstantShade {
    pub uid: u32,
    pub count: u32,
    pub values: [f64; 3],
    /// Flag (built-in key) and name.
    pub name: Option<(bool, String)>,
}

/// Document setup, from chunk 0x533 of the preferences object.
#[derive(Debug, Clone, PartialEq)]
pub struct DocumentPreferences {
    pub page_width: f64,
    pub page_height: f64,
    pub facing_pages: bool,
    /// Top, bottom, inside, outside (stored as inside, top, outside, bottom).
    pub bleed: [f64; 4],
    /// 0 print, 1 web, 2 mobile.
    pub intent: u32,
    /// Page binding (u16 at 64): 0 left to right, 1 right to left.
    pub page_binding: u16,
}

/// A language object (class 0x2D07), chunk 0x2D0F. See
/// `docs/format/objects.md`, languages.
#[derive(Debug, Clone, PartialEq)]
pub struct Language {
    pub uid: u32,
    pub name: String,
    pub primary: String,
    pub sub: String,
    pub id: u16,
    /// The spelling and hyphenation vendors, each with its flag byte.
    pub vendors: Option<[(u8, String); 2]>,
}

/// A bullet character of the document's list (IDML `ABullet`): u32
/// character type, u32 character value, u32 font family (0 = none), the
/// font style as a flag byte and string, then a byte (0 in every sample).
#[derive(Debug, Clone, PartialEq)]
pub struct Bullet {
    /// 0 `UnicodeOnly`, 1 `UnicodeWithFont`, 2 `GlyphWithFont`.
    pub kind: u32,
    pub value: u32,
    pub font: u32,
    pub font_style: Name,
}

impl<'a> Reader<'a> {
    /// The bullet characters in the preferences object (chunk 0x1A488):
    /// u16 1, u32 count, then the bullets.
    pub(super) fn bullets(&self) -> Result<Vec<Bullet>, Error> {
        let Some(&(uid, _)) = self
            .db
            .classes()
            .iter()
            .find(|(_, c)| *c == class::PREFERENCES)
        else {
            return Ok(Vec::new());
        };
        let Some(d) = self.chunk(uid, chunk::BULLETS)? else {
            return Ok(Vec::new());
        };
        let mut c = self.cursor(&d);
        if c.u16()? != 1 {
            return Ok(Vec::new());
        }
        let n = c.u32()?;
        let mut out = Vec::new();
        for _ in 0..n {
            let kind = c.u32()?;
            let value = c.u32()?;
            let font = c.u32()?;
            let font_style = c.name()?;
            // 0, or 1 and four bytes of unknown meaning (objects.md).
            match c.u8()? {
                0 => {}
                1 => c.skip(4)?,
                other => {
                    return Err(Error::Corrupt(format!(
                        "bullet font style followed by {other} at {}",
                        c.pos() - 1
                    )));
                }
            }
            out.push(Bullet {
                kind,
                value,
                font,
                font_style,
            });
        }
        Ok(out)
    }

    /// The colour groups listed in the preferences object (chunk 0x1F61).
    pub(super) fn color_group_order(&self) -> Result<Vec<u32>, Error> {
        let Some(&(uid, _)) = self
            .db
            .classes()
            .iter()
            .find(|(_, c)| *c == class::PREFERENCES)
        else {
            return Ok(Vec::new());
        };
        self.uid_list(uid, color::chunk::COLOR_GROUPS)
    }

    pub(super) fn document_preferences(&self) -> Result<Option<DocumentPreferences>, Error> {
        let Some(&(uid, _)) = self
            .db
            .classes()
            .iter()
            .find(|(_, c)| *c == class::PREFERENCES)
        else {
            return Ok(None);
        };
        let Some(d) = self.chunk(uid, chunk::DOCUMENT_PREFERENCES)? else {
            return Ok(None);
        };
        if d.len() < 146 {
            self.warn(format!(
                "document preferences of {} bytes are not known; left out",
                d.len()
            ));
            return Ok(None);
        }
        let f = |o: usize| self.cursor(&d[o..]).f64();
        let u = |o: usize| self.cursor(&d[o..]).u32();
        let binding = self.cursor(&d[64..]).u16()?;
        if binding > 1 {
            self.warn(format!(
                "document preferences: page binding code {binding} is not known; left out"
            ));
        }
        Ok(Some(DocumentPreferences {
            page_width: f(0)?,
            page_height: f(8)?,
            facing_pages: d[58] == 2,
            bleed: [f(78)?, f(94)?, f(70)?, f(86)?],
            intent: u(142)?,
            page_binding: binding,
        }))
    }

    /// The document's users, from chunk 0xA443 of the document.
    pub(super) fn users(&self, doc: u32) -> Result<Vec<(u8, String)>, Error> {
        let Some(d) = self.chunk(doc, chunk::DOC_USERS)? else {
            return Ok(Vec::new());
        };
        let mut c = self.cursor(&d);
        let n = c.u32()?;
        if n as usize > d.len() / 9 {
            return Err(Error::Corrupt(format!("{n} document users")));
        }
        (0..n)
            .map(|_| {
                let flag = c.flag()?;
                let name = c.string()?;
                c.u32()?;
                Ok((flag, name))
            })
            .collect()
    }

    /// The numbering lists (preferences chunk 0x1A49B): the default list
    /// first, then the others in stored order. A list whose chunk cannot
    /// be read is left out with a warning. See `docs/format/objects.md`.
    pub(super) fn numbering_lists(&self) -> Result<Vec<NumberingList>, Error> {
        let Some(&(prefs, _)) = self
            .db
            .classes()
            .iter()
            .find(|(_, c)| *c == class::PREFERENCES)
        else {
            return Ok(Vec::new());
        };
        let Some(d) = self.chunk(prefs, chunk::NUMBERING_LISTS)? else {
            return Ok(Vec::new());
        };
        let mut c = self.cursor(&d);
        let default = c.u32()?;
        let n = c.u16()? as usize;
        let mut uids = vec![default];
        for _ in 0..n {
            uids.push(c.u32()?);
        }
        let mut out = Vec::new();
        for uid in uids {
            if self.class(uid) != Some(class::NUMBERING_LIST) {
                continue;
            }
            let Some(d) = self.chunk(uid, chunk::NUMBERING_LIST_INFO)? else {
                continue;
            };
            let mut c = self.cursor(&d);
            let read = (|| -> Result<NumberingList, Error> {
                let name = c.name()?.idml();
                let flags = c.bytes(3)?;
                Ok(NumberingList {
                    uid,
                    name,
                    across_stories: flags[0] == 1,
                    across_documents: flags[2] == 1,
                })
            })();
            match read {
                Ok(l) => out.push(l),
                Err(e) => self.warn(format!("numbering list {uid} left out: {e}")),
            }
        }
        Ok(out)
    }

    /// The label of an object (chunk 0x1630B of the document): u16 0x7B7B,
    /// u32 count, then key and value of each pair as a flag byte (1 for a
    /// built-in key) and an in-object string. See `docs/format/objects.md`.
    pub(super) fn label(&self, uid: u32) -> Result<Vec<(String, String)>, Error> {
        let Some(d) = self.chunk(uid, chunk::DOC_LABEL)? else {
            return Ok(Vec::new());
        };
        let mut c = self.cursor(&d);
        if c.u16()? != 0x7B7B {
            return Err(Error::Corrupt("label of unknown layout".into()));
        }
        let n = c.u32()? as usize;
        if n > d.len() / 8 {
            return Err(Error::Corrupt(format!("{n} label pairs")));
        }
        (0..n)
            .map(|_| Ok((c.name()?.idml(), c.name()?.idml())))
            .collect()
    }

    /// The code of the last record of the save history (chunk 0x1D8 of
    /// the document): u32 count, then per session u16 kind, u16 platform,
    /// two u16, u16 code, a flag byte and the application version string,
    /// u16 build and a FILETIME (two u32). `None` when the chunk is
    /// missing or empty or does not parse. See `docs/format/objects.md`,
    /// save history.
    pub(super) fn last_session_code(&self, doc: u32) -> Option<u16> {
        let d = self.chunk(doc, chunk::DOC_HISTORY).ok()??;
        let mut c = self.cursor(&d);
        let n = c.u32().ok()? as usize;
        // A record with an empty string has 25 bytes.
        if n == 0 || n > d.len() / 25 {
            return None;
        }
        let mut code = None;
        for _ in 0..n {
            for _ in 0..4 {
                c.u16().ok()?;
            }
            code = Some(c.u16().ok()?);
            c.flag().ok()?;
            c.string().ok()?;
            c.u16().ok()?;
            c.u32().ok()?;
            c.u32().ok()?;
        }
        code
    }

    /// The script byte of the last document user's name: the second byte
    /// of its in-object string (`objects.md`, document users). Only read
    /// in little-endian files.
    pub(super) fn users_script(&self, doc: u32) -> Option<u8> {
        if self.enc().big_endian() {
            return None;
        }
        let d = self.chunk(doc, chunk::DOC_USERS).ok()??;
        let mut c = self.cursor(&d);
        let mut last = None;
        for _ in 0..c.u32().ok()? {
            c.flag().ok()?;
            last = d.get(c.pos() + 1).copied();
            c.string().ok()?;
            c.u32().ok()?;
        }
        last
    }

    /// The index sort groups of the preferences object, chunk 0x1307E:
    /// name, include flag and header variant of each group, in stored
    /// order. Empty when the chunk does not parse to its end. See
    /// `docs/format/objects.md`, index sort options.
    pub(super) fn index_groups(&self) -> Vec<(String, bool, u16)> {
        let Some(&(uid, _)) = self
            .db
            .classes()
            .iter()
            .find(|(_, c)| *c == class::PREFERENCES)
        else {
            return Vec::new();
        };
        let Ok(Some(d)) = self.chunk(uid, chunk::INDEX_GROUPS) else {
            return Vec::new();
        };
        if self.enc().big_endian() {
            return Vec::new();
        }
        match index_groups(&mut self.cursor(&d)) {
            Ok(g) => g,
            Err(e) => {
                self.warn(format!("index sort options left out: {e}"));
                Vec::new()
            }
        }
    }

    /// The first pasted smooth shade (class 0x5533) whose chunk 0x5532
    /// holds a constant shade: 92 bytes, with u32 28 at offset 60 and then
    /// a u32 and three f64. See `docs/format/objects.md`.
    pub(super) fn constant_shade(&self) -> Option<ConstantShade> {
        let mut uids: Vec<u32> = self
            .db
            .classes()
            .iter()
            .filter(|&&(_, c)| c == class::SMOOTH_SHADE)
            .map(|&(u, _)| u)
            .collect();
        uids.sort_unstable();
        uids.into_iter().find_map(|uid| {
            let d = self.chunk(uid, chunk::SMOOTH_SHADE).ok()??;
            if d.len() != 92 || self.enc().u32_at(&d, 60) != Some(28) {
                return None;
            }
            let name = self
                .chunk(uid, chunk::SMOOTH_SHADE_NAME)
                .ok()
                .flatten()
                .and_then(|n| {
                    let mut c = self.cursor(&n);
                    let builtin = c.flag().ok()? == 1;
                    Some((builtin, c.string().ok()?))
                });
            Some(ConstantShade {
                uid,
                count: self.enc().u32_at(&d, 64)?,
                values: [
                    self.enc().f64_at(&d, 68)?,
                    self.enc().f64_at(&d, 76)?,
                    self.enc().f64_at(&d, 84)?,
                ],
                name,
            })
        })
    }

    /// The named grids, in UID order: built-in key, name and the grid's
    /// own settings (chunk 0xCD02, which only grids made by the user
    /// have; objects.md, named grids).
    pub(super) fn named_grids(&self) -> Vec<NamedGrid> {
        let mut out = Vec::new();
        for &(uid, cls) in self.db.classes() {
            if cls != class::NAMED_GRID {
                continue;
            }
            let read = (|| -> Result<Option<NamedGrid>, Error> {
                let Some(d) = self.chunk(uid, chunk::NAMED_GRID)? else {
                    return Ok(None);
                };
                let mut c = self.cursor(&d);
                c.u32()?;
                let builtin = c.flag()? == 1;
                let name = c.string()?;
                let grid = match self.chunk(uid, chunk::PAGE_GRID)? {
                    Some(d) => Some(GridData::read(&mut self.cursor(&d))?),
                    None => None,
                };
                Ok(Some(NamedGrid {
                    builtin,
                    name,
                    grid,
                }))
            })();
            match read {
                Ok(Some(g)) => out.push(g),
                Ok(None) => {}
                Err(e) => self.warn(format!("named grid {uid} left out: {e}")),
            }
        }
        out
    }
}

impl<'a> Reader<'a> {
    /// A language object (class 0x2D07): its name, and the full language
    /// if the rest of chunk 0x2D0F can be read. `None` if it has no name.
    pub(super) fn language(&self, uid: u32) -> Result<Option<(String, Option<Language>)>, Error> {
        let Some(d) = self.chunk(uid, chunk::LANGUAGE_NAME)? else {
            return Ok(None);
        };
        if d.len() <= 1 {
            return Ok(None);
        }
        let mut c = self.cursor(&d);
        c.flag()?;
        let name = c.string()?;
        // Then the primary and secondary names, u16 ID, and two vendors
        // (flag, u32, string).
        let rest = (|| -> Result<_, Error> {
            c.flag()?;
            let primary = c.string()?;
            c.flag()?;
            let sub = c.string()?;
            let id = c.u16()?;
            let mut vendor = || -> Result<(u8, String), Error> {
                let flag = c.u8()?;
                c.u32()?;
                Ok((flag, c.string()?))
            };
            let spelling = vendor()?;
            let hyphenation = vendor()?;
            Ok((primary, sub, id, [spelling, hyphenation]))
        })();
        let language = rest.ok().map(|(primary, sub, id, vendors)| Language {
            uid,
            name: name.clone(),
            primary,
            sub,
            id,
            vendors: Some(vendors),
        });
        Ok(Some((name, language)))
    }
}

/// A u32 length in UTF-16 units, then text segments.
fn counted_string(c: &mut crate::object::Cursor) -> Result<String, Error> {
    let n = c.u32()? as usize;
    if n == 0 {
        Ok(String::new())
    } else {
        c.segments(n)
    }
}

/// The groups of chunk 0x1307E (`objects.md`, index sort options): for
/// each, a flagged name, u8 include, u8, u16 header variant, u16 and its
/// header variants with their sections. Returns name, include and
/// variant of each group.
fn index_groups(c: &mut crate::object::Cursor) -> Result<Vec<(String, bool, u16)>, Error> {
    let count = c.u32()?;
    let mut out = Vec::new();
    for _ in 0..count {
        c.u8()?;
        let name = c.string()?;
        let include = c.u8()? != 0;
        c.u8()?;
        let variant = c.u16()?;
        c.u16()?;
        for _ in 0..c.u32()? {
            for _ in 0..2 {
                c.u8()?;
                c.string()?;
            }
            counted_string(c)?;
            c.skip(2)?;
            for _ in 0..c.u32()? {
                counted_string(c)?;
                counted_string(c)?;
                c.u8()?;
                c.string()?;
                c.u16()?;
            }
        }
        out.push((name, include, variant));
    }
    if c.remaining() != 0 {
        return Err(Error::Corrupt(format!("{} bytes left", c.remaining())));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::synthetic;
    use crate::object::Encoding;

    /// A save history with one record per code.
    fn history(codes: &[u16]) -> Vec<u8> {
        let mut d = (codes.len() as u32).to_le_bytes().to_vec();
        for &code in codes {
            for v in [7u16, 0, 10, 0, code] {
                d.extend_from_slice(&v.to_le_bytes());
            }
            d.extend(synthetic::flagged_string(
                Encoding::default(),
                1,
                "16.1.0.20",
            ));
            d.extend_from_slice(&20u16.to_le_bytes());
            d.extend_from_slice(&[0; 8]);
        }
        d
    }

    fn last_code(chunk: Option<Vec<u8>>) -> Option<u16> {
        let chunks = synthetic::chunks(
            &chunk
                .into_iter()
                .map(|d| (chunk::DOC_HISTORY, d))
                .collect::<Vec<_>>(),
        );
        let objects = [(1, class::DOCUMENT, chunks)];
        let bytes = synthetic::image(&objects);
        let db = synthetic::database(&bytes, &objects);
        Reader::new(&db).last_session_code(1)
    }

    #[test]
    fn reads_the_code_of_the_last_session() {
        assert_eq!(last_code(Some(history(&[0x0100, 0x0101]))), Some(0x0101));
        assert_eq!(last_code(Some(history(&[0x0101, 0x0100]))), Some(0x0100));
        assert_eq!(last_code(Some(history(&[]))), None);
        assert_eq!(last_code(None), None);
        let mut cut = history(&[0x0101]);
        cut.pop();
        assert_eq!(last_code(Some(cut)), None);
    }
}
