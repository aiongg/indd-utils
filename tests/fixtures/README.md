# Test fixtures

Every file here is redistributable. Nothing else may be added to this
directory: files without a licence that allows redistribution belong in the
local, git-ignored `corpus/` directory.

| Directory | File | InDesign version | Licence | Source |
|---|---|---|---|---|
| `opf-neddy-flyer` | `Neddy_Flyer_HeatherRyan.indd`, `.pdf` | 3.0 (big-endian) | CC0, Heather Ryan (waiver in the `.rtf` README) | [openpreserve/format-corpus](https://github.com/openpreserve/format-corpus/tree/master/desktop-publishing/InDesign) |
| `lizdenys-minizine` | `indesign-minizine-template.indd` | 20.3 | CC0, Liz Denys | [lizdenys.com](https://lizdenys.com/journal/articles/indesign-minizine-template.html) |
| `bootstrap3-template` | `bootstrap3-indesign-template.indd` | 9.2 | MIT, © 2014 Miix (see `LICENSE`) | [jeffing/bootstrap3-indesign-template](https://github.com/jeffing/bootstrap3-indesign-template) @ `59a7d9fbe360` |
| `scml-template` | `scml.indt` | 7.5 | MIT, © 2014 Scribe, Inc. (see `LICENSE`) | [scribenet/scr-scml-indesign-templates](https://github.com/scribenet/scr-scml-indesign-templates) @ `556f09acc5ab` |

None of these has a matching IDML file. Use them for parser smoke tests:
the file opens, the header is read, and the converter runs without error.
