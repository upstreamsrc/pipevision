# PipeVision

A TUI tool for inspecting data as it moves through Unix pipelines. Each stage shows
lines, bytes, throughput, and the actual output passing through — so you can figure
out which `grep` ate your data or why `sort` got nothing.

## Project layout

```
pipevision/
├── Cargo.toml          # ratatui + crossterm for the TUI
├── src/
│   ├── main.rs         # dispatch: injected mode vs TUI mode
│   ├── app.rs          # TUI state machine, key handling, file loading
│   ├── pipeline.rs     # pipeline file parsing, command injection, stats capture
│   ├── stats.rs        # metrics data structures, stats file I/O
│   └── ui.rs           # ratatui rendering (welcome, table, detail, output)
├── pipe.txt            # example pipeline file
├── test.txt            # sample log data
└── README.md
```

## Two modes

### 1. TUI mode (no arguments, or with a file)

```
pipevision              # opens the TUI with a welcome screen — press L to load a file
pipevision pipe.txt     # loads pipe.txt and starts the pipeline immediately
```

### 2. Injected mode (when used inside a real pipeline)

```
cat test.txt | pipevision --stage=1 --run-id=$$ --quiet | grep ERROR | sort
```

Each `pipevision` instance passes data through and writes two temp files per stage:
- `/tmp/pipevision_<run-id>_<stage>.txt` — metrics (lines, bytes, throughput)
- `/tmp/pipevision_<run-id>_<stage>_output.txt` — up to 5,000 lines of captured data

The `--quiet` flag suppresses the stderr logging, which keeps the TUI clean when
running in orchestrated mode.

## How pipeline injection works

When you point PipeVision at a pipeline file, it rewrites the command before running
it. For every `|` between commands, it inserts itself as an observer.

Before:
```
cat test.txt | grep ERROR | sort
```

After:
```
cat test.txt | pipevision --stage=1 --run-id=<pid> --quiet \
  | grep ERROR \
  | pipevision --stage=2 --run-id=<pid> --quiet \
  | sort
```

The last stage doesn't get an injected instance — its output goes to the shell
and is discarded (the TUI doesn't need it).

## Controls

### Welcome / file load screen
| Key | Action |
|-----|--------|
| `L` | Open file picker |
| Type | Filter file suggestions (substring match) |
| `↑/↓` | Navigate suggestions |
| `Tab` | Cycle suggestions |
| `Enter` | Load selected file (or typed path) |
| `Esc` | Cancel |

### Pipeline screen
| Key | Action |
|-----|--------|
| `↑/↓` / `k/j` | Select stage |
| `PgUp/PgDn` | Scroll stage output |
| `R` | Run or rerun the pipeline |
| `Q` / `Esc` | Quit |

## File loading rules

PipeVision validates pipeline files when they're loaded:

| File content | Result |
|---|---|
| `cat x \| grep y \| sort` | Parses into 3 stages |
| `echo hello` | Single stage — valid |
| *(empty file)* | Rejected: "file is empty" |
| `cat x \|` | Rejected: "stage 2 is empty (trailing pipe?)" |
| `cat x \|\| grep y` | Rejected: "stage 2 is empty (double pipe?)" |
| Binary / non-UTF-8 | Rejected by `read_to_string` |

## Per-stage metrics

| Metric | Source |
|--------|--------|
| Lines | Total lines written to stdout by this stage |
| Bytes | Total bytes including newlines |
| L/s | Lines ÷ elapsed wall time |
| B/s | Bytes ÷ elapsed wall time |
| Filtered | Previous stage's lines − this stage's lines |
| Reduction % | Filtered ÷ previous stage's lines × 100 |

## Example walkthrough

The repo ships with two files:

**test.txt** — 24 lines of fake server logs:
```
INFO Starting reactor
ERROR Database connection timeout
ERROR Failed to fetch laser data
ERROR Job failed
...
```

**pipe.txt** — a pipeline that processes them:
```
cat test.txt | grep ERROR | grep INFO | sort
```

`grep ERROR` keeps 3 lines. `grep INFO` sees those 3 lines (none contain both
ERROR and INFO) and keeps 0. `sort` sorts 0 lines. The TUI shows all of this
live: throughput, filtered counts, and the actual data at each stage.

### Running it

```
cargo run -- pipe.txt
```

Or compile first:
```
cargo build --release
./target/release/pipevision pipe.txt
```

## Temp files

PipeVision writes to `/tmp/pipevision_*` during execution. These are cleaned up
on rerun and on exit. If the program crashes, leftovers can be cleaned up with:

```
rm -f /tmp/pipevision_*.txt
```

## Dependencies

- **ratatui 0.26** — terminal UI framework
- **crossterm 0.27** — raw mode, alternate screen, keyboard input
- **tokio** — present in Cargo.toml but not yet used (future: async execution)

## What it doesn't do (yet)

- No scrolling in the file suggestion list (only 8 visible, but navigable)
- No recursive directory scanning for the file picker (just the cwd)
- No hexdump view or byte-level diff between stages
- No save/load of pipeline captures
