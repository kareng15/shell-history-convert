mod formats;

use std::env;
use std::fs;
use std::io::{self, Read, Write};
use std::process::ExitCode;

struct Args {
    from: Option<String>,
    to: String,
    paths: Vec<String>,
    in_place: bool,
}

fn parse_args() -> Result<Args, String> {
    let mut from = None;
    let mut to = None;
    let mut paths = Vec::new();
    let mut in_place = false;

    let mut iter = env::args().skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--from" => from = Some(iter.next().ok_or("--from needs a value")?),
            "--to" => to = Some(iter.next().ok_or("--to needs a value")?),
            "--in-place" => in_place = true,
            "-h" | "--help" => {
                print_usage();
                std::process::exit(0);
            }
            other if other == "-" || !other.starts_with('-') => paths.push(other.to_string()),
            other => return Err(format!("unexpected argument: {other}")),
        }
    }

    let to = to.ok_or("missing --to <zsh|bash|fish>")?;

    for fmt in from.iter().chain([&to]) {
        if fmt != "zsh" && fmt != "bash" && fmt != "fish" {
            return Err(format!("unknown format '{fmt}', expected 'zsh', 'bash', or 'fish'"));
        }
    }

    if in_place && (paths.is_empty() || paths.iter().any(|p| p == "-")) {
        return Err("--in-place needs at least one FILE, not stdin".to_string());
    }

    Ok(Args { from, to, paths, in_place })
}

fn print_usage() {
    eprintln!("histconv [--from <zsh|bash|fish>] --to <zsh|bash|fish> [--in-place] [FILE...]");
    eprintln!();
    eprintln!("Converts shell history between zsh extended history, bash, and");
    eprintln!("fish history formats. Reads FILE if given, otherwise reads stdin.");
    eprintln!("Pass '-' as FILE to read stdin explicitly. Multiple FILEs are");
    eprintln!("concatenated in the order given before conversion.");
    eprintln!();
    eprintln!("--from        input format; guessed from the input if omitted");
    eprintln!("--in-place    convert each FILE and write the result back to it,");
    eprintln!("              instead of concatenating them to stdout");
    eprintln!("              (requires at least one FILE; can't be used with stdin)");
}

fn read_input(path: &str) -> io::Result<String> {
    match path {
        "-" => {
            let mut buf = String::new();
            io::stdin().read_to_string(&mut buf)?;
            Ok(buf)
        }
        p => fs::read_to_string(p),
    }
}

/// Concatenates the contents of `paths` in order, reading stdin if `paths`
/// is empty. A newline is inserted between files whose content doesn't
/// already end in one, so the last line of one file can't merge with the
/// first line of the next into something that fails to parse as either.
fn read_inputs(paths: &[String]) -> io::Result<String> {
    if paths.is_empty() {
        return read_input("-");
    }

    let mut combined = String::new();
    for path in paths {
        let content = read_input(path)?;
        combined.push_str(&content);
        if !content.ends_with('\n') {
            combined.push('\n');
        }
    }
    Ok(combined)
}

/// Picks the input format: whatever `--from` said, unless detection
/// recognizes the input as confidently something else (see the warning
/// text below for why "bash" doesn't count as confident).
fn resolve_from<'a>(explicit: Option<&'a str>, input: &str, label: &str) -> &'a str {
    match explicit {
        Some(f) => {
            let detected = formats::detect_format(input);
            if detected != "bash" && detected != f {
                eprintln!(
                    "warning: {label} looks like {detected} history, but --from {f} was given; \
                     proceeding with {f}"
                );
            }
            f
        }
        None => formats::detect_format(input),
    }
}

fn parse_entries(from: &str, input: &str) -> Vec<formats::Entry> {
    match from {
        "zsh" => formats::parse_zsh(input),
        "bash" => formats::parse_bash(input),
        "fish" => formats::parse_fish(input),
        _ => unreachable!("validated in parse_args"),
    }
}

fn render(to: &str, entries: &[formats::Entry]) -> String {
    match to {
        "zsh" => formats::to_zsh(entries),
        "bash" => formats::to_bash(entries),
        "fish" => formats::to_fish(entries),
        _ => unreachable!("validated in parse_args"),
    }
}

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("error: {e}");
            print_usage();
            return ExitCode::FAILURE;
        }
    };

    if args.in_place {
        for path in &args.paths {
            let input = match read_input(path) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("error reading {path}: {e}");
                    return ExitCode::FAILURE;
                }
            };
            let from = resolve_from(args.from.as_deref(), &input, path);
            let entries = parse_entries(from, &input);
            let output = render(&args.to, &entries);
            if let Err(e) = fs::write(path, output) {
                eprintln!("error writing {path}: {e}");
                return ExitCode::FAILURE;
            }
        }
        return ExitCode::SUCCESS;
    }

    let input = match read_inputs(&args.paths) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error reading input: {e}");
            return ExitCode::FAILURE;
        }
    };

    let from = resolve_from(args.from.as_deref(), &input, "input");
    let entries = parse_entries(from, &input);
    let output = render(&args.to, &entries);

    if io::stdout().write_all(output.as_bytes()).is_err() {
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}
