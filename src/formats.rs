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
pub fn parse_zsh(input: &str) -> Vec<Entry> {
    let mut entries = Vec::new();
    for line in input.lines() {
        if line.is_empty() {
            continue;
        }
        match parse_zsh_line(line) {
            Some(entry) => entries.push(entry),
            None => entries.push(Entry {
                timestamp: None,
                duration: None,
                command: line.to_string(),
            }),
        }
    }
    entries
}

fn parse_zsh_line(line: &str) -> Option<Entry> {
    let rest = line.strip_prefix(": ")?;
    let (meta, command) = rest.split_once(';')?;
    let (ts, dur) = meta.split_once(':')?;
    let timestamp = ts.trim().parse().ok()?;
    let duration = dur.trim().parse().ok()?;
    Some(Entry {
        timestamp: Some(timestamp),
        duration: Some(duration),
        command: command.to_string(),
    })
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
        out.push_str(&format!(": {}:{};{}\n", ts, dur, e.command));
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
