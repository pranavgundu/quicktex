# Benchmarks

How long quicktex takes against other LaTeX compilers, with no editor involved. Numbers are in [results.md](results.md).

```sh
cd bench && bun install && cd ..
bun run wasm
bun run bench
```

`bun run bench note paper` runs only those documents. `BENCH_SECONDS=5 bun run bench` shortens each measurement. A full run takes about 15 minutes. Compilers that are not installed are skipped. `BENCH_TOOLS=quicktex bun run bench` remeasures quicktex across every document while preserving the other compilers’ results.

You need `hyperfine`, `pdfinfo` from Poppler, and any of `pdflatex`, `xelatex`, `lualatex`, `latexmk` and `tectonic`.

## What is measured

Every compiler gets the same `.tex` file and has to leave a finished PDF on disk.

- quicktex runs from a fresh process, the same as the others. The time covers starting Bun, loading the wasm, compiling, building the PDF, and writing it.
- `hyperfine` does the timing, with one warmup run, and the table shows the median. Generated files are deleted before each run, so nothing is reused.
- Documents with references or a table of contents use `latexmk` for `pdflatex`, `xelatex` and `lualatex`, because one pass leaves `??` in the output. Tectonic reruns on its own, and quicktex resolves references inside one compile.
- A run stops with an error if quicktex reports a warning. This catches diagnosed unsupported features, but cannot prove output completeness or equivalence: missing diagnostics and layout bugs can still leave work out. Compare the actual PDFs as well as timings.
- Tectonic's package downloads happen before timing starts.

The formula table compares libraries that turn one formula into markup: KaTeX, MathJax and Temml. It runs in Node, on the formulas in `tests/support/corpus.ts` that all four accept.

## What this does not show

quicktex is faster partly because it does less. These are real differences, not fine print.

- It handles a subset of LaTeX. Most real documents with custom classes or unsupported packages will not come out the same, and any of these compilers will handle them better. The documents here use only what quicktex supports.
- It is not TeX. It uses English hyphenation patterns but has no microtype. Paragraphs break with Knuth-Plass and pages with TeX's costs and penalties, using the real Computer Modern widths, kerns and glue at every size, but the page builder looks at one line at a time, so it can still differ near headings and displays, and page counts can differ.
- Letter shapes are not all TeX's. Every size is drawn from the 10pt outlines, in the position and width of the font file LaTeX would load, and `fi`, `fl`, `ff`, `ffi` and `ffl` are set at their ligature widths but drawn as separate letters.
- Footnotes reserve space at the column bottom; plots use their own tick spacing.
- PDF text uses embedded outline fonts with Unicode maps and can be selected and searched. Font encoding and compression differ from TeX; see the PDF size table.
- It can use more memory than `pdflatex` on long documents. See the peak memory table.
- TeX loads its format and packages on every run, so a fixed start-up cost makes up much of its time on small documents. The gap is widest on drawings, where TikZ and pgfplots run as TeX macros.
- Most of quicktex's fresh process time is starting Bun and loading the wasm, not compiling. Compare the table of already loaded times.
- `latexmk` adds a little time of its own on top of the passes it runs.
- Tectonic has a fixed memory limit and stops with `TeX capacity exceeded` on the fractal at 8,191 branches, so `tree` uses 4,095, which every compiler finishes.

Timings come from one machine. Compare the ratios, not the milliseconds.

## Files

| File | Purpose |
| --- | --- |
| `run.ts` | Runs every compiler on every document and writes `results.json` |
| `documents.ts` | The benchmark documents |
| `compile.ts` | Command line quicktex: `.tex` in, `.pdf` out |
| `math.mjs` | The formula comparison |
| `report.ts` | Turns `results.json` into `results.md` |
| `comparison.json` | Historical before/after measurements from the initial implementation pass |
| `compare.ts` | Alternates a preserved baseline and candidate WASM to detect regressions |
| `compatibility-comparison.json` | Before/after timing samples and output checks for the compatibility fixes |

## Checking pagination against LaTeX

Run `bun run test:parity` (or `bun run test:parity paper formulas`) with
`pdflatex` and Poppler's `pdfinfo` installed. This compiles the same sources
with both engines, resolves TeX references over three passes where needed,
and exits with an error for any page-count mismatch or QuickTeX warning.
Comparison PDFs and a JSON report are saved under `.work/parity/`.

This is a compatibility check, separate from timing. Equal page counts do
not establish equal content, line breaks, or visual output. The current
engine still has known pagination differences; the command exposes them
rather than accepting them as a successful comparison.

## Comparing an implementation change

Preserve the old `wasm-core/js/quicktex.wasm` before rebuilding, then run:

```sh
bun bench/compare.ts /path/to/baseline.wasm
```

This alternates both builds in the same process over 21 batches per document and across the formula corpus and records layout plus PDF timings in `compatibility-comparison.json`. It fails if an existing benchmark gains warnings, changes page count or PDF bytes, or slows down by more than the default 5% timing-noise allowance (`BENCH_TOLERANCE` changes that fraction). This is a regression check on the existing corpus, not a substitute for feature-specific correctness tests or PDF review. Run `bun run bench` as well for fresh-process time and memory.

## Source-changing edit performance and caches

```sh
bun bench/compare-edits.ts /path/to/baseline.wasm wasm-core/js/quicktex.wasm bench/edit-comparison.json
```

This alternates two builds across eight actual source edits per scenario,
uploading each project's files once. It measures layout only (not PDF export)
and checks all eight PDF fingerprints outside the timed samples. It includes
text documents, a 768×512 RGBA PNG project and a 300-entry bibliography. It is
not a benchmark of repeated identical-source layout-cache hits.

The current profile-guided pass adds a bounded ASCII metric cache, reuses
paragraph line-break scratch, caches decoded images (32-entry/16 MiB LRU), and
avoids rescanning unchanged project text for fallback glyphs. Images invalidate
on project replacement/registration; font, scale and document point size are
part of metric keys; project glyph preparation invalidates when fonts change.
Custom Unicode outlines are not stored in the immutable ASCII metric cache.

`cache-comparison.json` records the median of three quiet trials: paper/book
edits are about 14% faster, and the image edit scenario falls from 7.61 ms to
0.004 ms once the image cache is warm. First image decoding is still required.
The first trial's bibliography timing failed the 5% gate; both repeat trials
were faster, and the artifact retains all samples including that outlier. The
compile-plus-PDF benchmarks improve about 12–14% for paper/book, while cold book
initialization plus first PDF improves about 11%. All 185 upstream PDFs remain
byte-identical. These measurements do not establish fidelity or a speed ranking
against other compilers. Full raw samples and the native CPU profile are in
`.work/perf-cache/`.

For native CPU sampling, an opt-in ignored Rust test accepts a document path:

```sh
QT_PROFILE_SOURCE=/path/to/document.tex QT_PROFILE_RUNS=200 sh wasm-core/test.sh profile_document -- --ignored --nocapture
```

It is intentionally excluded from ordinary test runs. Run sampling separately
from performance comparisons and browser tests.

## Upstream latextemplates corpus

`templates-manifest.json` pins all 18 repositories and the detected `.tex`
roots containing an uncommented `\documentclass` or `\documentstyle`.
This includes standalone subfiles. Raw EJS templates are inventoried as
`sourceTemplates` rather than compiled as documents. The materializer adds the
upstream generator’s 26 pairwise configurations, both scientific-thesis options,
34 bilingual snippet examples, and a labelled smoke document for the class-only
`stys-for-overleaf` repository. It also extracts Handin’s packaged ZIP so its
stylesheet and logo accompany the two distributed documents. Generated inputs and their content hashes are
recorded in `templates-generated-manifest.json`. No sources are simplified to make QuickTeX accept them.

```sh
python3 bench/templates.py --prepare --report-only
node bench/materialize-templates.mjs
python3 bench/templates.py --resume
# Rerun one engine or project:
python3 bench/templates.py --tools quicktex --repositories lncs-enhanced --resume
# Rebuild the summary from completed jobs during a running comparison:
python3 bench/templates.py --report-only
# Explicitly update the upstream source revisions:
python3 bench/templates.py --prepare --refresh --resume
```

The runner uses latexmk to finish pdfLaTeX, XeLaTeX and LuaLaTeX documents,
including bibliography/reference passes; Tectonic handles its own reruns.
Each job gets a fresh project copy and deletes its target PDF before running.
Timeouts terminate the compiler process group. Per-job JSON, PDFs, extracted
text and logs live in `.work/latextemplates/`; `templates-results.md` summarizes
completed jobs, failures and pending work. An emitted PDF is only a diagnostic
render, and QuickTeX PDFs with compatibility warnings are counted separately.
Visual/content comparison is still required before declaring compatibility.
Runs use concurrent workers, so their elapsed times cannot establish a speed
ranking. Use the existing hyperfine benchmark after fidelity is established.

The runner searches `.work/latextemplates/bin` before the installed TeX tools
for optional local tool repairs. On this machine the universal Biber launcher
fails when invoking `lipo -extract_family`; an ARM-only workspace copy created
with `lipo /Library/TeX/texbin/biber -thin arm64 -o bench/.work/latextemplates/bin/biber`
works. The installed tool is unchanged. Completed results are reused only when
the source revision, compiler binary and runner dependencies match.

The materializer requires Node, npm, `patch`, `tar` and `unzip`; it installs the generators’
locked dependencies with lifecycle scripts disabled, applies their supplied
compatibility patch, and invokes the generators directly. It resolves the
snippet repository’s exact generator gitlink and uses the upstream builder’s
own EJS rendering and context-document wrapper. It does not run Docker, release
scripts, or Git commands. Eight catalog entries (four packages in two locales)
contain preamble configuration only and have no example to typeset; these are
explicit in `.work/latextemplates/materialization.json`. Generator dependencies
are removed after materialization so the pinned source trees remain clean.

`--only-pending` preserves completed jobs matching the source revision, generated
content hash and compiler binary, even if the runner changed. Use it only after
reviewing those runner changes; ordinary `--resume` also invalidates results when
the runner dependencies change.

## Delivery-size check

`delivery-comparison.json` records the preserved build and the current build
using a 64 KiB initial outline block, 256 KiB subsequent blocks, and an LLVM
inlining threshold of 150.
All original outline bytes are retained. The outline blocks now use zlib's
`Z_FILTERED` strategy (level 9, memory level 9), with round-trip assertions in
`wasm-core/assets/build.py`. Current core gzip size is 599,967 bytes (598,752
before caching; 593,901 before the 52-failure resolution pass; 626,152 before
font compression work),
within the unchanged 601,000-byte test budget. Regenerating the assets is
byte-for-byte reproducible in the verified environment. The seven benchmark
PDFs and formula-corpus SVGs are identical between those builds.

The quiet warm comparison passes the 5% timing-noise allowance for every
benchmark document and the formula corpus. Sampled cold initialization plus
first-PDF times also stay within 5% for note, paper and book, with 21 alternating
engine pairs per document in fresh processes. Those times exclude Bun process
startup. Earlier noisy trials and the earlier block layouts are retained
separately. These checks establish no regression on the supported benchmark
corpus, not upstream template compatibility.

The verified build passes 639 Rust tests (plus one intentionally ignored native
profiling harness), 389 app/browser tests, type checks,
and the production build. Its current corpus results are 185 PDFs with
compatibility warnings, zero failures, zero timeouts and no pending QuickTeX
jobs. No upstream document is warning-free. `failure-resolution.md` explains
how the previous 52 failures were addressed, including the limits of the
partial publisher bibliography formatters. See `templates-results.md` for the reference
engine results and remaining errors; `delivery-comparison.json` records the
verified binary hash and performance evidence. Full logs and raw samples are
in `.work/perf-cache/`; previous correctness probes remain in
`.work/parity-next/` and `.work/final-review/`.

To reproduce the cold check without the temporary workspace scripts:

```sh
bun bench/compare-cold.ts /path/to/baseline.wasm wasm-core/js/quicktex.wasm bench/cold-comparison.json
```

This launches 21 alternating fresh-process pairs for note, paper and book,
checks PDF fingerprints and warnings on every sample, and applies the same
`BENCH_TOLERANCE` to median initialization-plus-first-PDF time. It does not
measure process startup. Run both timing checks without concurrent tests or
reference compilers.

The corpus manifest records repository-root working directories for comparison
papers that reference `comparison/body.tex`. The worker resolves uploaded files
relative to the actual compiler working directory. Tectonic receives an explicit
output directory and search path; templates needing shell escape use its own
`-Z shell-escape-cwd` option. Previous results with different build recipes are
excluded from the report until rerun.

The workspace also contains a `bin/latexminted` launcher that executes the
installed tool with Python 3.13. The installed latexminted 0.6.0 crashes with
Python 3.14 because its argument-parser subclass does not accept the new
`color` argument. The wrapper leaves the installed program unchanged. After
repairing a dependency, `--only-pending --retry-log 'Package minted Error'`
reruns failed jobs with matching compiler logs while preserving other completed
results. Logs from timed-out thesis examples contain thousands of page markers;
the report exposes this pagination behavior instead of assuming a longer
timeout will fix it.
