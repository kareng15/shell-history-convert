/// A single history entry, independent of which format it came from.
///
/// `duration` only exists in zsh's extended history; bash has no equivalent,
/// so it's dropped (as None) on any entry that started life as bash.
pub struct Entry {
    pub timestamp: Option<u64>,
    pub duration: Option<u64>,
    pub command: String,
}

/// Parses zsh extended history: `: <start>:<elapsed>;<command>` per line.
///
/// Lines that don't match the extended format are kept as bare commands
/// rather than dropped, since plain `setopt histfile` (non-extended) history
/// is just the command text and shows up mixed in real-world files.
///
/// A command containing an embedded newline is written by zsh as multiple
/// physical lines, each but the last ending in a backslash. A literal
/// backslash at the end of a line is itself doubled so it can't be mistaken
/// for that continuation marker, so the split has to count trailing
/// backslashes rather than just check for one.
pub fn parse_zsh(input: &str) -> Vec<Entry> {
    let lines: Vec<&str> = input.lines().collect();
    let mut entries = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        i += 1;
        if line.is_empty() {
            continue;
        }

        let (meta, first_segment) = match parse_zsh_prefix(line) {
            Some((ts, dur, command)) => (Some((ts, dur)), command),
            None => (None, line.to_string()),
        };

        let mut segments = vec![first_segment];
        while trailing_backslashes(segments.last().unwrap()) % 2 == 1 {
            if i >= lines.len() {
                break;
            }
            segments.push(lines[i].to_string());
            i += 1;
        }

        let last = segments.len() - 1;
        for segment in &mut segments[..last] {
            segment.truncate(segment.len() - 1);
        }

        entries.push(Entry {
            timestamp: meta.map(|(ts, _)| ts),
            duration: meta.map(|(_, dur)| dur),
            command: unescape_zsh_command(&segments.join("\n")),
        });
    }
    entries
}

fn parse_zsh_prefix(line: &str) -> Option<(u64, u64, String)> {
    let rest = line.strip_prefix(": ")?;
    let (meta, command) = rest.split_once(';')?;
    let (ts, dur) = meta.split_once(':')?;
    let timestamp = ts.trim().parse().ok()?;
    let duration = dur.trim().parse().ok()?;
    Some((timestamp, duration, command.to_string()))
}

fn trailing_backslashes(s: &str) -> usize {
    s.chars().rev().take_while(|&c| c == '\\').count()
}

/// Collapses the doubled backslashes zsh writes in place of literal ones.
/// Real newlines are untouched: by the time this runs, continuation markers
/// have already been stripped and the segments joined with actual `\n`s.
fn unescape_zsh_command(s: &str) -> String {
    s.replace("\\\\", "\\")
}

/// Reverses `unescape_zsh_command`: doubles literal backslashes and turns
/// embedded newlines into a backslash-continuation so the round trip through
/// `parse_zsh` reproduces the original command.
fn escape_zsh_command(command: &str) -> String {
    if !command.contains('\\') && !command.contains('\n') {
        return command.to_string();
    }
    let mut out = String::with_capacity(command.len());
    for ch in command.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\\n"),
            other => out.push(other),
        }
    }
    out
}

/// Parses bash history: plain commands, optionally preceded by a
/// `#<unix-epoch>` timestamp line (written when HISTTIMEFORMAT is set).
pub fn parse_bash(input: &str) -> Vec<Entry> {
    let mut entries = Vec::new();
    let mut pending_timestamp: Option<u64> = None;
    for line in input.lines() {
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix('#') {
            if let Ok(ts) = rest.trim().parse::<u64>() {
                pending_timestamp = Some(ts);
                continue;
            }
        }
        entries.push(Entry {
            timestamp: pending_timestamp.take(),
            duration: None,
            command: line.to_string(),
        });
    }
    entries
}

pub fn to_zsh(entries: &[Entry]) -> String {
    let mut out = String::new();
    for e in entries {
        let ts = e.timestamp.unwrap_or(0);
        let dur = e.duration.unwrap_or(0);
        out.push_str(&format!(": {}:{};{}\n", ts, dur, escape_zsh_command(&e.command)));
    }
    out
}

pub fn to_bash(entries: &[Entry]) -> String {
    let mut out = String::new();
    for e in entries {
        if let Some(ts) = e.timestamp {
            out.push_str(&format!("#{}\n", ts));
        }
        out.push_str(&e.command);
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_single_line_extended_entry() {
        let entries = parse_zsh(": 1693600000:0;git status\n");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].timestamp, Some(1693600000));
        assert_eq!(entries[0].duration, Some(0));
        assert_eq!(entries[0].command, "git status");
    }

    #[test]
    fn reassembles_backslash_continued_command() {
        let entries = parse_zsh(": 1693600000:0;echo foo && \\\necho bar\n");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].command, "echo foo && \necho bar");
    }

    #[test]
    fn reassembles_command_spanning_more_than_two_lines() {
        let entries = parse_zsh(": 1693600000:0;one \\\ntwo \\\nthree\n");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].command, "one \ntwo \nthree");
    }

    #[test]
    fn trailing_literal_backslash_is_not_treated_as_continuation() {
        // zsh doubles a real trailing backslash, so this is one line with a
        // literal backslash at the end, not a continuation.
        let entries = parse_zsh(": 1693600000:0;echo foo\\\\\nnext command\n");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].command, "echo foo\\");
        assert_eq!(entries[1].command, "next command");
    }

    #[test]
    fn zsh_round_trips_multiline_and_literal_backslash_commands() {
        let entries = vec![
            Entry {
                timestamp: Some(1),
                duration: Some(0),
                command: "echo foo\necho bar".to_string(),
            },
            Entry {
                timestamp: Some(2),
                duration: Some(0),
                command: "echo foo\\".to_string(),
            },
        ];
        let written = to_zsh(&entries);
        let reparsed = parse_zsh(&written);
        assert_eq!(reparsed.len(), 2);
        assert_eq!(reparsed[0].command, entries[0].command);
        assert_eq!(reparsed[1].command, entries[1].command);
    }

    #[test]
    fn bare_lines_without_extended_prefix_still_parse() {
        let entries = parse_zsh("git status\nls -la\n");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].timestamp, None);
        assert_eq!(entries[0].command, "git status");
    }
}
