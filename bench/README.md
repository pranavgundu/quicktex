# Benchmarks

How long quicktex takes against other LaTeX compilers, with no editor involved. Numbers are in [results.md](results.md).

```sh
cd bench && bun install && cd ..
bun run wasm
bun run bench
```

`bun run bench note paper` runs only those documents. `BENCH_SECONDS=5 bun run bench` shortens each measurement. A full run takes about 15 minutes. Compilers that are not installed are skipped.

You need `hyperfine`, `pdfinfo` from Poppler, and any of `pdflatex`, `xelatex`, `lualatex`, `latexmk` and `tectonic`.

## What is measured

Every compiler gets the same `.tex` file and has to leave a finished PDF on disk.

- quicktex runs from a fresh process, the same as the others. The time covers starting Bun, loading the wasm, compiling, building the PDF, and writing it.
- `hyperfine` does the timing, with one warmup run, and the table shows the median. Generated files are deleted before each run, so nothing is reused.
- Documents with references or a table of contents use `latexmk` for `pdflatex`, `xelatex` and `lualatex`, because one pass leaves `??` in the output. Tectonic reruns on its own, and quicktex resolves references inside one compile.
- A run stops with an error if quicktex skipped any command in a document, so it cannot win by leaving work out.
- Tectonic's package downloads happen before timing starts.

The formula table compares libraries that turn one formula into markup: KaTeX, MathJax and Temml. It runs in Node, on the formulas in `tests/support/corpus.ts` that all four accept.

## What this does not show

quicktex is faster partly because it does less. These are real differences, not fine print.

- It handles a subset of LaTeX. Most real documents with custom classes, `\input` files or images will not come out the same, and any of these compilers will handle them better. The documents here use only what quicktex supports.
- It is not TeX. There is no hyphenation or microtype, line breaks differ, and displayed equations sit further apart, which is why `formulas` runs to more pages.
- Footnotes print under their paragraph, and plots use their own tick spacing.
- Text in its PDF is drawn as shapes, so it cannot be selected or searched, and files come out far larger. See the PDF size table.
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
