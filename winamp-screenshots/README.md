# Classic Winamp reference screenshots

Ground-truth captures of the real Winamp UI, kept so every change to Nostalgia's chrome can be
compared against the thing it clones. Nostalgia targets the **classic Winamp 2.x base skin**:
the 275×116 main window with its steel-blue chrome, green LCD readouts, beveled transport
buttons, marquee title, and the dockable equalizer and playlist editor. The 0.x and 1.x captures
show the lineage that chrome grew out of; the 2.95 captures show the last of the classic line.

**When a change touches anything visible — a window, a panel, a button, a readout, a color —
compare the result against the closest capture here and match its chrome, palette, bevels,
layout, and typography.** `docs/screenshots/` holds Nostalgia's own captures (the README's
mockups); this directory holds the target they are measured against.

These are screenshots of proprietary software (Winamp, © Nullsoft / AOL / Llama Group), kept as
visual reference only; they are **not** covered by the repository's MIT license.

## Index

| File | Version | Shows |
| --- | --- | --- |
| `winamp-0.2a-main-1997.jpg` | 0.2a (1997) | The first released prototype: a plain Windows window with an `MP3`/`Help` menu bar (the `MP3` menu is open) and a blank content area — no skinned chrome yet. |
| `winamp-0.92-main-1997.jpg` | 0.92 (1997) | Still a standard Windows frame, but the green-LED player is there: `SONG` / `BITRATE 128 kbps` / `MIXRATE 44 kHz` / `VOLUME` / `BALANCE` / `MONO-STERO`, transport row, shuffle and repeat, llama-bolt logo. Playing Tasmin Archer – Sleeping Satellite at 00:12. |
| `winamp-1.006-main-1997.png` | 1.006 (1997) | The skinned main window in its 1.x form: left-hand LED display, spelled-out `SONG`/`BITRATE`/`MIXRATE` labels, stacked volume and balance, no seek bar, no EQ/PL buttons. Track `1. CRUSHERP - ECHO`. |
| `winamp-2.05-main.jpg` | 2.05 (1998) | The classic main window alone, at native 275×116: spectrum analyzer, marquee `16. RADIOHEAD - CREEP <3:56>`, `128 kbps 44 kHz`, `02:58`, Shuffle lit. |
| `winamp-2.05-docked-main-eq-playlist.gif` | 2.05 (1998) | Main + equalizer + playlist docked into one block, as the official overview page showed them: `APOLLO 440 - OMEGA POINT`, an 18-row playlist, flat EQ. |
| `winamp-2.05-windowshade.gif` | 2.05 (1998) | Windowshade (rolled-up) mode: the main player and the playlist editor as two 14px strips docked side by side, showing the marquee `14. APOLLO FOUR FORTY - RAW POWER`, time `3:59`, volume, and the shade close buttons. |
| `winamp-2.1-main-eq.gif` | 2.1 (1999) | Main window + equalizer docked, EQ `ON` and `AUTO` lit with the preamp at +3 dB and all ten bands boosted into a uniform curve. Track `1. [SMASHING PUMPKINS] - MAYONAISE`, `00:52`. |
| `winamp-2.1-stack-with-minibrowser.gif` | 2.1 (1999) | The full 2.1 stack: main + equalizer + playlist + the new Minibrowser (showing an ad page), a playlist of Wesley Willis tracks. |
| `winamp-2.x-stack.png` | 2.x (2.9x-era) | Main + equalizer + playlist stacked vertically with no gaps, EQ `ON`/`AUTO` lit, Shuffle lit, `3. CIRCUSP - GOODBYE` at `0:01:23`. |
| `winamp-2.95-stack.png` | 2.95 (2003) | The canonical 2.95 press crop: main + equalizer + playlist, playing the Nullsoft demo track `DJ Mike Llama - Llama Whippin' Intro` (0:05), Play pressed, flat EQ. |
| `winamp-2.95-docked-on-desktop.png` | 2.95 (2003) | The same era in situ: Winamp 2.95 playing on a Windows XP desktop, windows docked together in the bottom-left corner with a real playlist. A third-party capture, so treat it as context, not as a pixel-measurement source. |

## Sources

All files were retrieved on 2026-10-29.

- **0.2a, 0.92, 1.006, 2.x** — English Wikipedia's Winamp article images (uploaded there as
  non-free historical screenshots):
  [Winamp 0.2a](https://en.wikipedia.org/wiki/File:Winamp_0.2a.jpg),
  [WinAMP 0.92](https://en.wikipedia.org/wiki/File:WinAMP_0.92.jpg),
  [Winamp1.006](https://en.wikipedia.org/wiki/File:Winamp1.006.PNG),
  [Winamp2](https://en.wikipedia.org/wiki/File:Winamp2.PNG).
- **2.05** — the official Nullsoft overview page as archived by the Wayback Machine
  ([capture](https://web.archive.org/web/19981207050153/http://winamp.com/winamp/overview.html));
  the images came from `media.winamp.com/images/screenshot-main.jpg` and
  `media.winamp.com/nontoxic/winamp/images/config1.gif` / `config3.gif`.
- **2.1** — the 1999 winamp.com front page as archived by the Wayback Machine;
  `media.winamp.com/nontoxic/winamp/images/waprevue-mainandeq.gif` and
  [`wa210shot.gif`](https://web.archive.org/web/19990508064504/http://media.winamp.com/nontoxic/winamp/images/wa210shot.gif).
- **2.95 (stack)** — OldVersion.com's Winamp 2.95 page,
  [oldversion.com/software/winamp/winamp-2-95](https://oldversion.com/software/winamp/winamp-2-95/).
- **2.95 (desktop)** — deskto.ps's Winamp 2.95 gallery,
  [deskto.ps/a/audio/winamp_2_95](https://deskto.ps/a/audio/winamp_2_95) (capture uploaded by
  HEXWALKER).

## Gaps

Still wanted, and not yet in this directory: a capture of the skinned **main menu** (right-click
on the main window) and of the **playlist context menu**, plus the **EQ preset load/save
dialog** and the **file info (ID3) dialog**. Add them here with their source, and reference them
from the README the same way.
