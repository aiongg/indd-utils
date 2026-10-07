//! Page numbering: the pages of each section, page names in the section's
//! number style, and the pages of alternate layouts.
//!
//! Evidence: `docs/format/objects.md` (sections and pages).

use super::*;

/// UIDs of the document pages, in order.
pub(super) fn document_pages(doc: &Document) -> Vec<u32> {
    doc.spreads
        .iter()
        .flat_map(|s| &s.pages)
        .map(|p| p.uid)
        .collect()
}

/// Sections in page order, each with the index of its first page and its
/// length in pages. A section whose first page is not a document page is
/// left out.
pub(super) fn section_ranges(doc: &Document) -> Vec<(&Section, usize, usize)> {
    let pages = document_pages(doc);
    if pages.is_empty() {
        return Vec::new();
    }
    let mut starts: Vec<(usize, &Section)> = doc
        .sections
        .iter()
        .filter_map(|s| match s.page {
            None => Some((0, s)),
            Some(p) => pages.iter().position(|&u| u == p).map(|i| (i, s)),
        })
        .collect();
    starts.sort_by_key(|&(i, _)| i);
    starts.dedup_by_key(|&mut (i, _)| i);
    (0..starts.len())
        .map(|k| {
            let (i, s) = starts[k];
            let end = starts.get(k + 1).map_or(pages.len(), |&(j, _)| j);
            (s, i, end - i)
        })
        .collect()
}

/// The IDML `AlternateLayout` of a section: its name, with `$ID/` before
/// a built-in key.
pub(super) fn alternate_layout_name(s: &Section) -> Option<String> {
    s.alternate_layout.as_ref().map(|(flag, name)| match flag {
        1 => builtin_key(name),
        _ => name.clone(),
    })
}

/// For each section of `section_ranges`, the number of pages from its
/// start to the start of the next alternate layout (a section with a
/// layout name) or the end of the document; and for each document page,
/// the section that starts its alternate layout. See
/// `docs/format/objects.md`, sections.
pub(super) fn alternate_layouts(doc: &Document) -> (Vec<usize>, Vec<u32>) {
    let ranges = section_ranges(doc);
    let total: usize = ranges.iter().map(|&(_, _, n)| n).sum();
    let starts: Vec<(usize, u32)> = ranges
        .iter()
        .filter(|(s, _, _)| alternate_layout_name(s).is_some_and(|n| !n.is_empty() && n != "$ID/"))
        .map(|&(s, i, _)| (i, s.uid))
        .collect();
    let lengths = ranges
        .iter()
        .map(|&(_, i, _)| {
            starts
                .iter()
                .find(|&&(j, _)| j > i)
                .map_or(total, |&(j, _)| j)
                - i
        })
        .collect();
    let first = ranges.first().map(|(s, _, _)| s.uid);
    let pages = (0..total)
        .filter_map(|p| {
            starts
                .iter()
                .rev()
                .find(|&&(j, _)| j <= p)
                .map(|&(_, u)| u)
                .or(first)
        })
        .collect();
    (lengths, pages)
}

/// IDML `PageNumberStyle` of a section's style code.
pub(super) fn number_style(code: u32) -> Option<&'static str> {
    match code {
        numbering::ARABIC => Some("Arabic"),
        numbering::LOWER_ROMAN => Some("LowerRoman"),
        numbering::KANJI => Some("Kanji"),
        _ => None,
    }
}

/// Lower-case Roman numeral of `n` (1 or more).
pub(super) fn lower_roman(mut n: u32) -> String {
    const DIGITS: [(u32, &str); 13] = [
        (1000, "m"),
        (900, "cm"),
        (500, "d"),
        (400, "cd"),
        (100, "c"),
        (90, "xc"),
        (50, "l"),
        (40, "xl"),
        (10, "x"),
        (9, "ix"),
        (5, "v"),
        (4, "iv"),
        (1, "i"),
    ];
    let mut out = String::new();
    for (value, digits) in DIGITS {
        while n >= value {
            out.push_str(digits);
            n -= value;
        }
    }
    out
}

/// Chinese numeral of `n`, digit by digit (10 is 一〇), as the folios of
/// a sample with that style show.
pub(super) fn kanji_digits(n: u32) -> String {
    const DIGITS: [char; 10] = ['〇', '一', '二', '三', '四', '五', '六', '七', '八', '九'];
    n.to_string()
        .bytes()
        .map(|b| DIGITS[usize::from(b - b'0')])
        .collect()
}

/// The name of each document page: its number (see `page_numbers`) in
/// its section's style. Styles other than lower-case Roman and Kanji are
/// written as Arabic numbers.
pub(super) fn page_names(doc: &Document) -> Vec<String> {
    let numbers = page_numbers(doc);
    let mut out: Vec<String> = numbers.iter().map(u32::to_string).collect();
    for (s, first, length) in section_ranges(doc) {
        let name: fn(u32) -> String = match s.style {
            numbering::LOWER_ROMAN => lower_roman,
            numbering::KANJI => kanji_digits,
            _ => continue,
        };
        for i in (first..first + length).filter(|&i| numbers[i] > 0) {
            out[i] = name(numbers[i]);
        }
    }
    out
}

/// The number shown on each document page, from the sections. Pages
/// not covered by a section are numbered by their position.
/// The section and page number of each document page.
pub(super) fn page_sections(doc: &Document) -> Vec<(Section, u32)> {
    let numbers = page_numbers(doc);
    let mut out = Vec::new();
    for (s, first, length) in section_ranges(doc) {
        for &n in numbers.iter().skip(first).take(length) {
            out.push((s.clone(), n));
        }
    }
    out
}

pub(super) fn page_numbers(doc: &Document) -> Vec<u32> {
    let count = document_pages(doc).len();
    let mut out: Vec<u32> = (1..=count as u32).collect();
    let mut next: Option<u32> = None;
    for (s, first, length) in section_ranges(doc) {
        let mut number = match next {
            Some(n) if s.continue_numbering => n,
            None if s.continue_numbering => first as u32 + 1,
            _ => s.start,
        };
        for n in out.iter_mut().skip(first).take(length) {
            *n = number;
            number += 1;
        }
        next = Some(number);
    }
    out
}
