# Resolution of the 52 QuickTeX compilation failures

All 185 pinned latextemplates documents now produce PDFs. The final corpus run
has **185 renders with compatibility warnings, zero hard failures, zero timeouts,
and zero pending jobs**. This is not full TeX parity: unsupported class/package
behavior and layout/content differences remain visible in the warnings and
comparisons. In particular, accepting the publisher styles below does not mean
executing their BST programs.

| Previous failures | Cause | Resolution |
| ---: | --- | --- |
| 15 | Undefined `topnumber`, `bottomnumber`, `totalnumber` | Implemented LaTeX float counters, including `dbltopnumber`, per-column limits, `!` overrides and `[H]` handling. End-of-document float-only pages make progress even with zero count limits. |
| 4 | Table-cell expansion corrupted the source-reader state | Restore the entire bounded reader after cell parsing, including when counter expansion replaces its buffer. The fast path borrows the cell span instead of copying every cell. Regression tests check expansion followed by more rows and document text. |
| 3 | Missing `\oiint` operator caused `\limits` to be misplaced | Implemented double/triple contour operators with normal scripts and limits, a shared vector contour, and SVG/PDF emission. Path-token separation has a regression test; the rebuilt PDF was also rasterized and inspected. |
| 28 | Unsupported publisher bibliography styles | Added partial IEEE, Springer/LNCS and ACM family renderers: citation ordering, author names/initials, protected sentence-case titles and common reference fields. Native BibTeX probes guided formatting. These implementations **retain compatibility warnings**; arbitrary `.bst` programs and publisher options are not implemented. Unknown styles still fail. |
| 2 | Blank LNI templates name nonexistent `mybibfile` without citing anything | Match the native LaTeX no-bibliography behavior, with a missing-resource warning. No references are invented. A missing database with citations, `\nocite{*}`, or potentially macro-generated citations still fails. |

## Verification

- 635 Rust tests pass, including float limits, reader restoration, contour
  geometry/path encoding, bibliography formatting and missing-resource behavior.
- 387 JavaScript/browser tests pass, including searchable PDF reference fields,
  explicit contour limits and missing-resource diagnostics.
- Type checks, production build and the unchanged WASM size gate pass.
- Core: 1,125,689 bytes; gzip: 598,752 bytes.
- All seven supported benchmark PDFs and formula-corpus SVGs are identical to
  the previous verified build (`dfe03191953b9048360c9a7cea4c296b9e85620eb6eb355673cb00ea04114e50`).
- Quiet warm and 21-pair fresh-process cold benchmarks pass the existing 5%
  timing-noise allowance. Cold medians include initialization and first PDF,
  not process startup. This does not establish a speed ranking against other
  compilers or fidelity for the upstream corpus.

The verified current binary hash and timing summary are in
`delivery-comparison.json`. Full current logs, raw benchmark samples, native
BibTeX probes and visual-check images are in `.work/parity-next/`.
See `templates-results.md` for all five compilers' diagnostic results: all
259 extracted-word comparisons still differ, and 120 pairs differ in page count.
