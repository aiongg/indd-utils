# Transparency

Implemented in `src/idml/transparency.rs`.

Page items keep their transparency settings in their attribute list
(chunk 0x6E03, see `attributes.md`), with attribute IDs in two ranges:
0x108xx and 0x1EBxx. Object styles keep a full list of the same
attributes in chunk 0x1B92C (u16 count, then records).

IDML writes them as child elements of the page item: `TransparencySetting`
(the whole object), `StrokeTransparencySetting` and
`FillTransparencySetting`, each holding one element per effect
(`BlendingSetting`, `DropShadowSetting`, `InnerShadowSetting`,
`GradientFeatherSetting`, ...).

## How the mapping was checked

IDML writes a setting on a page item only when it differs from the item's
object style. So each INDD value was compared with the value in effect:

1. the value written on the IDML item, if there is one;
2. otherwise the value of its object style, following `BasedOn`;
3. otherwise the default in `Resources/Preferences.xml`
   (`TransparencyDefaultContainerObject`).

The tables count page items of the pairs whose IDML is from the same
InDesign version as the INDD. "On the item" means the IDML item states the
value itself, which is the evidence that tells the attribute apart. The
576 object styles of the pairs (matched by name) agree with their IDML
style on every attribute below.

## Object transparency (`TransparencySetting`)

| ID | IDML | Encoding | On the item |
|---|---|---|---|
| 0x10816 | `BlendingSetting/Opacity` | f64, percent | 57 items in 4 files (40, 49, 50, 57, 69, 70) |
| 0x10817 | `BlendingSetting/BlendMode` | i32 code (below) | 56 items in 4 files |
| 0x1081A | `DropShadowSetting/Mode` | i32: 0 `None`, 1 `Drop` | 8 items with 1 in 2 files |
| 0x10820 | `DropShadowSetting/Size` | f64 | 6 items, two values; 406 of 406 rectangles and text frames over the 654 pairs of the later corpus |
| 0x1084D | `InnerShadowSetting/Applied` | u16: 1 true, 0 false | 10 items with 1 |
| 0x1084E | `InnerShadowSetting/EffectColor` | swatch UID | 10 items (below) |
| 0x10852 | `InnerShadowSetting/Distance` | f64 | 10 items with 0 |
| 0x10855 | `InnerShadowSetting/Size` | f64 | 10 items with 2.83 |
| 0x1EB8A | `GradientFeatherSetting/Applied` | u16: 1 true, 0 false | 56 items in 2 files |
| 0x1EB8C | `GradientFeatherSetting` stops | list (below) | 59 items, 170 stops |
| 0x1EB8D | `GradientFeatherSetting/Angle` | f64 | 1 item (−90) |
| 0x1EB8E | `GradientFeatherSetting/Length` | f64 | 96 items in 3 files |
| 0x1EB8F | `GradientFeatherSetting/GradientStart` | two f64 | 96 items in 3 files |
| 0x1EB91 | `GradientFeatherSetting/HiliteAngle` | f64 | 1 item (−62.2) |
| 0x1EB92 | `GradientFeatherSetting/HiliteLength` | f64 | 1 item (1) |

All items whose value is not on the IDML item (it comes from the style
or the default) match as well: for example 743 items for
`InnerShadowSetting` and the gradient feather attributes, with the
default values (`Applied="false"`, `Distance="7"`, `Size="7"`, `Length="0"`).

In pairs whose IDML is from an older version, 14 more items match on
opacity and blend mode. One item there has 50 where the IDML says 100;
that file was probably saved again after the export.

**Values outside the schema's range.** Four items in a file without an
IDML store a drop shadow size of −2.83; the schema allows 0 to 1000.
The converter leaves out any value of the attributes above that is
outside the schema's range, with a warning: sizes and distances 0–1000,
opacity 0–100, angles −180–180.

**Blend mode codes.** 1 `Multiply` (3 items), 3 `Overlay` (62),
9 `Lighten` (5); 0 `Normal` in all 576 object styles, whose IDML blend mode
is `Normal`. These codes are the positions of the values in the IDML
schema's `BlendMode` enumeration, but no other code occurs in the corpus,
so the converter writes only these four.

**Inner shadow colour.** The 10 items with the effect applied name the
swatch that IDML writes as `EffectColor`. Items without the effect store
UID 0xB, where the IDML default is `n` (no colour). The converter writes
the colour only when the effect is applied.

## Stroke and fill transparency

Only the gradient feather has values that tell its attributes apart:

| ID | IDML | On the item |
|---|---|---|
| 0x1EB93 | `StrokeTransparencySetting/GradientFeatherSetting/Applied` | 2 items through their style, and the style |
| 0x1EB95 | stroke gradient feather stops | 2 items |
| 0x1EB96 | `StrokeTransparencySetting/GradientFeatherSetting/Angle` | 2 items through their style (90), and the style |
| 0x1EB97 | `StrokeTransparencySetting/GradientFeatherSetting/Length` | 4 items in 2 files |
| 0x1EB98 | `StrokeTransparencySetting/GradientFeatherSetting/GradientStart` | 4 items in 2 files |
| 0x1EB9C | `FillTransparencySetting/GradientFeatherSetting/Applied` | 10 items, 1 through its style |
| 0x1EB9E | fill gradient feather stops | 11 items |
| 0x1EB9F | `FillTransparencySetting/GradientFeatherSetting/Angle` | 10 items (−90) |
| 0x1EBA0 | `FillTransparencySetting/GradientFeatherSetting/Length` | 3 items in 2 files |
| 0x1EBA1 | `FillTransparencySetting/GradientFeatherSetting/GradientStart` | 3 items in 2 files |

The stroke, fill and object gradient feathers of the same item can hold
the same length and start, so each ID was taken from items where only one
of the three is set (for example 0x1EB97 matches the stroke value in 4
items and the fill value in only 2).

## Opacity gradient stops

Value type 0x1085F. A u32 stop count, then three f64 per stop:

| Field | Contents |
|---|---|
| f64 | Location, 0–1 (IDML `Location` in percent) |
| f64 | Position of the midpoint between this stop and the next, 0–1, measured from the start of the gradient; 1 for the last stop |
| f64 | Opacity in percent (IDML `Opacity`) |

IDML writes a `Midpoint` on every stop but the first: the midpoint
between the previous stop and this one, in percent of the distance
between them. So for stop *i*, `Midpoint` = (midpoint of stop *i*−1 −
location of stop *i*−1) / (location of stop *i* − location of stop *i*−1)
× 100. A list with count 0 is stored as a 4-byte value.

All 170 stops of the items above match on `Opacity`, `Location` and
`Midpoint`; so do the 5 stops of two object styles. IDML names an item's
stops `<item>TransparencySetting1GradientFeatherSetting1OpacityGradientStop<i>`
(with `StrokeTransparencySetting` or `FillTransparencySetting` for those
settings), 170 of 170.

## Not converted

- `DropShadowSetting/XOffset` and `YOffset`: attributes 0x1081B and
  0x1081C hold the offsets, but every item and style has the same value in
  both, so which is which is not known.
- Stroke, fill and content blending (opacity and blend mode) and the
  other effects (feather, outer and inner glow, bevel and emboss, satin,
  directional feather): no item or style in the pairs has a value other
  than the default, so their attributes cannot be told apart. One text
  frame has `ContentTransparencySetting` with an inner glow and a satin
  effect applied, and two attributes set to 1, so it cannot show which is
  which.
- `GradientFeatherSetting/Type`: `Linear` in every sample.
- Transparency of placed graphics and object styles.
