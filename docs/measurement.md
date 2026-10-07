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
| INDD stories not in the IDML | The INDD's story list, as converted; the IDML's stories | The INDD has a story that the IDML does not |

Pairs whose IDML `DOMVersion` has a different major version from the INDD
header are not compared at all.

### Evidence

**IDML UIDs.** In 631 of 654 pairs every `Self="u…"` in the IDML is a UID
in the INDD database. In the other 23, between 1 and 1,361 IDML UIDs are
missing from the INDD, so the IDML describes objects that the INDD no
longer (or never) had.

**ModifyDate.** InDesign writes `xmp:ModifyDate` both when it saves an INDD
and when it exports an IDML. A document exported and then saved within a few
minutes (as when packaging) has a small positive gap. The table counts
pairs by gap (INDD date minus IDML date), split by whether their content
visibly differs: the presence signals above fire, or a story's text is
less than 90 % similar to the converter's text for the same story
(`difflib` ratio, ignoring U+FEFF, which the converter writes for
anchors).

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

The rule flags 159 of 654 pairs; 495 are trustworthy.

| Group | Pairs | Stories with identical text | Pairs with a differing story |
|---|---:|---:|---:|
| Trustworthy | 495 | 18,823 of 18,929 (99.4 %) | 49 (10 %) |
| Stale by a presence signal | 33 | 724 of 1,209 (59.9 %) | 22 (67 %) |
| Stale by ModifyDate only (INDD over 1 hour later) | 125 | 5,626 of 5,802 (97.0 %) | 40 (32 %) |
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
| Text range | Start of a paragraph and character style range | The output has a range starting at the same offset |
| Text range attribute | Attribute or `Properties` child of the range | Equal |

When an element is not produced, its presence and all its values count as
not reproduced. When a story's text differs, its text ranges cannot be
lined up, so all their values count as not reproduced.

**Value coverage** is reproduced values divided by all values, summed over
pairs, so each value counts once and large documents weigh more. Per-pair
coverage is the same ratio within one pair. `target/compare/pairs.tsv`
lists every pair with its coverage and stale reasons (`--out DIR` changes
the directory).

**Document score** is the share of trustworthy pairs whose coverage is at
least 99 %, and at least 99.9 %.

### Values left out

Some values describe the computer that exported the IDML or the IDML
package itself, not the document. No INDD file can supply them, so they
are left out of the counts (`EXCLUDED` in `compare.py`).

| Element | Value | Why it cannot come from the INDD |
|---|---|---|
| `Font` | `Status` | It records whether the font is installed on the computer that exported the IDML. The same document gives a different value on another computer. |
| `idPkg:Story`, `idPkg:Spread` and the other part references in `designmap.xml` | `src` | The file names of the parts inside the IDML package. The IDML writer chooses them; they are not document content. |
| `DocumentUser` | `UserColor` | The colour of each user of the document as the exporting InDesign shows it. The INDD stores a colour for each user (`objects.md`, document users), but the IDML colour does not follow from it: users stored with the same colour get different IDML colours in different files. |

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

`values.tsv` and `values-all.tsv` list every key with its number of
values and how many are reproduced, wrong and missing. Comparing them
between two runs shows whether a change reproduces fewer values of any
key.

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
