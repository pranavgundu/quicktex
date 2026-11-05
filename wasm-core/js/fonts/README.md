# Optional document fallback fonts

These assets load only when `loadFontsFor` encounters the corresponding scripts.
They and the separate `quicktex-fonts.wasm` parser are not part of the initial page download.

- `NotoSerif-Regular.ttf`: https://github.com/notofonts/noto-fonts/blob/main/hinted/ttf/NotoSerif/NotoSerif-Regular.ttf (SIL OFL, see `OFL.txt`). Used for Greek and Cyrillic.
- `NotoSerifCJKsc-Regular.otf`: https://github.com/notofonts/noto-cjk/blob/main/Serif/OTF/SimplifiedChinese/NotoSerifCJKsc-Regular.otf (SIL OFL, see `OFL-CJK.txt`). Includes Chinese ideographs, Japanese kana, and Korean syllables. Shared ideographs use the SC face's forms.

Downloaded 2026-11-05. Outlines are parsed with ttf-parser and cached only for used characters; generated PDFs embed those characters, not the complete font assets.
