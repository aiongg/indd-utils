//! Document-level settings and lists: document preferences, bullets,
//! colour group order, index groups, constant shade, named grids, document
//! users and languages.
//!
//! Evidence: `docs/format/objects.md` and `preferences.md`.

use super::*;

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

    /// The named grids, in UID order.
    pub(super) fn named_grids(&self) -> Vec<(bool, String)> {
        let mut out = Vec::new();
        for &(uid, cls) in self.db.classes() {
            if cls != class::NAMED_GRID {
                continue;
            }
            let read = (|| -> Result<Option<(u32, bool, String)>, Error> {
                let Some(d) = self.chunk(uid, chunk::NAMED_GRID)? else {
                    return Ok(None);
                };
                let mut c = self.cursor(&d);
                c.u32()?;
                let builtin = c.flag()? == 1;
                Ok(Some((uid, builtin, c.string()?)))
            })();
            match read {
                Ok(Some(g)) => out.push(g),
                Ok(None) => {}
                Err(e) => self.warn(format!("named grid {uid} left out: {e}")),
            }
        }
        out.sort_by_key(|g| g.0);
        out.into_iter().map(|(_, b, n)| (b, n)).collect()
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
