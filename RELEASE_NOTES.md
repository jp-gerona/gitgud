## gitgud v0.1.0

First public release. A lightweight terminal UI for git, written in Rust, that surfaces every `git ...` command it runs so you learn the underlying CLI as you go.

### Features
- **Status view** — staged / unstaged / untracked panes with a live diff preview, color-coded by file status.
- **Log view** — last 200 commits with ref chips (HEAD / branches / tags); right-pane `git show --stat` detail. Switch tabs with `1` / `2`.
- **Slash Command mode** — press `/` and type real `git ...` commands; `↑`/`↓` recall history. `/exit` / `/quit` close gitgud.
- **Stage / unstage / discard** — `s` / `u` / `X` (with `[y/N]` confirmation), each showing the exact `git` invocation in the teaching command bar.
- **Modal vi commit editor** — Normal / Insert / Command modes (`i a I A o O`, `h j k l w b 0 $ gg G`, `x dd dw D`, `:w :wq :x :q :q!`), with a mode-aware hints panel and live subject (≤50) / body (≤72) column warnings.

### Install

**Pre-built binary (macOS x86_64):**
```sh
curl -LO [https://github.com/jp-gerona/gitgud/releases/download/v0.1.0/gitgud-v0.1.0-x86_64-apple-darwin](https://github.com/jp-gerona/gitgud/releases/download/v0.1.0/gitgud-v0.1.0-x86_64-apple-darwin)
curl -LO [https://github.com/jp-gerona/gitgud/releases/download/v0.1.0/SHA256SUMS](https://github.com/jp-gerona/gitgud/releases/download/v0.1.0/SHA256SUMS)
shasum -a 256 -c SHA256SUMS
chmod +x gitgud-v0.1.0-x86_64-apple-darwin
mv gitgud-v0.1.0-x86_64-apple-darwin /usr/local/bin/gitgud
```

**From source (any platform with Cargo):**
```sh
cargo install --git [https://github.com/jp-gerona/gitgud](https://github.com/jp-gerona/gitgud) --tag v0.1.0
```

### Known limitations / not yet built

- Hunk-level staging
- Branch and stash views
- Interactive rebase
- Honoring slash-command args (`--oneline`, `-n`, `--graph`) — tracked in issue #1
- Pre-built binaries for Linux (build from source for now)

### Verification

The SHA256 checksum of the macOS x86_64 binary can be verified with:
```sh
shasum -a 256 -c SHA256SUMS
```
