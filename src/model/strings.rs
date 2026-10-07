//! Searches for names stored as in-object strings at positions that are
//! not decoded: layer, style and table style names.
//!
//! Evidence: `docs/format/objects.md`.

use super::*;

/// Find a flag byte (1 = built-in name) followed by an in-object string
/// that `accept` takes, at or after `from`. Returns the offset of the pair,
/// whether the flag is 1, and the string. In big-endian data the flag and
/// the string's tag (2) are swapped (`docs/format/big-endian.md`).
pub(super) fn find_flagged_string(
    data: &[u8],
    from: usize,
    accept: impl Fn(&str) -> bool,
) -> Option<(usize, bool, String)> {
    let big = crate::object::big_endian();
    (from..data.len().saturating_sub(6)).find_map(|i| {
        let header = match big {
            false => data[i] <= 2 && data[i + 1] == 2,
            true => data[i] == 2 && data[i + 1] <= 2,
        };
        if !header {
            return None;
        }
        let mut c = Cursor::new(&data[i..]);
        let builtin = c.flag().ok()? == 1;
        let s = c.string().ok()?;
        accept(&s).then_some((i, builtin, s))
    })
}

/// Find the first in-object string at or after `from`.
pub(super) fn find_string(data: &[u8], from: usize) -> Result<String, Error> {
    if crate::object::big_endian() {
        // The tag (2), the byte before it in little-endian data, then the
        // rest of the header (`Cursor::string`).
        for i in from..data.len().saturating_sub(5) {
            if data[i] == 2
                && data[i + 1] <= 2
                && u16_from([data[i + 3], data[i + 4]]) > 0
                && let Ok(s) = Cursor::new(&data[i..]).string()
            {
                return Ok(s);
            }
        }
        return Ok(String::new());
    }
    for i in from..data.len().saturating_sub(4) {
        if data[i] == 2 && data[i + 1] == 0 {
            let n = u16_from([data[i + 2], data[i + 3]]) as usize;
            if n > 0
                && i + 6 <= data.len()
                && (u16_from([data[i + 4], data[i + 5]]) & 0xC000) != 0
                && let Ok(s) = Cursor::new(&data[i..]).string()
            {
                return Ok(s);
            }
        }
    }
    Ok(String::new())
}
