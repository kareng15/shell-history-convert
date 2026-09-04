# histconv

I switch between bash and zsh depending on the machine, and every time I do
the shell history doesn't come with me in a useful shape. zsh writes an
extended history line like

```
: 1693600000:0;git status
```

(start time, elapsed seconds, command), while bash just writes the command,
or, if `HISTTIMEFORMAT` is set, a `#<epoch>` comment line ahead of it:

```
#1693600000
git status
```

Neither shell reads the other's format, so importing history across a
migration means hand-editing a text file with tens of thousands of lines.
`histconv` converts between the two.

## Usage

```
histconv --from <zsh|bash> --to <zsh|bash> [FILE]
```

If `FILE` is omitted (or is `-`), input is read from stdin. Output always
goes to stdout, so redirect it where you want it.

Convert a zsh history file to bash format:

```
histconv --from zsh --to bash ~/.zsh_history > bash_history.txt
```

Pipe bash history in and get zsh extended history out:

```
cat ~/.bash_history | histconv --from bash --to zsh > zsh_history.txt
```

Commands with no timestamp (plain bash history, or a zsh line that didn't
parse as extended format) come out the other side with a timestamp of `0`
when converting to zsh, and no `#` line at all when converting to bash.

## Building

Standard library only, no dependencies:

```
cargo build --release
```

## Status

Early. zsh's backslash-continuation for commands containing embedded
newlines is reassembled on read and re-emitted on write, so a multi-line
command survives a round trip through either format. Fish history isn't
supported yet, and there's no way to convert a file in place. See the
issues for what's planned next.

## License

MIT, see [LICENSE](LICENSE).
