# Plans

Planned features, written by the plan loop and implemented by the feature loop.
Each plan: goal, approach, files touched, acceptance criteria. Move finished plans to Done.

> **Steward drift note (2026-10-06; condensed 2026-10-23 by the steward role).** This note
> tracked the initial prompt's two commitments — a "very close if not pixel-perfect" classic
> Winamp UI and Apple Music as the library — while the build caught up. Both are now reflected
> in the shipped app and the usage reference: the Winamp chrome (base-skin palette,
> bevel layer, custom title bar, title bands, Now Playing LCD, browse selection and row
> styling, compact and mini transport strips) and the Apple Music library (REST browse and
> search behind the `AppleMusicService` seam with the sample library as fallback, and preview
> playback through the audio seam). The per-landing detail this note once carried is retired
> here; `PLANS-DONE.md` holds the individual entries, and no current brief-vs-build drift is
> known. Standing lesson: the chrome arrived piecemeal, so later changes should reuse the
> existing vocabulary — `bevel::lcd_well`, `bevel::raised_panel`, and the shared style
> helpers — rather than inventing new chrome. The README's detailed Usage reference moved to
> `docs/usage.md` on 2026-10-26, so plans that change usage prose edit that file, not the README.
> Record any new drift here rather than appending another note.

## Planned

_None yet._

### Answer `--version` and `--help` on the command line without opening the window (planned 2026-10-29 by the plan role)

**Goal.** `docs/usage.md` ends with "There are no CLI flags yet": `src/main.rs` `main` ignores its
arguments, so `nostalgia --version` or `nostalgia --help` (or a typo such as `nostalgia --hlep`)
loads the settings file, prints `Winamp-style Apple Music Player started!`, starts the Apple
Music service, and opens the player window. Make `--version`/`-V` print the version and
`--help`/`-h` print a short usage text, each exiting 0 without opening a window; any other
argument prints an error and exits 2. With no arguments, launch is unchanged.

**Approach.** Decided, so the implementer has no open questions. Standard library only (no
argument-parsing crate).

1. New module `src/cli.rs`, declared `pub mod cli;` in `src/main.rs` beside the other modules (the
   crate denies `missing_docs`, so the module and every public item carry doc comments):
   - `pub enum CliAction { Run, PrintVersion, PrintHelp, Unrecognized(String) }`.
   - `pub fn parse(args: impl IntoIterator<Item = std::ffi::OsString>) -> CliAction`, taking the
     arguments **after** the program name. No arguments → `Run`. Exactly one argument equal to
     `--version` or `-V` → `PrintVersion`; `--help` or `-h` → `PrintHelp`. Otherwise
     `Unrecognized(arg)` naming the first argument that is not a recognized flag, or the second
     argument when the first was recognized (so `--help --version` and `--version extra` are
     errors naming `--version` and `extra`). The argument is converted with
     `to_string_lossy`, so a non-UTF-8 argument is reported, never a panic — `main` must read
     `std::env::args_os()`, not `std::env::args()`, which panics on non-UTF-8 input.
   - `pub const VERSION_TEXT: &str = concat!(env!("CARGO_PKG_NAME"), " ", env!("CARGO_PKG_VERSION"));`
     (`nostalgia 0.1.0`).
   - `pub const HELP_TEXT: &str`: a usage line `Usage: nostalgia [--help | --version]`, one line
     each for `-h, --help` and `-V, --version`, an `Environment:` block naming
     `APPLE_MUSIC_DEVELOPER_TOKEN` (starts MusicKit sign-in) and `NOSTALGIA_SETTINGS_PATH`
     (overrides the settings file path), and a closing line pointing at `docs/usage.md` for the
     controls.
   - `pub fn unrecognized_text(arg: &str) -> String` returning
     `nostalgia: unrecognized argument '<arg>'\nTry 'nostalgia --help' for more information.`
2. `src/main.rs` `main`: as its first statement, before `FileStore::from_environment()`, match
   `cli::parse(std::env::args_os().skip(1))`: `Run` falls through to the existing body;
   `PrintVersion` does `println!("{}", cli::VERSION_TEXT)` and `return Ok(())`; `PrintHelp` prints
   `HELP_TEXT` the same way; `Unrecognized(arg)` does `eprintln!("{}", cli::unrecognized_text(&arg))`
   then `std::process::exit(2)`. So a flag never reads or writes settings, starts sign-in, or opens
   a window.
3. Tests in `src/cli.rs`'s own `#[cfg(test)] mod tests` (the binary has no `tests/` directory;
   `cargo test` runs these): `no_arguments_runs_the_player`, `version_flags_print_the_version`
   (`--version` and `-V`), `help_flags_print_the_help` (`--help` and `-h`),
   `an_unknown_argument_is_unrecognized` (`--hlep`, and a bare `song.mp3`),
   `a_second_argument_is_unrecognized` (`--help --version` → `--version`; `-V extra` → `extra`),
   `a_non_utf8_argument_is_reported_lossily` (unix-only `#[cfg(unix)]`, building the `OsString`
   with `std::os::unix::ffi::OsStringExt::from_vec(vec![0x66, 0xff])` and expecting
   `Unrecognized("f\u{FFFD}")`), `version_text_names_the_package_version` (equals
   `format!("nostalgia {}", env!("CARGO_PKG_VERSION"))`), `help_text_names_every_flag_and_variable`
   (contains `--help`, `-h`, `--version`, `-V`, `APPLE_MUSIC_DEVELOPER_TOKEN`,
   `NOSTALGIA_SETTINGS_PATH`), and `unrecognized_text_names_the_argument_and_help`.
4. `docs/usage.md`: replace the closing "There are no CLI flags yet." with a sentence saying
   `nostalgia --version` (or `-V`) prints the version and `nostalgia --help` (or `-h`) prints the
   flags and environment variables, each exiting without opening the window, and any other
   argument prints an error and exits with status 2.
5. `docs/invariants.md`: add a short `## Command line` section directly before `## Sign-in server`
   stating that with no arguments launch is unchanged, that a flag or an unrecognized argument is
   handled before settings load, sign-in, or the window, and that a non-UTF-8 argument is
   reported rather than panicking; name `cli::parse` and `main` as the enforcing code and the
   tests above.

**Files touched.** `src/cli.rs` (new), `src/main.rs`, `docs/usage.md`, `docs/invariants.md`.

**Acceptance criteria.**
- `cargo run -- --version` prints `nostalgia 0.1.0` and `cargo run -- --help` prints the help text,
  each exiting 0 without opening a window or printing the `started!` line; `cargo run -- --bogus`
  prints the two-line error to stderr and exits 2.
- `cargo run` with no arguments behaves exactly as before (same `main` body after the match).
- `main` reads `std::env::args_os()`, and the unit tests listed above pass.
- No new dependency, message, state field, or setting.
- `docs/usage.md` and `docs/invariants.md` describe the flags, and `make check` passes.

## Done
