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
