# Measuring the converter

`tools/compare.py` converts every corpus INDD file that has an IDML
exported by InDesign, and compares the output with that IDML. This page
defines which pairs the comparison trusts and how the headline numbers are
computed. It records how the tooling measures; facts about the INDD format
stay in `docs/format/`.

Numbers below are from the corpus without the privately held samples
(`--exclude own/`): 654 distinct pairs whose IDML has the same major version
as the INDD.

## Stale pairs

An IDML in the corpus is not always an export of the same save as the INDD
next to it. Some were exported, then the document was edited and saved
again; some were exported from a copy. Comparing against such an IDML
counts the edits as converter errors.

### Rule

A pair is **stale** if any of these holds. The others are **trustworthy**.

| Signal | Data used | Stale when |
|---|---|---|
| IDML modified after the INDD | `xmp:ModifyDate` of the INDD's XMP packet (`indd xmp`) and of the IDML's `META-INF/metadata.xml` | The INDD's date is earlier than the IDML's |
| INDD saved over an hour after the IDML | Same dates | The INDD's date is more than 1 hour after the IDML's |
| No ModifyDate | Same dates | Either file has no `xmp:ModifyDate` |
| IDML objects not in the INDD | `Self="u…"` identifiers in the IDML; UIDs in the INDD database (`indd uids`) | The IDML names a UID that the INDD database does not have |
| IDML objects deleted in the INDD | Same identifiers; UIDs whose INDD slot has no class (`indd uids` prints class `-`) | The IDML names such a UID |
| INDD stories not in the IDML | The INDD's story list, as converted; the IDML's stories | The INDD has a story that the IDML does not |

Pairs whose IDML `DOMVersion` has a different major version from the INDD
header are not compared at all.

### Evidence

**IDML UIDs.** In 631 of 654 pairs every `Self="u…"` in the IDML is a UID
in the INDD database. In the other 23, between 1 and 1,361 IDML UIDs are
missing from the INDD, so the IDML describes objects that the INDD no
longer (or never) had.

**Deleted slots.** A UID can be in the INDD database without a class (the
third tree; `database.md`). In 25 of 654 pairs the IDML names such a
UID: the IDML has an object that the INDD has deleted. In 6 of these 25
no other signal fires. Their IDMLs name 1 to 185 such objects each, 218
in all: 116 `PDF`, 54 `Image`, 23 `EPS`, 5 `TextFrame`, 5 `Story`, 2
`Rectangle`, 2 `Link` and 1 `Page`.

**ModifyDate.** InDesign writes `xmp:ModifyDate` both when it saves an INDD
and when it exports an IDML. A document exported and then saved within a few
minutes (as when packaging) has a small positive gap. The table counts
pairs by gap (INDD date minus IDML date), split by whether their content
visibly differs: the presence signals above fire, or a story's text is
less than 90 % similar to the converter's text for the same story
(`difflib` ratio, ignoring U+FEFF, which the converter writes for
anchors). It was measured before the deleted-slot signal was added.

| Gap | Content same | Content differs |
|---|---:|---:|
| negative | 1 | 3 |
| 0 | 189 | 0 |
| under 1 minute | 207 | 2 |
| 1–10 minutes | 67 | 1 |
| 10–60 minutes | 31 | 0 |
| 1 hour – 1 day | 46 | 6 |
| 1–30 days | 45 | 17 |
| 30 days or more | 29 | 10 |

Pairs with a gap of 0 to 1 hour differ in 3 of 497 cases; pairs with a gap
over 1 hour differ in 33 of 153. A negative gap means the IDML was written
after the INDD's last save; in one such pair the two files have the same
`xmpMM:InstanceID` but the IDML is 64 seconds newer and has 9 objects the
INDD lacks, so it was exported from unsaved changes.

**XMP history did not separate the pairs as well.** The number of saves in
the INDD's `xmpMM:History` after the IDML's `xmpMM:InstanceID` was tried as
a signal: of 101 pairs with two or more saves after it, 76 showed no
difference. The gap in `xmp:ModifyDate` was kept instead.

**DOMVersion minor version is not a signal.** 17 pairs whose files carry
the same `xmpMM:InstanceID` (the same save) have an IDML `DOMVersion`
whose minor version is lower than the INDD header's.

### Result

The rule flags 165 of 654 pairs; 489 are trustworthy.

| Group | Pairs | Stories with identical text | Pairs with a differing story |
|---|---:|---:|---:|
| Trustworthy | 489 | 18,509 of 18,610 (99.5 %) | 47 (10 %) |
| Stale by a presence signal | 51 | 2,000 of 2,547 (78.5 %) | 29 (57 %) |
| Stale by ModifyDate only (INDD over 1 hour later) | 113 | 4,664 of 4,783 (97.5 %) | 35 (31 %) |
| Stale by ModifyDate only (IDML newer) | 1 | 12 of 12 | 0 |

Spot checks of the pairs flagged by ModifyDate alone show edits made after
the export: spelling corrections ("comming" in the IDML, "coming" in the
INDD; "Taging" and "Tagging"), changed punctuation, a reworded caption, a
changed date in a heading, and a heading with a line added. The 7
trustworthy pairs with a story less than 98 % similar were checked too:
there the converter leaves out text that the IDML has (an endnote story, text
in tracked changes, the end of a long line of characters), so the
difference is a converter error and not an edit.

`compare.py` prints the full numbers over all pairs and the numbers over
trustworthy pairs. `--stale N` lists N stale pairs with their reasons;
`--trusted` restricts the element tables, shortfalls and attribute details
to trustworthy pairs.

## Headline metric

`compare.py` ends with a headline over trustworthy pairs, followed by the
same numbers over all pairs.

### Value coverage

A **value** is one of these, in the reference IDML:

| Value | Counted once per | Reproduced when |
|---|---|---|
| Element presence | Element with `Self`; the first child without `Self` of each tag, if it has attributes (counted as `Parent/Child`); in a child without attributes, its children with attributes (`Parent/Child/Grandchild`); each preference element | The output has an element with the same tag and `Self` (of the parent, for children) |
| Attribute | Attribute other than `Self` | The output's attribute is equal, with numbers compared to 6 significant digits |
| `<Properties>` child | Child of `Properties` (`P.Name`) | Equal text and attributes; a structured child (`PathGeometry`, lists) is compared as a whole |
| Story text | Story | The text, with paragraph breaks, is identical |
| Text range | Start of a paragraph and character style range, at any depth inside wrappers, in the story's text or in a footnote, table cell or note (below) | The output has a range starting at the same offset of the same text flow |
| Text range attribute | Attribute or `Properties` child of the range | Equal |

When an element is not produced, its presence and all its values count as
not reproduced. When a story's text differs, its text ranges cannot be
lined up, so all their values count as not reproduced.

#### Text ranges inside wrappers

IDML puts some elements around text ranges or around text inside a range:
`HyperlinkTextSource`, `CrossReferenceSource`, `XMLElement`, `Change` and
`EndnoteRange`. They hold ranges at any level, for example
`ParagraphStyleRange > HyperlinkTextSource > CharacterStyleRange`,
`XMLElement > ParagraphStyleRange` at story level, or
`CharacterStyleRange > Change > ParagraphStyleRange` for a tracked change.
`compare.py` looks through them: every `ParagraphStyleRange` and
`CharacterStyleRange` of the story counts, and the text inside a wrapper
counts towards the offsets. Offsets count `Content` in UTF-16 code units
and every other element inside a character range (`Br`, `Table`,
`Footnote`, an anchored frame) as one position. `Properties`,
`XMLAttribute` and elements outside character ranges (`StoryPreference`,
`MetadataPacketPreference`, …) count nothing. When a range
nested in a wrapper starts at the same offset as the range around it, the
nested range is the one compared. The wrapper elements themselves have a
`Self` and count as elements.

Counting them changed the trustworthy totals as follows:

| Ranges counted | Reference values | Reproduced | Coverage | Extra values |
|---|---:|---:|---:|---:|
| Only `ParagraphStyleRange` children of the story and their `CharacterStyleRange` children | 11,142,835 | 10,501,458 | 94.24 % | 30,237 |
| Also inside `HyperlinkTextSource` and `CrossReferenceSource` | 11,158,348 | 10,505,425 | 94.15 % | 26,297 |
| Also inside `XMLElement`, `Change` and `EndnoteRange` | 11,508,691 | 10,505,425 | 91.28 % | 26,297 |

Before, the skipped ranges were not counted, and every later range of the
story started at a smaller offset in the reference than in the output: the
converter's ranges were counted as extra and the reference's as missing.
#### Text flows in footnotes, cells and notes

`Footnote`, `Cell` and `Note` elements hold paragraph and character
ranges of their own. Each is a text flow with its own offsets from 0,
numbered in document order; the story's own text is flow 0, where the
element counts as one position. A range is identified by its flow and
offset. The story text that decides whether a story's ranges are
compared includes the text of these flows, so the flows of a story line
up when its text is identical.

Counting these flows added 124,277 reference values in the trustworthy
pairs (11,508,691 → 11,632,968), 103,180 of them reproduced (coverage
92.71 % → 92.60 %, measured on the converter of that commit); extra
values did not change.

The 350,343 values inside `XMLElement`, `Change` and `EndnoteRange` come
from 23 trustworthy pairs; one document with `XMLElement` around its text
has 348,306 of them. They count although the converter does not reproduce
them yet, because the headline counts everything InDesign writes.

**Value coverage** is reproduced values divided by all values, summed over
pairs, so each value counts once and large documents weigh more. Per-pair
coverage is the same ratio within one pair. `target/compare/pairs.tsv`
lists every pair with its coverage and stale reasons (`--out DIR` changes
the directory).

**Document score** is the share of trustworthy pairs whose coverage is at
least 99 %, and at least 99.9 %.

### Extra values

Coverage counts only values that the reference has, so writing more never
lowers it. **Extra values** count the other direction: values the output
has and the reference does not, with the same definitions as above:

| Extra value | Counted as |
|---|---|
| Attribute or `<Properties>` child of an element both have | 1, under (element, attribute) |
| Element (or child element, or preference) that only the output has | 1 plus its values, under (element, `(element not in reference)`) |
| Text range that only the output has, in a story whose text is identical | 1 plus its attributes, under (`TextRange`, `(element not in reference)`) |
| Story that only the output has | 1 for its text, plus its text ranges |

An extra value is not always wrong: IDML leaves out some values that
equal a default, and the converter may write them. It is still output
that the reference does not confirm. The headline prints the number of
extra values and their share of the reference values;
`extras.tsv` (trustworthy pairs) and `extras-all.tsv` list them by key
with the number of documents affected, and `pairs.tsv` has a column for
each pair. Values left out (below) are left out of both counts.

### Values left out

Some values describe the computer that exported the IDML or the IDML
package itself, not the document. No INDD file can supply them, so they
are left out of the counts (`EXCLUDED` in `compare.py`).

| Element | Value | Why it cannot come from the INDD |
|---|---|---|
| `Font` | `Status` | It records whether the font is installed on the computer that exported the IDML. The same document gives a different value on another computer. |
| `idPkg:Story`, `idPkg:Spread` and the other part references in `designmap.xml` | `src` | The file names of the parts inside the IDML package. The IDML writer chooses them; they are not document content. |
| `Assignment` | `Name` | The INDD stores the name of the assignment that holds unassigned InCopy content in the language of the computer that made the document (`Unassigned InCopy Content`, `Contenu InCopy non affecté`, …). IDML writes `$ID/UnassignedInCopy` when the exporting InDesign recognises it (476 of 495 trustworthy pairs) and the stored name otherwise (`objects.md`, assignments). |
| `DocumentUser` | `UserColor` | The colour of each user of the document as the exporting InDesign shows it. The INDD stores a colour for each user (`objects.md`, document users), but the IDML colour does not follow from it: users stored with the same colour get different IDML colours in different files. |
| `Link` | `LinkImportTime`, `LinkImportModificationTime` | Local time of the exporting computer. The INDD stores each time as a count of 100 ns intervals since 1601-01-01 (the `LinkImportStamp` text `file <n> <size>` repeats the first one in 4,686 of 4,686 links). Read as UTC, it differs from the IDML time by a whole number of hours in 9,009 of 9,385 time values. The offset differs between documents (−8 to +13 hours) and, in 71 of 284 documents, between links of one document with the season of the date: the daylight-saving rule of the exporting computer's time zone. The INDD does not store the time zone. |
| `PrintPreference`, `PrintBookletPrintPreference` | `BitmapPrinting` | No byte of the print settings, the preferences object or the document object follows it. It follows the platform of the stored print record (Windows record: 102 `false`, 13 `true`; Cocoa: 213 `true`, 4 `false`; Carbon: 57 `true`, 19 `false`; none: 84 `true`, 2 `false`), so the exporting InDesign most likely evaluates it. |

Two more rules adjust the comparison for values that depend on the
exporting computer:

- **Links resolved to another file at export.** When InDesign exports,
  it looks for each linked file and writes what it finds. If the IDML's
  `LinkResourceURI` differs from the converter's (which comes from the
  INDD), the file was found in another folder or replaced by a newer one,
  and the link's `LinkResource…` and `LinkImport…` attributes describe
  that file, not the INDD. They are left out for that link. The headline
  prints how many links this affects (329 in trustworthy pairs).
- **Font technology suffix.** IDML adds ` (OTF)`, ` (TT)` or ` (T1)` to
  some font family names, depending on the fonts installed where it was
  exported (`fonts.md`; the same family has the suffix in some documents
  and not in others, and the strings occur in no INDD file). Family names
  (`FontFamily` `Name`, `Font` `FontFamily`, `AppliedFont`,
  `BulletsFont`) are compared with that suffix removed from both values.
  A `Font` element is matched by its `Self`, which contains the family
  name (`…FontnMinion Pro (OTF) Regular`), and its `Name` starts with
  it: the suffix is removed from the family name in both before the
  fonts are matched and compared. Otherwise each such font counted as
  missing and its counterpart in the output as extra (12,743 extra
  values in the trustworthy pairs before this rule).

A value enters this list only with a reason of this kind. Values that are
hard to decode, or that the converter does not write yet, stay in the
counts.

### Gap list

Each value that is not reproduced is counted against a key:
(element, attribute), (element, `(element not produced)`), or
(`TextRange`, `(story text differs)`). `gaps.tsv` (trustworthy pairs) and
`gaps-all.tsv` (all pairs) in the output directory list every key with the
number of documents affected and the number of values wrong and missing.
`compare.py` prints the top 30 keys by documents affected (`--gaps N`) and
the top 15 by values.

`extras.tsv` and `extras-all.tsv` list the extra values by key the same
way, and `compare.py` prints the top keys by documents affected.

`values.tsv` and `values-all.tsv` list every key with its number of
values and how many are reproduced, wrong and missing. Comparing them
between two runs shows whether a change reproduces fewer values of any
key.

## Schema validation

`compare.py --schemas … --jing …` validates every output against the
IDML RelaxNG schemas (`tools/validate.sh`). A conversion with any schema
error counts as a failure, with one exception.

**Endnote markup.** The 21.5 schema, which InDesign generates, rejects
InDesign's own endnote markup: an `Endnote` inside a
`CharacterStyleRange`, and any content inside an `EndnoteRange`
(`format/footnotes.md`). The converter writes endnotes as InDesign does.
An output's errors are accepted when both hold:

- every error is `element "Endnote" not allowed here` or names an element
  whose parent is an `EndnoteRange`;
- the reference IDML of the same pair has the same errors (same part and
  message, positions ignored).

`compare.py` prints how many files this accepts. Files without a
reference IDML get no exception: an endnote error there counts as a
failure, and `compare.py --all` lists those files separately.

## Audit: what the converter does not read

`indd audit <file>` needs no reference IDML. It converts the file while
recording which objects, chunks and attributes the converter reads
(`src/audit.rs`), then lists what the file contains that was not read:

| Item | Not read means |
|---|---|
| Class | No object of the class was read |
| Chunk | In a class the converter reads, no object's chunk with this ID was looked up |
| Byte-stream object | An object without chunks (such as embedded file data) was not read |
| Attribute | An attribute ID in a list of this kind (page item, object style, style, text, table, cell) was never looked up |
| Code | The converter looked up the attribute but has no IDML value for its code (an unknown enumeration value) |
| Strand run kind | A strand holds run data of a kind the converter skips |

It also lists the conversion's warnings. "Read" means looked up; a read
value can still be converted wrongly, which `compare.py` measures. IDs are
printed in hexadecimal and are not named.

`indd audit --tsv` prints the same as tab-separated rows, including the
items that were read. `tools/audit_corpus.py --exclude own/` audits every
distinct INDD and INDT file in the corpus and ranks the items by the number
of files in which they are not read, with the number of files in which
they are read. It writes the full tables to `target/audit/` and takes
about 20 seconds.
