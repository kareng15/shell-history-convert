// Runs the built binary end to end against the fixture files, rather than
// calling formats::parse_*/to_* directly, so a regression in argument
// parsing or stdin/file handling shows up here even though those pieces
// have no unit tests of their own.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name)
}

fn fixture(name: &str) -> String {
    std::fs::read_to_string(fixture_path(name)).expect("failed to read fixture")
}

fn run_on_file(from: &str, to: &str, file: &Path) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_histconv"))
        .args(["--from", from, "--to", to])
        .arg(file)
        .output()
        .expect("failed to run histconv");
    assert!(
        output.status.success(),
        "histconv --from {from} --to {to} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("output was not valid utf-8")
}

fn run_on_stdin(from: &str, to: &str, input: &str) -> String {
    let mut child = Command::new(env!("CARGO_BIN_EXE_histconv"))
        .args(["--from", from, "--to", to])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to spawn histconv");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .expect("failed to write to stdin");
    let output = child.wait_with_output().expect("failed to wait on histconv");
    assert!(
        output.status.success(),
        "histconv --from {from} --to {to} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("output was not valid utf-8")
}

macro_rules! conversion_test {
    ($name:ident, $from:literal, $to:literal) => {
        #[test]
        fn $name() {
            let from_file = concat!($from, "_history.txt");
            let to_file = concat!($to, "_history.txt");
            let got = run_on_file($from, $to, &fixture_path(from_file));
            assert_eq!(got, fixture(to_file));
        }
    };
}

conversion_test!(converts_zsh_to_bash, "zsh", "bash");
conversion_test!(converts_bash_to_zsh, "bash", "zsh");
conversion_test!(converts_zsh_to_fish, "zsh", "fish");
conversion_test!(converts_fish_to_zsh, "fish", "zsh");
conversion_test!(converts_bash_to_fish, "bash", "fish");
conversion_test!(converts_fish_to_bash, "fish", "bash");

#[test]
fn reads_from_stdin_when_no_file_given() {
    let got = run_on_stdin("zsh", "bash", &fixture("zsh_history.txt"));
    assert_eq!(got, fixture("bash_history.txt"));
}

#[test]
fn flattens_multiline_zsh_command_when_converting_to_bash() {
    let got = run_on_stdin("zsh", "bash", ": 1693600000:0;echo foo && \\\necho bar\n");
    assert_eq!(got, "#1693600000\necho foo && ; echo bar\n");
}

#[test]
fn rejects_unknown_format() {
    let output = Command::new(env!("CARGO_BIN_EXE_histconv"))
        .args(["--from", "csh", "--to", "bash"])
        .output()
        .expect("failed to run histconv");
    assert!(!output.status.success());
}
