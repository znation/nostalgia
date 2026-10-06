# nostalgia

A (very close if not pixel-perfect) clone of the classic Winamp UI, built in Rust, using Apple
Music as the music library.

## Initial prompt

<!-- tumwater:prompt:start -->
Nostalgia is a (very close if not pixel-perfect) clone of the classic Winamp UI, but built in Rust, and using Apple Music as the music library.
<!-- tumwater:prompt:end -->

## Status

<!-- tumwater:status:start -->
Early skeleton (0.1.0): an iced (0.14) desktop shell — a Now Playing bar showing the real song
title, transport controls with Play/Pause/Stop and Previous/Next stepping through the current album and
a Winamp-style volume slider, and an artist → album → song browser with Back navigation over an in-memory sample library; the Apple
Music API is not wired up yet. Open work lives in PLANS.md, BUGS.md, and QUESTIONS.md.
<!-- tumwater:status:end -->

## Usage

Build and run with `cargo run`; the player opens an iced window showing the Now Playing bar, the
Play/Pause/Stop/Previous/Next controls, a volume slider, and a browse list. There are no CLI flags or
config files yet.
