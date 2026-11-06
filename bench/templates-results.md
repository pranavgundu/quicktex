# Upstream template rendering

Pinned 18 repositories and 185 document roots in `templates-manifest.json` and `templates-generated-manifest.json` (when present). Generated projects use the upstream generator and snippet catalog.

These are diagnostic renders. “rendered” means the process finished and produced a valid PDF; it does not prove content or visual equivalence. Warnings count separately. Runs execute concurrently, so elapsed times are not a speed ranking. Engine-specific source requirements and locally available dependencies also affect the render counts.

| Compiler | Rendered | With warnings | Failed | Timeout | Pending |
| --- | ---: | ---: | ---: | ---: | ---: |
| quicktex | 0 | 185 | 0 | 0 | 0 |
| pdflatex | 100 | 0 | 79 | 6 | 0 |
| xelatex | 37 | 0 | 148 | 0 | 0 |
| lualatex | 95 | 0 | 84 | 6 | 0 |
| tectonic | 27 | 0 | 158 | 0 | 0 |

## Fidelity checks

Compared 259 QuickTeX/reference PDF pairs: 120 page-count mismatches and 259 extracted-word mismatches.

Word comparison uses lowercased token counts; it cannot validate ordering, equations, images, typography or layout. All pairs still require visual review. Per-pair metrics are in `.work/latextemplates/comparisons.json`.

Visual spot check: `scientific-thesis-template/chapters-en/10 - introduction.tex` has different fonts, margins, list spacing and unresolved reference text in QuickTeX; its 4 pages differ from pdfLaTeX’s 5. It is not compatible.


## Timeout diagnostics

These jobs emitted thousands of page markers before the process-group timeout. Their pagination behavior needs investigation; a longer timeout alone is not a useful remedy.

| Project | Compiler | Largest page marker |
| --- | --- | ---: |
| uni-stuttgart-dissertation-template/thesis-example-basic-listings-en.tex | pdflatex | 11273 |
| uni-stuttgart-dissertation-template/thesis-example-basic-listings-en.tex | lualatex | 6677 |
| uni-stuttgart-dissertation-template/thesis-example-basic-listings.tex | pdflatex | 11222 |
| generator-latex-template/matrix-03/thesis-example.tex | pdflatex | 9328 |
| generator-latex-template/matrix-03/thesis-example.tex | lualatex | 5603 |
| generator-latex-template/matrix-12/thesis-example.tex | pdflatex | 11693 |
| generator-latex-template/matrix-18/thesis-example.tex | pdflatex | 11533 |
| generator-latex-template/matrix-18/thesis-example.tex | lualatex | 6733 |
| generator-latex-template/matrix-23/thesis-example.tex | lualatex | 6628 |
| generator-latex-template/matrix-24/thesis-example.tex | lualatex | 6596 |
| generator-latex-template/matrix-25/thesis-example.tex | pdflatex | 10835 |
| generator-latex-template/matrix-25/thesis-example.tex | lualatex | 6539 |

## Repository coverage

| Repository | Roots |
| --- | ---: |
| [SAGP](https://github.com/latextemplates/SAGP/tree/c2f3476e3d57604ef3d7a8f689b7e8abfff18f11) | 1 |
| [scientific-thesis-template](https://github.com/latextemplates/scientific-thesis-template/tree/c6a348f9b74c452da71dd8f28e5001fcceaff5d2) | 28 |
| [uni-stuttgart-dissertation-template](https://github.com/latextemplates/uni-stuttgart-dissertation-template/tree/d017eb4cb08e82f0ed994a8a78fe8222dba3363c) | 22 |
| [scientific-thesis-cover](https://github.com/latextemplates/scientific-thesis-cover/tree/85b736fa37ce98d65785163d4bca2effa3c60274) | 8 |
| [uni-stuttgart-dissertation-cover](https://github.com/latextemplates/uni-stuttgart-dissertation-cover/tree/e9f584db2571911b09d5b5b8ebd9468d82998c2c) | 1 |
| [lncs-enhanced](https://github.com/latextemplates/lncs-enhanced/tree/a57931e26e634a71633d183c86ade42c73edc2ea) | 9 |
| [latextemplates.github.io](https://github.com/latextemplates/latextemplates.github.io/tree/e174c2b4f2e5570eaac174fad5fb88f403997195) | 0 |
| [LNI-archive](https://github.com/latextemplates/LNI-archive/tree/a1f6a4dc576d0173aab4545c470bd74f65dc6b8c) | 3 |
| [LNI-enhanced](https://github.com/latextemplates/LNI-enhanced/tree/61c7edc356683621ea0d58c4297b089a9e665f39) | 3 |
| [ieee-enhanced](https://github.com/latextemplates/ieee-enhanced/tree/652f45d11c9bc266761c25ea72ff6927df2eb39a) | 3 |
| [stys-for-overleaf](https://github.com/latextemplates/stys-for-overleaf/tree/2f62f4f2ffcaf9386f62d86c8a380934e98042ec) | 0 |
| [generator-latex-template](https://github.com/latextemplates/generator-latex-template/tree/04453b2ed35a6d85ab04c3064f07416ce0c3c3a5) | 0 |
| [Handin-LaTeX-template](https://github.com/latextemplates/Handin-LaTeX-template/tree/d29ed2182b30cf7fc8ed30f34875e22618558640) | 4 |
| [gadr-latex-packages](https://github.com/latextemplates/gadr-latex-packages/tree/6673b11c4e50c33672e4a26252efe4d86639cf07) | 32 |
| [svjour-enhanced](https://github.com/latextemplates/svjour-enhanced/tree/689ad94ce9145affab4b259e42a6fadbd3e945d8) | 1 |
| [acm-enhanced](https://github.com/latextemplates/acm-enhanced/tree/55f4c66e5a4d4421b615c1edc5e300211f704138) | 3 |
| [latex-snippets](https://github.com/latextemplates/latex-snippets/tree/ff90f46c3238a97f5f11666856930641aaac869a) | 0 |
| [markdown-latex-quickstart](https://github.com/latextemplates/markdown-latex-quickstart/tree/6596fd843b6613cbe82cb75a5a8d04c8b2332547) | 2 |
| [generator-latex-template/matrix-00](https://github.com/latextemplates/generator-latex-template/tree/04453b2ed35a6d85ab04c3064f07416ce0c3c3a5) | 1 |
| [generator-latex-template/matrix-01](https://github.com/latextemplates/generator-latex-template/tree/04453b2ed35a6d85ab04c3064f07416ce0c3c3a5) | 1 |
| [generator-latex-template/matrix-02](https://github.com/latextemplates/generator-latex-template/tree/04453b2ed35a6d85ab04c3064f07416ce0c3c3a5) | 1 |
| [generator-latex-template/matrix-03](https://github.com/latextemplates/generator-latex-template/tree/04453b2ed35a6d85ab04c3064f07416ce0c3c3a5) | 1 |
| [generator-latex-template/matrix-04](https://github.com/latextemplates/generator-latex-template/tree/04453b2ed35a6d85ab04c3064f07416ce0c3c3a5) | 1 |
| [generator-latex-template/matrix-05](https://github.com/latextemplates/generator-latex-template/tree/04453b2ed35a6d85ab04c3064f07416ce0c3c3a5) | 1 |
| [generator-latex-template/matrix-06](https://github.com/latextemplates/generator-latex-template/tree/04453b2ed35a6d85ab04c3064f07416ce0c3c3a5) | 1 |
| [generator-latex-template/matrix-07](https://github.com/latextemplates/generator-latex-template/tree/04453b2ed35a6d85ab04c3064f07416ce0c3c3a5) | 1 |
| [generator-latex-template/matrix-08](https://github.com/latextemplates/generator-latex-template/tree/04453b2ed35a6d85ab04c3064f07416ce0c3c3a5) | 1 |
| [generator-latex-template/matrix-09](https://github.com/latextemplates/generator-latex-template/tree/04453b2ed35a6d85ab04c3064f07416ce0c3c3a5) | 1 |
| [generator-latex-template/matrix-10](https://github.com/latextemplates/generator-latex-template/tree/04453b2ed35a6d85ab04c3064f07416ce0c3c3a5) | 1 |
| [generator-latex-template/matrix-11](https://github.com/latextemplates/generator-latex-template/tree/04453b2ed35a6d85ab04c3064f07416ce0c3c3a5) | 1 |
| [generator-latex-template/matrix-12](https://github.com/latextemplates/generator-latex-template/tree/04453b2ed35a6d85ab04c3064f07416ce0c3c3a5) | 1 |
| [generator-latex-template/matrix-13](https://github.com/latextemplates/generator-latex-template/tree/04453b2ed35a6d85ab04c3064f07416ce0c3c3a5) | 1 |
| [generator-latex-template/matrix-14](https://github.com/latextemplates/generator-latex-template/tree/04453b2ed35a6d85ab04c3064f07416ce0c3c3a5) | 1 |
| [generator-latex-template/matrix-15](https://github.com/latextemplates/generator-latex-template/tree/04453b2ed35a6d85ab04c3064f07416ce0c3c3a5) | 1 |
| [generator-latex-template/matrix-16](https://github.com/latextemplates/generator-latex-template/tree/04453b2ed35a6d85ab04c3064f07416ce0c3c3a5) | 1 |
| [generator-latex-template/matrix-17](https://github.com/latextemplates/generator-latex-template/tree/04453b2ed35a6d85ab04c3064f07416ce0c3c3a5) | 1 |
| [generator-latex-template/matrix-18](https://github.com/latextemplates/generator-latex-template/tree/04453b2ed35a6d85ab04c3064f07416ce0c3c3a5) | 1 |
| [generator-latex-template/matrix-19](https://github.com/latextemplates/generator-latex-template/tree/04453b2ed35a6d85ab04c3064f07416ce0c3c3a5) | 1 |
| [generator-latex-template/matrix-20](https://github.com/latextemplates/generator-latex-template/tree/04453b2ed35a6d85ab04c3064f07416ce0c3c3a5) | 1 |
| [generator-latex-template/matrix-21](https://github.com/latextemplates/generator-latex-template/tree/04453b2ed35a6d85ab04c3064f07416ce0c3c3a5) | 1 |
| [generator-latex-template/matrix-22](https://github.com/latextemplates/generator-latex-template/tree/04453b2ed35a6d85ab04c3064f07416ce0c3c3a5) | 1 |
| [generator-latex-template/matrix-23](https://github.com/latextemplates/generator-latex-template/tree/04453b2ed35a6d85ab04c3064f07416ce0c3c3a5) | 1 |
| [generator-latex-template/matrix-24](https://github.com/latextemplates/generator-latex-template/tree/04453b2ed35a6d85ab04c3064f07416ce0c3c3a5) | 1 |
| [generator-latex-template/matrix-25](https://github.com/latextemplates/generator-latex-template/tree/04453b2ed35a6d85ab04c3064f07416ce0c3c3a5) | 1 |
| [generator-latex-template/matrix-26](https://github.com/latextemplates/generator-latex-template/tree/04453b2ed35a6d85ab04c3064f07416ce0c3c3a5) | 1 |
| [generator-latex-template/matrix-27](https://github.com/latextemplates/generator-latex-template/tree/04453b2ed35a6d85ab04c3064f07416ce0c3c3a5) | 1 |
| [latex-snippets/minted-en](https://github.com/latextemplates/latex-snippets/tree/ff90f46c3238a97f5f11666856930641aaac869a) | 1 |
| [latex-snippets/listings-en](https://github.com/latextemplates/latex-snippets/tree/ff90f46c3238a97f5f11666856930641aaac869a) | 1 |
| [latex-snippets/todonotes-en](https://github.com/latextemplates/latex-snippets/tree/ff90f46c3238a97f5f11666856930641aaac869a) | 1 |
| [latex-snippets/pdfcomment-en](https://github.com/latextemplates/latex-snippets/tree/ff90f46c3238a97f5f11666856930641aaac869a) | 1 |
| [latex-snippets/zref-clever-en](https://github.com/latextemplates/latex-snippets/tree/ff90f46c3238a97f5f11666856930641aaac869a) | 1 |
| [latex-snippets/cleveref-en](https://github.com/latextemplates/latex-snippets/tree/ff90f46c3238a97f5f11666856930641aaac869a) | 1 |
| [latex-snippets/siunitx-en](https://github.com/latextemplates/latex-snippets/tree/ff90f46c3238a97f5f11666856930641aaac869a) | 1 |
| [latex-snippets/csquotes-en](https://github.com/latextemplates/latex-snippets/tree/ff90f46c3238a97f5f11666856930641aaac869a) | 1 |
| [latex-snippets/outerquote-en](https://github.com/latextemplates/latex-snippets/tree/ff90f46c3238a97f5f11666856930641aaac869a) | 1 |
| [latex-snippets/paralist-en](https://github.com/latextemplates/latex-snippets/tree/ff90f46c3238a97f5f11666856930641aaac869a) | 1 |
| [latex-snippets/diagbox-en](https://github.com/latextemplates/latex-snippets/tree/ff90f46c3238a97f5f11666856930641aaac869a) | 1 |
| [latex-snippets/mindflow-en](https://github.com/latextemplates/latex-snippets/tree/ff90f46c3238a97f5f11666856930641aaac869a) | 1 |
| [latex-snippets/textcmds-en](https://github.com/latextemplates/latex-snippets/tree/ff90f46c3238a97f5f11666856930641aaac869a) | 1 |
| [latex-snippets/booktabs-en](https://github.com/latextemplates/latex-snippets/tree/ff90f46c3238a97f5f11666856930641aaac869a) | 1 |
| [latex-snippets/paragraphs-en](https://github.com/latextemplates/latex-snippets/tree/ff90f46c3238a97f5f11666856930641aaac869a) | 1 |
| [latex-snippets/hyphenation-en](https://github.com/latextemplates/latex-snippets/tree/ff90f46c3238a97f5f11666856930641aaac869a) | 1 |
| [latex-snippets/abbreviations-en](https://github.com/latextemplates/latex-snippets/tree/ff90f46c3238a97f5f11666856930641aaac869a) | 1 |
| [latex-snippets/minted-de](https://github.com/latextemplates/latex-snippets/tree/ff90f46c3238a97f5f11666856930641aaac869a) | 1 |
| [latex-snippets/listings-de](https://github.com/latextemplates/latex-snippets/tree/ff90f46c3238a97f5f11666856930641aaac869a) | 1 |
| [latex-snippets/todonotes-de](https://github.com/latextemplates/latex-snippets/tree/ff90f46c3238a97f5f11666856930641aaac869a) | 1 |
| [latex-snippets/pdfcomment-de](https://github.com/latextemplates/latex-snippets/tree/ff90f46c3238a97f5f11666856930641aaac869a) | 1 |
| [latex-snippets/zref-clever-de](https://github.com/latextemplates/latex-snippets/tree/ff90f46c3238a97f5f11666856930641aaac869a) | 1 |
| [latex-snippets/cleveref-de](https://github.com/latextemplates/latex-snippets/tree/ff90f46c3238a97f5f11666856930641aaac869a) | 1 |
| [latex-snippets/siunitx-de](https://github.com/latextemplates/latex-snippets/tree/ff90f46c3238a97f5f11666856930641aaac869a) | 1 |
| [latex-snippets/csquotes-de](https://github.com/latextemplates/latex-snippets/tree/ff90f46c3238a97f5f11666856930641aaac869a) | 1 |
| [latex-snippets/outerquote-de](https://github.com/latextemplates/latex-snippets/tree/ff90f46c3238a97f5f11666856930641aaac869a) | 1 |
| [latex-snippets/paralist-de](https://github.com/latextemplates/latex-snippets/tree/ff90f46c3238a97f5f11666856930641aaac869a) | 1 |
| [latex-snippets/diagbox-de](https://github.com/latextemplates/latex-snippets/tree/ff90f46c3238a97f5f11666856930641aaac869a) | 1 |
| [latex-snippets/mindflow-de](https://github.com/latextemplates/latex-snippets/tree/ff90f46c3238a97f5f11666856930641aaac869a) | 1 |
| [latex-snippets/textcmds-de](https://github.com/latextemplates/latex-snippets/tree/ff90f46c3238a97f5f11666856930641aaac869a) | 1 |
| [latex-snippets/booktabs-de](https://github.com/latextemplates/latex-snippets/tree/ff90f46c3238a97f5f11666856930641aaac869a) | 1 |
| [latex-snippets/paragraphs-de](https://github.com/latextemplates/latex-snippets/tree/ff90f46c3238a97f5f11666856930641aaac869a) | 1 |
| [latex-snippets/hyphenation-de](https://github.com/latextemplates/latex-snippets/tree/ff90f46c3238a97f5f11666856930641aaac869a) | 1 |
| [latex-snippets/abbreviations-de](https://github.com/latextemplates/latex-snippets/tree/ff90f46c3238a97f5f11666856930641aaac869a) | 1 |
| [stys-for-overleaf/class-smoke](https://github.com/latextemplates/stys-for-overleaf/tree/2f62f4f2ffcaf9386f62d86c8a380934e98042ec) | 1 |
| [Handin-LaTeX-template/distribution](https://github.com/latextemplates/Handin-LaTeX-template/tree/d29ed2182b30cf7fc8ed30f34875e22618558640) | 2 |

## QuickTeX errors


Full per-document results, PDFs, extracted text and logs are in `bench/.work/latextemplates/`. Raw EJS source templates are inventoried separately; generator outputs, snippet context documents and the class smoke test are listed as generated projects. See `materialization.json` for preamble-only catalog entries that have no example to typeset.
