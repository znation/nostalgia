//! Command-line flags: `--version` and `--help` are answered before the
//! settings load, sign-in, or the window, and any other argument is an error.

use std::ffi::OsString;

/// What the process should do for its command-line arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliAction {
    /// No arguments: launch the player.
    Run,
    /// `--version` or `-V`: print [`VERSION_TEXT`] and exit.
    PrintVersion,
    /// `--help` or `-h`: print [`HELP_TEXT`] and exit.
    PrintHelp,
    /// An argument that is not a recognized flag, or a flag with a trailing argument.
    Unrecognized(String),
}

/// The line `--version` prints: the package name and version.
pub const VERSION_TEXT: &str = concat!(env!("CARGO_PKG_NAME"), " ", env!("CARGO_PKG_VERSION"));

/// The text `--help` prints.
pub const HELP_TEXT: &str = "Usage: nostalgia [--help | --version]

  -h, --help     Print this help and exit
  -V, --version  Print the version and exit

Environment:
  APPLE_MUSIC_DEVELOPER_TOKEN  Starts MusicKit sign-in when set
  NOSTALGIA_SETTINGS_PATH      Overrides the settings file path

See docs/usage.md for the controls.";

/// Decides the action for `args`, the arguments after the program name.
///
/// A non-UTF-8 argument is converted lossily, so it is reported, never a panic.
pub fn parse(args: impl IntoIterator<Item = OsString>) -> CliAction {
    let mut args = args.into_iter();
    let Some(first) = args.next() else {
        return CliAction::Run;
    };
    let first = first.to_string_lossy().into_owned();
    let action = match first.as_str() {
        "--version" | "-V" => CliAction::PrintVersion,
        "--help" | "-h" => CliAction::PrintHelp,
        _ => return CliAction::Unrecognized(first),
    };
    match args.next() {
        Some(extra) => CliAction::Unrecognized(extra.to_string_lossy().into_owned()),
        None => action,
    }
}

/// The two-line error printed for an unrecognized argument.
///
/// The argument is the process's own command line, echoed to the terminal
/// raw, so it is formatted with `Debug` — as the play path's `play_log_line`
/// (`src/apple_music/playback/play.rs`) formats a track id —
/// rather than `Display`: a control character (a newline
/// forging a second output line, an escape sequence rewriting the
/// terminal's state) or a Unicode format character (`\u{202e}`) in an
/// argument would otherwise reach the stream as-is, while `Debug` renders
/// it as its `\u{...}` escape.
pub fn unrecognized_text(arg: &str) -> String {
    format!(
        "nostalgia: unrecognized argument {arg:?}\nTry 'nostalgia --help' for more information."
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(args: &[&str]) -> CliAction {
        parse(args.iter().map(OsString::from))
    }

    #[test]
    fn no_arguments_runs_the_player() {
        assert_eq!(run(&[]), CliAction::Run);
    }

    #[test]
    fn version_flags_print_the_version() {
        assert_eq!(run(&["--version"]), CliAction::PrintVersion);
        assert_eq!(run(&["-V"]), CliAction::PrintVersion);
    }

    #[test]
    fn help_flags_print_the_help() {
        assert_eq!(run(&["--help"]), CliAction::PrintHelp);
        assert_eq!(run(&["-h"]), CliAction::PrintHelp);
    }

    #[test]
    fn an_unknown_argument_is_unrecognized() {
        assert_eq!(run(&["--hlep"]), CliAction::Unrecognized("--hlep".into()));
        assert_eq!(
            run(&["song.mp3"]),
            CliAction::Unrecognized("song.mp3".into())
        );
    }

    #[test]
    fn a_second_argument_is_unrecognized() {
        assert_eq!(
            run(&["--help", "--version"]),
            CliAction::Unrecognized("--version".into())
        );
        assert_eq!(
            run(&["-V", "extra"]),
            CliAction::Unrecognized("extra".into())
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_non_utf8_argument_is_reported_lossily() {
        use std::os::unix::ffi::OsStringExt;
        let arg = OsString::from_vec(vec![0x66, 0xff]);
        assert_eq!(
            parse([arg]),
            CliAction::Unrecognized("f\u{FFFD}".to_string())
        );
    }

    #[test]
    fn version_text_names_the_package_version() {
        assert_eq!(
            VERSION_TEXT,
            format!("nostalgia {}", env!("CARGO_PKG_VERSION"))
        );
    }

    #[test]
    fn help_text_names_every_flag_and_variable() {
        for needle in [
            "--help",
            "-h",
            "--version",
            "-V",
            "APPLE_MUSIC_DEVELOPER_TOKEN",
            "NOSTALGIA_SETTINGS_PATH",
        ] {
            assert!(HELP_TEXT.contains(needle), "{needle}");
        }
    }

    #[test]
    fn unrecognized_text_names_the_argument_and_help() {
        let text = unrecognized_text("--bogus");
        assert!(text.contains("--bogus"));
        assert!(text.contains("nostalgia --help"));
    }

    // An argument echoing to the terminal is escaped like the play path's
    // track id: a control character must not land in the stream raw, so a
    // newline cannot forge a second output line and an escape sequence
    // cannot rewrite the terminal's state. A Unicode format character
    // (`\u{202e}`) is escaped too, as `char::is_control` never fires on the
    // `Cf` category. The message stays two lines, and no raw byte escapes.
    #[test]
    fn unrecognized_text_escapes_control_and_format_characters() {
        let text = unrecognized_text("evil\u{1b}]0;pwnd\u{7}\na \u{202e}b");
        assert!(!text.contains('\u{1b}'));
        assert!(!text.contains('\u{7}'));
        assert!(!text.contains('\u{202e}'));
        // The only newline is the one between the error line and the Try
        // line, so the argument's own newline cannot forge an extra line.
        assert_eq!(text.lines().count(), 2);
        assert!(text.contains("nostalgia --help"));
    }
}
