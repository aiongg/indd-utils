//! Dates of the document's XMP packet, which give the time zone offsets
//! that IDML link times are written in (`docs/format/objects.md`, link
//! times).

/// An XMP date with a UTC offset: the instant (seconds since 1970 UTC)
/// and the offset in seconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct XmpDate {
    pub utc: i64,
    pub offset: i64,
}

/// The properties whose dates are read.
const DATE_PROPERTIES: [&str; 4] = [
    "xmp:CreateDate",
    "xmp:MetadataDate",
    "xmp:ModifyDate",
    "stEvt:when",
];

/// The dates of `packet` that have a UTC offset (`Z` or `±HH:MM`), from
/// the properties `xmp:CreateDate`, `xmp:MetadataDate`, `xmp:ModifyDate`
/// and `stEvt:when` (each in attribute or element form). Dates without
/// an offset or that do not parse are left out.
pub fn dates(packet: &[u8]) -> Vec<XmpDate> {
    let text = String::from_utf8_lossy(packet);
    let mut out = Vec::new();
    for name in DATE_PROPERTIES {
        let mut rest = &text[..];
        while let Some(i) = rest.find(name) {
            rest = &rest[i + name.len()..];
            let value = if let Some(r) = rest.strip_prefix("=\"") {
                r
            } else if let Some(r) = rest.strip_prefix('>') {
                r
            } else {
                continue;
            };
            if let Some(d) = parse(value) {
                out.push(d);
            }
        }
    }
    out
}

/// A date `YYYY-MM-DDTHH:MM[:SS[.fff]]` followed by `Z` or `±HH:MM`.
fn parse(s: &str) -> Option<XmpDate> {
    let b = s.as_bytes();
    let num = |from: usize, len: usize| -> Option<i64> {
        let t = s.get(from..from + len)?;
        t.bytes()
            .all(|c| c.is_ascii_digit())
            .then(|| t.parse().ok())?
    };
    let (year, month, day) = (num(0, 4)?, num(5, 2)?, num(8, 2)?);
    if b.get(4) != Some(&b'-') || b.get(7) != Some(&b'-') || b.get(10) != Some(&b'T') {
        return None;
    }
    let (hour, minute) = (num(11, 2)?, num(14, 2)?);
    let mut i = 16;
    let mut second = 0;
    if b.get(i) == Some(&b':') {
        second = num(17, 2)?;
        i = 19;
        if b.get(i) == Some(&b'.') {
            i += 1;
            while b.get(i).is_some_and(u8::is_ascii_digit) {
                i += 1;
            }
        }
    }
    let offset = match b.get(i)? {
        b'Z' => 0,
        &c @ (b'+' | b'-') => {
            let v = num(i + 1, 2)? * 3600 + num(i + 4, 2)? * 60;
            if c == b'-' { -v } else { v }
        }
        _ => return None,
    };
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) || hour > 23 || minute > 59 {
        return None;
    }
    let local = days_from_civil(year, month, day) * 86_400 + hour * 3600 + minute * 60 + second;
    Some(XmpDate {
        utc: local - offset,
        offset,
    })
}

/// Days since 1970-01-01 of a date in the proleptic Gregorian calendar.
pub fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// The date (year, month, day) of a day count since 1970-01-01.
pub fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

/// The day of the year (0-based) of an instant, in UTC.
pub fn day_of_year(utc: i64) -> i64 {
    let days = utc.div_euclid(86_400);
    let (year, _, _) = civil_from_days(days);
    days - days_from_civil(year, 1, 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_dates_with_offsets() {
        let packet = br#"<x xmp:CreateDate="2020-03-01T10:00:00+01:00" xmp:ModifyDate="2020-07-01T10:00:00.25-04:00">
<stEvt:when>2021-01-02T03:04:05Z</stEvt:when><xmp:MetadataDate>2020-01-01T00:00</xmp:MetadataDate></x>"#;
        let d = dates(packet);
        assert_eq!(d.len(), 3);
        assert_eq!(d[0].offset, 3600);
        assert_eq!(d[0].utc, days_from_civil(2020, 3, 1) * 86_400 + 9 * 3600);
        assert_eq!(d[1].offset, -4 * 3600);
        assert_eq!(d[2].offset, 0);
    }

    #[test]
    fn converts_between_days_and_dates() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(civil_from_days(days_from_civil(2024, 2, 29)), (2024, 2, 29));
        assert_eq!(civil_from_days(days_from_civil(1601, 1, 1)), (1601, 1, 1));
        assert_eq!(day_of_year(days_from_civil(2023, 12, 31) * 86_400), 364);
    }
}
