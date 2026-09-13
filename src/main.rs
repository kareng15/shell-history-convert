mod formats;

use std::env;
use std::fs;
use std::io::{self, Read, Write};
use std::process::ExitCode;

struct Args {
    from: String,
    to: String,
    path: Option<String>,
    in_place: bool,
}

fn parse_args() -> Result<Args, String> {
    let mut from = None;
    let mut to = None;
    let mut path = None;
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
            other if path.is_none() => path = Some(other.to_string()),
            other => return Err(format!("unexpected argument: {other}")),
        }
    }

    let from = from.ok_or("missing --from <zsh|bash>")?;
    let to = to.ok_or("missing --to <zsh|bash>")?;

    for fmt in [&from, &to] {
        if fmt != "zsh" && fmt != "bash" && fmt != "fish" {
            return Err(format!("unknown format '{fmt}', expected 'zsh', 'bash', or 'fish'"));
        }
    }

    if in_place {
        match path.as_deref() {
            None | Some("-") => return Err("--in-place needs a FILE, not stdin".to_string()),
            _ => {}
        }
    }

    Ok(Args { from, to, path, in_place })
}

fn print_usage() {
    eprintln!("histconv --from <zsh|bash|fish> --to <zsh|bash|fish> [FILE]");
    eprintln!();
    eprintln!("Converts shell history between zsh extended history and bash");
    eprintln!("history formats. Reads FILE if given, otherwise reads stdin.");
    eprintln!("Pass '-' as FILE to read stdin explicitly.");
    eprintln!();
    eprintln!("--in-place    write the result back to FILE instead of stdout");
    eprintln!("              (requires FILE; can't be used with stdin)");
}

fn read_input(path: &Option<String>) -> io::Result<String> {
    match path.as_deref() {
        None | Some("-") => {
            let mut buf = String::new();
            io::stdin().read_to_string(&mut buf)?;
            Ok(buf)
        }
        Some(p) => fs::read_to_string(p),
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

    let input = match read_input(&args.path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error reading input: {e}");
            return ExitCode::FAILURE;
        }
    };

    let entries = match args.from.as_str() {
        "zsh" => formats::parse_zsh(&input),
        "bash" => formats::parse_bash(&input),
        "fish" => formats::parse_fish(&input),
        _ => unreachable!("validated in parse_args"),
    };

    let output = match args.to.as_str() {
        "zsh" => formats::to_zsh(&entries),
        "bash" => formats::to_bash(&entries),
        "fish" => formats::to_fish(&entries),
        _ => unreachable!("validated in parse_args"),
    };

    if args.in_place {
        let path = args.path.as_deref().expect("validated in parse_args");
        if let Err(e) = fs::write(path, output) {
            eprintln!("error writing {path}: {e}");
            return ExitCode::FAILURE;
        }
    } else if io::stdout().write_all(output.as_bytes()).is_err() {
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}
