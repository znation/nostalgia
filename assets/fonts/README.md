# Vendored fonts

The two openly-licensed faces the chrome embeds and renders its text in, from
part 1 of the app-text pixel-font plan. Both are SIL Open Font License 1.1
(see `OFL.txt`, which covers both files); embedding them in the binary is
what OFL permits, and shipping this directory with the repo keeps each
copyright and source record beside the bytes it names.

| File | Family name (iced `Font::with_name`) | Bytes | Source | Copyright |
|---|---|---|---|---|
| `Silkscreen-Regular.ttf` | `Silkscreen` | 32 KB | https://raw.githubusercontent.com/googlefonts/silkscreen/main/fonts/ttf/Silkscreen-Regular.ttf | Copyright 2001 The Silkscreen Project Authors (https://github.com/googlefonts/silkscreen) |
| `Tiny5.ttf` | `Tiny5` | 433 KB | https://raw.githubusercontent.com/google/fonts/main/ofl/tiny5/Tiny5%5BBLED%2CJITT%2CROND%2Cwdth%2Cwght%5D.ttf | Copyright 2022-2024 The Tiny5 Project Authors (https://github.com/Gissio/font_tiny5) |

`Tiny5.ttf` is the upstream variable font; it is registered and rendered at
its default Regular instance.

Both were vendored on 2026-10-30 as the pixel-font plan split's prerequisite,
after rendering each through the headless test renderer: the on-grid sizes
(multiples of 8 px) drew Silkscreen's letterforms in at most 4 distinct
colors and Tiny5's glyphs in exactly 2, and Tiny5 covers the shape glyphs
the chrome draws (`▶`, `◀`, `■`, `✕`, `–`, `II`, `□`, `≡`) that Silkscreen
lacks. Neither face covers `▭` or `⚡`; the plans that adopt the faces
substitute the covered `□` and `≡` for those two.
