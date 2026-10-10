# Measuring the converter

`tools/compare.py` converts every corpus INDD file that has an IDML
exported by InDesign, and compares the output with that IDML. This page
defines which pairs the comparison trusts and how the headline numbers are
computed. It records how the tooling measures; facts about the INDD format
stay in `docs/format/`.

Numbers below are from the corpus without the privately held samples
(`--exclude own/`). The evidence was measured when the corpus had 654
distinct pairs whose IDML has the same major version as the INDD (489
trustworthy); counts given for the corpus of 2026-10, with 803 such pairs
(606 trustworthy), say so.

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

The rule flags 165 of 654 pairs; 489 are trustworthy. In the corpus of
2026-10 it flags 197 of 803 pairs (INDD saved over an hour after the
IDML: 185; IDML objects deleted in the INDD: 36; INDD stories not in the
IDML: 24; IDML objects not in the INDD: 23; IDML modified after the INDD:
4; a pair can have several reasons); 606 are trustworthy, and 19,416 of
their 19,424 stories have identical text.

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
| `<Properties>` child | Child of `Properties` (`P.Name`) | Equal text and attributes; a structured child (`PathGeometry`, lists) is compared as a whole. Text that is only a line feed and tabs counts as empty: InDesign writes some empty elements open and closed on two lines (`XMLImportPreference/Properties/TransformParameters`) |
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

## Rejected files

`compare.py --all` also converts files without a reference. Some of them
the converter rejects for a correct reason: the file is not an INDD
file, is from InDesign 1.x (another container), is truncated, or has no
object database. These are counted as **rejected files**, by kind
(`REJECTIONS` in `compare.py`), not as conversion failures. A
**conversion failure** is a valid file whose conversion fails.
`summary.json` lists both (`rejected`, `failures`) for paired and other
files, and the README's table reports them in separate rows.

## Schema validation

`compare.py --schemas … --jing …` validates every output against the
IDML RelaxNG schemas (`tools/validate.sh`). A conversion with any schema
error counts as a failure, with one exception.

**Endnote markup.** The 21.5 schema, which InDesign generates, rejects
InDesign's own endnote markup: an `Endnote` inside a
`CharacterStyleRange`, and any content inside an `EndnoteRange`
(`format/footnotes.md`). The converter writes endnotes as InDesign does.
An output's errors are accepted when both hold:

- every error is `element "Endnote" not allowed here`, names an element
  whose parent is an `EndnoteRange`, or is an anchored form field (below);
- the reference IDML of the same pair has the same errors (same part and
  message, positions ignored).

**Anchored form fields.** The same schema rejects a `CheckBox`,
`RadioButton`, `TextBox`, `ComboBox` or `SignatureField` inside a
`CharacterStyleRange`, which is where InDesign writes a form field
anchored in text (`format/objects.md`, form fields). The reference IDML
of every pair with such a field has these errors (3 pairs of the corpus
of 2026-10), and the converter writes the fields as InDesign does. The
rule above accepts them in the same way.

`compare.py` prints how many files this accepts. Files without a
reference IDML get no exception: an endnote or anchored form field error
there counts as a failure, and `compare.py --all` lists those files
separately.

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

## Render comparison

`tools/render_compare.py` compares renders of the documents that come
with a PDF exported by InDesign. For each same-save triple (an INDD file,
its reference IDML and its PDF, all of the same save) it compares three
PDFs page by page:

| PDF | What it is |
|---|---|
| R | The InDesign PDF from the corpus |
| A | DesignCraft's PDF of the reference IDML |
| B | DesignCraft's PDF of the converter's IDML |

- **A vs B measures the converter.** Both are rendered by the same program
  from the same folder, with the same fonts and linked files, so a
  difference comes from the IDML. A pixel-identical page is a page whose
  rendering the converter's IDML does not change.
- **R vs A measures DesignCraft.** Where the reference IDML renders
  differently from InDesign's PDF, the renderer differs from InDesign.
  Most of these differences come from fonts DesignCraft does not have, so
  issue counts are reported for font-complete documents separately.

DesignCraft is an input program (`--designcraft PATH` or the
`DESIGNCRAFT_CLI` environment variable); nothing from it is in this
repository. Each IDML is rendered with `designcraft-cli run --in doc.idml
--cmd font.list --cmd preflight.run --export out.pdf`, which also reports
its font matches and preflight items.

### Same-save PDFs

A PDF beside an INDD file is not always an export of the INDD's last
save. A triple is measured when the PDF shows the same save as the INDD
(`pdf_stale_reasons` in `compare.py`) and the IDML pair is trustworthy
(the stale rule above, applied with the converter's output):

| Signal | Data used | Same save when |
|---|---|---|
| PDF names an INDD state | The PDF's `xmpMM:DerivedFrom` `stRef:instanceID` (`pdfinfo -meta`); the INDD's `xmpMM:InstanceID` and the `stEvt:instanceID` of its `xmpMM:History` (`indd xmp`) | It equals the INDD's InstanceID or one in its history |
| Dates | `xmp:ModifyDate` of the PDF and of the INDD | At most 1 hour apart |
| Pages | `pdfinfo` page count; `<Page>` elements in the IDML's spreads | Equal (exports of spreads or of a page range are left out) |
| Writer | `pdfinfo` Creator | Starts with `Adobe InDesign` |

Evidence, over 496 distinct triples of the corpus of 2026-10:

- The PDF's source instance is the INDD's current InstanceID in 42
  triples. It is a history entry 1 step back in 9 triples, 2 steps back in
  258 and 3 or more steps back in 103; the entries after it are saves
  (`stEvt:changed` `/metadata` or `/`). It is not in the history in 81
  triples, and 3 PDFs name no source.
- Where the instance is found, the ModifyDate gap has a median of 4
  seconds: 279 of 412 gaps are within 60 seconds and 312 within 1 hour.

The history entry alone accepts PDFs exported several saves before the
INDD's last save; the date limit removes those whose saves came later.

### Measures

Pages are rendered with `pdftoppm -r 50 -gray`, cropped to the page's
TrimBox (some InDesign PDFs have bleed and marks; DesignCraft's PDFs have
all boxes equal) and compared:

| Measure | Definition |
|---|---|
| Pixel-identical | The two page images have the same bytes |
| Mean difference | Mean absolute difference of the images after a 2 × 2 mean, from 0 to 1 |
| Missing ink, extra ink | Share of pixels with ink (value below 200) in one image and none within 2 pixels in the other, after the 2 × 2 mean |
| Lines | `pdftotext -bbox-layout` lines; a line's key is its text without white space, casefolded. Lines with the same key are matched, nearest first |
| Moved line | A matched line whose left edge or top differs by more than 1 pt. Between R and A the top is first corrected by a line-box offset (below) |
| Character overlap | Share of the characters of a page (without white space) that the other page also has, counted as a multiset |
| Font-complete | DesignCraft matches every font family of the document exactly (`font.list`); a page of another document is font-complete when every font `pdffonts` lists for the page in R belongs to such a family |

**Line-box offset (R vs A).** A line's box depends on the ascent and
descent each PDF declares for its fonts, so the same line can have a
different top in R and A without moving. Matched lines are grouped by
their pair of box heights (R, A). Each page corrects a group by the
median vertical difference of that group's lines on the page. A group
with fewer than 3 lines on the page takes the median of the group over
the whole document instead: with 3 lines the median is one of the lines
and one line that moved does not change it, while with 2 lines it is
their mean and a moved line shifts both. The per-page median keeps
pagination drift on other pages out of a page's correction. A shift
that most lines of a group share on one page is not counted as moved.

Evidence, on the same renders of 170 same-save documents (2026-10):
with the document's median alone, 17,066 of 42,932 matched lines count
as moved and 1,003 pages as `lines moved`; with the per-page median,
11,786 lines and 905 pages. On one page whose lines have the same
coordinates in two renderer builds, the document's median changed from
11.6 to 73.6 pt between the builds and 29 of 30 matched lines counted as
moved; with the page's median, none do.

**First differing page.** In A vs B, the first page that is not
pixel-identical is where the converter's output first changes the
rendering; later pages often only reflow. For that page the tool lists
compare.py's gap keys (and extra-value keys) among the elements that can
draw on it: the items of its spread and master spread, the stories of
their text frames, and the styles those apply, with their BasedOn chain.
Items and stories that can also draw on a pixel-identical page of the
document are left out, unless that leaves no key (`AB_gaps_scope` in
`docs.tsv`); styles are kept, because a style can change one page and
not another.
`summary.json` ranks these keys by documents. Next to each it gives the
number of pixel-identical documents that have the same gap anywhere (a
gap that many of them have is unlikely to change a rendering) and the
number of documents compare.py's `gaps.tsv` lists for the key, if that
file exists.

**DesignCraft issue kinds (R vs A).** A document counts for an issue
kind when at least one page or item has it:

| Issue | Rule |
|---|---|
| line breaks differ | A page with at least 20 characters and 3 lines in R, of which fewer than 80 % match a line of A |
| lines moved | Over 20 % of the matched lines moved |
| text missing | Character overlap below 0.9 |
| ink missing, ink extra | Over 2 % of the pixels. Most come from linked files missing from the corpus, which DesignCraft reports as `preflight: missingLink` |
| line past the column edge | A line of A that starts at least 50 pt left of a right edge shared by 3 or more lines of the page and ends 2–80 pt past it, and is not such a line in R |
| spread line | A line of A of 2–6 words whose gaps are all at least 0.8 × the line's box height (about 1 em) and that ends on such an edge, or two pieces on one line at least 3 such units apart that span a column; not such a line in R |
| story not drawn | A story in a single text frame whose first 24 characters R shows and A does not. Reported apart when the frame's inner height is under 1.05 × the first character's point size |
| preflight: KIND | An item of DesignCraft's preflight of A |
| page count differs, render failed, render timed out | As named |

Right-to-left lines are left out of the line-edge and spread checks.

### Running it

```sh
cargo build --release
python3 -I tools/render_compare.py --designcraft PATH/designcraft-cli
```

It needs poppler-utils (`pdfinfo`, `pdftoppm`, `pdftotext`, `pdffonts`)
and the Python standard library only. Options:

| Option | Effect |
|---|---|
| `--jobs N` | Triples processed in parallel (default: the number of CPUs) |
| `--limit N`, `--file SUBSTR` | Only the first N triples, or those whose INDD path contains SUBSTR |
| `--exclude PREFIX` | Leave out paths under `corpus/` that start with PREFIX (default `own/`) |
| `--versions 13-21` | INDD major versions measured (little-endian only). Other triples are listed but not measured |
| `--max-pages N` | Pages compared per document (default 100); the whole document is rendered |
| `--link-fonts` | Give DesignCraft the font files a package keeps beside the IDML, in a `Document Fonts` folder; DesignCraft reads only that folder |
| `--bin PATH` | Another converter binary |
| `--gaps PATH` | compare.py's `gaps.tsv` to rank first-difference keys against (default `target/compare/gaps.tsv`) |
| `--reuse DIR` | Take A.pdf, B.pdf and DesignCraft's results from `work/` of an earlier run in DIR (which may be `--out` itself) for each triple whose converted IDML has the same bytes as in that run; render the others, which needs `--designcraft`. For measuring a change to the tool on the same renders: pass the earlier run's `--bin` |
| `--images ID:PAGE…` | Write images of pages of the last run (below) and nothing else |

Run it after converter changes, like `diff_outputs.py`: a page whose A vs
B result changes is a fix or a regression, and the first differing page
locates it. Run it after DesignCraft updates for the R vs A issues.

### Outputs

In `target/render-compare/` (`--out DIR`). Documents are named by the
first 10 hexadecimal digits of the INDD's SHA-256; only `triples.tsv` and
`docs.tsv` give corpus paths.

| File | Content |
|---|---|
| `triples.tsv` | Every triple with an IDML and a PDF: verdict (same save, rejected, not measured) and reasons |
| `docs.tsv` | Per document: page counts, exit codes and times, fonts DesignCraft lacks, missing links, overset items in A and B, A vs B identical pages, first differing page with its gap keys, pages differing after it, DesignCraft issue kinds |
| `pages.tsv` | Per page: sizes, font-complete, R vs A and A vs B pixel and line measures, flags |
| `lines.tsv` | Lines past the column edge and spread lines in R, A and B, and whether R has the same line |
| `dropped.tsv` | Stories R shows and A does not, with frame height, point size, leading and fonts |
| `issues.tsv` | DesignCraft issue kinds per document, without paths, for reporting to DesignCraft |
| `summary.json` | Headline numbers: triples by verdict and reason; A vs B pixel-identical documents and pages, first-difference gap keys; DesignCraft renders, failures, times, issue kinds on font-complete documents and on all documents, and its warnings |
| `work/ID/` | The converter's IDML, `A.pdf`, `B.pdf`, the folders they were rendered from and DesignCraft's results (`run.json`) |
| `images/` | With `--images ID:PAGE`: `ID-pPAGE.png` (R, A and B side by side) and `ID-pPAGE-ab.png` (A vs B: red where only A has ink, blue where only B has it) |

### Limits

- Lines are not attributed to frames, so the line checks work on whole
  pages and also fire on tabbed and ragged layouts; counting only lines
  that R does not have removes most of those.
- Text extraction of right-to-left scripts follows `pdftotext`'s order,
  so line matching on Hebrew and Arabic is weaker.
- The gap keys of the first differing page are candidates. A change on a
  page can also come from a story that starts on an earlier page or from
  a value outside the elements listed above.
