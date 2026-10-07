//! A document model built from database objects. See
//! `docs/format/objects.md` for the class and chunk layouts used here.
//!
//! `Reader` (`reader`) reads typed objects; `Reader::document` (`document`)
//! builds the `Document`. The areas it reads are in `spread` (spreads,
//! pages, layers, sections), `item` (page items and graphics), `story`,
//! `style`, `settings` (document-level settings and lists) and the other
//! child modules; `ids` holds the class and chunk IDs.

pub mod attrs;
pub mod cjk;
pub mod color;
mod document;
pub mod font;
pub mod hyperlink;
mod ids;
mod item;
pub mod prefs;
mod reader;
mod settings;
mod spread;
mod story;
mod strings;
mod style;
pub mod table;
pub mod variable;
pub mod xml;
pub mod xref;

pub use document::*;
pub use ids::*;
pub use item::*;
pub use reader::*;
pub use settings::*;
pub use spread::*;
pub use story::*;
use strings::*;
pub use style::*;

use std::collections::{BTreeMap, HashMap};

pub use crate::object::Name;
pub use attrs::{Attrs, Value};
pub use cjk::{CjkTable, CompositeFont, CompositeFontEntry};
pub use color::{Color, ColorGroup, Gradient, Ink, Tint};
pub use font::{Font, FontFamily};
pub use hyperlink::{Bookmark, Destination, DestinationKind, Hyperlink, SourceRange, TextSource};
pub use table::{Cell, CellFormat, Table, TableStyle};
pub use variable::TextVariable;
pub use xref::CrossReferenceFormat;

use crate::audit::List;
use crate::object::{Cursor, Encoding, Object};
use crate::{Database, Error, Version};

#[cfg(test)]
mod tests {
    use crate::database::synthetic;

    /// Chunk data of a child list: 8 bytes, then a u32 list.
    fn child_list(uids: &[u32]) -> Vec<u8> {
        let mut d = vec![0; 8];
        d.extend_from_slice(&(uids.len() as u32).to_le_bytes());
        for u in uids {
            d.extend_from_slice(&u.to_le_bytes());
        }
        d
    }

    fn group(uid: u32, children: &[u32]) -> (u32, u32, Vec<u8>) {
        let chunks = synthetic::chunks(&[(chunk::ITEM_HIERARCHY, child_list(children))]);
        (uid, class::GROUP, chunks)
    }

    #[test]
    fn groups_that_contain_themselves_are_left_out() {
        // Group 10 contains itself and group 11; group 11 contains 10.
        let objects = [group(10, &[10, 11]), group(11, &[10])];
        let bytes = synthetic::image(&objects);
        let db = synthetic::database(&bytes, &objects);
        let reader = Reader::new(&db);
        let item = reader.page_item(10, None).unwrap().unwrap();
        assert_eq!(item.children.len(), 1);
        assert_eq!(item.children[0].uid, 11);
        assert!(item.children[0].children.is_empty());
        assert_eq!(reader.warnings.borrow().len(), 2);
    }

    #[test]
    fn deeply_nested_groups_are_left_out() {
        let n = 3 * MAX_ITEM_DEPTH as u32;
        let objects: Vec<_> = (1..=n)
            .map(|u| group(u, &[u + 1][..(u < n) as usize]))
            .collect();
        let bytes = synthetic::image(&objects);
        let db = synthetic::database(&bytes, &objects);
        let reader = Reader::new(&db);
        let mut item = reader.page_item(1, None).unwrap().unwrap();
        let mut depth = 1;
        while let Some(child) = item.children.pop() {
            item = child;
            depth += 1;
        }
        assert_eq!(depth, MAX_ITEM_DEPTH);
        assert_eq!(reader.warnings.borrow().len(), 1);
    }

    #[test]
    fn style_group_cycles_are_cut() {
        let g = |uid, root, children: &[u32]| {
            (
                uid,
                StyleGroup {
                    uid,
                    name: String::new(),
                    root,
                    children: children.to_vec(),
                },
            )
        };
        // Root 1 holds groups 2 and 3 and style 50; 2 holds 3 and 1; 3 holds 2.
        let mut groups = BTreeMap::from([
            g(1, Some(root_kind::PARAGRAPH), &[2, 3, 50]),
            g(2, None, &[3, 1]),
            g(3, None, &[2]),
        ]);
        let warnings = std::cell::RefCell::new(Vec::new());
        prune_style_groups(&mut groups, |m| warnings.borrow_mut().push(m));
        assert_eq!(groups[&1].children, [2, 3, 50]);
        assert!(groups[&2].children.is_empty());
        assert!(groups[&3].children.is_empty());
        assert_eq!(warnings.borrow().len(), 3);
    }

    use super::*;

    #[test]
    fn pages_take_margins_from_master_page_at_same_position() {
        let margins = |top, own| Margins {
            left: 0.0,
            top,
            right: 0.0,
            bottom: 0.0,
            own,
        };
        let page = |uid, tx, master, m| Page {
            uid,
            bounds: [0.0; 4],
            transform: Matrix([1.0, 0.0, 0.0, 1.0, tx, 0.0]),
            master,
            master_transform: Matrix::IDENTITY,
            margins: Some(m),
            columns: None,
            grid: None,
            settings: Default::default(),
        };
        let spread = |uid, pages| Spread {
            uid,
            master_name: None,
            transform: Matrix::IDENTITY,
            binding_location: 0,
            pages,
            items: Vec::new(),
            guides: Vec::new(),
            shuffle: None,
            flattener_resolution: None,
            show_master_items: None,
        };
        // Master B (pages listed right to left) is based on master A.
        let mut masters = vec![
            spread(
                10,
                vec![
                    page(1, -100.0, None, margins(10.0, true)),
                    page(2, 0.0, None, margins(20.0, true)),
                ],
            ),
            spread(
                11,
                vec![
                    page(3, 0.0, Some(10), margins(0.0, false)),
                    page(4, -100.0, Some(10), margins(30.0, true)),
                ],
            ),
        ];
        let mut spreads = vec![spread(
            12,
            vec![
                page(5, -100.0, Some(11), margins(0.0, false)),
                page(6, 0.0, Some(11), margins(0.0, false)),
            ],
        )];
        resolve_page_layout(&mut spreads, &mut masters);
        let tops: Vec<f64> = spreads[0]
            .pages
            .iter()
            .map(|p| p.margins.as_ref().unwrap().top)
            .collect();
        assert_eq!(tops, [30.0, 20.0]);
    }

    #[test]
    fn counts_a_surrogate_pair_as_one_position() {
        let text: Vec<u16> = "a\u{1F93F}b".encode_utf16().collect();
        assert_eq!(char_offsets(&text), [0, 1, 3, 4]);
    }

    #[test]
    fn splits_runs_at_style_boundaries() {
        let text: Vec<u16> = "abcdef".encode_utf16().collect();
        let a = Attrs::default;
        let runs = split_runs(
            &text,
            &[(4, 10, a()), (2, 11, a())],
            &[(2, 20, a()), (4, 21, a())],
            &[],
        );
        let texts: Vec<_> = runs.iter().map(|r| r.text.as_str()).collect();
        assert_eq!(texts, ["ab", "cd", "ef"]);
        assert_eq!(runs[1].paragraph_style, Some(10));
        assert_eq!(runs[1].character_style, Some(21));
        assert_eq!(runs[2].paragraph_style, Some(11));
    }

    #[test]
    fn classifies_shapes() {
        let corner = |x, y| PathPoint {
            anchor: (x, y),
            left: (x, y),
            right: (x, y),
            kind: 2,
        };
        let rect = Path {
            points: vec![
                corner(0., 0.),
                corner(0., 1.),
                corner(1., 1.),
                corner(1., 0.),
            ],
            open: false,
        };
        assert_eq!(
            classify(std::slice::from_ref(&rect), None),
            Shape::Rectangle
        );
        // The stored shape code decides where there is one.
        assert_eq!(classify(&[rect], Some(7)), Shape::Polygon);
        let line = Path {
            points: vec![corner(0., 0.), corner(3., 1.)],
            open: true,
        };
        assert_eq!(classify(&[line], Some(9)), Shape::GraphicLine);
        let smooth = |x, y, kind| PathPoint {
            anchor: (x, y),
            left: (x - 1., y),
            right: (x + 1., y),
            kind,
        };
        let round = |kind| Path {
            points: vec![
                smooth(0., 1., kind),
                smooth(1., 0., kind),
                smooth(2., 1., kind),
                smooth(1., 2., kind),
            ],
            open: false,
        };
        assert_eq!(classify(&[round(0)], None), Shape::Oval);
        assert_eq!(classify(&[round(1)], None), Shape::Polygon);
    }
}
