# CCometixLine: Install from Source Guide

## 1. Build & Install

```bash
# Clone and build
git clone https://github.com/Haleclipse/CCometixLine.git
cd CCometixLine
cargo build --release

# The binary is at target/release/ccometixline
# Install it system-wide so "ccline" works anywhere:
sudo ln -sf "$(pwd)/target/release/ccometixline" /usr/local/bin/ccline
```

> **Note:** The Cargo binary name is `ccometixline` (from `Cargo.toml`), but the CLI declares itself as `ccline`. The symlink lets you use the shorter name. `/usr/local/bin` is already in `$PATH` on virtually every distro.

**Alternative without sudo** — add to your shell config (`~/.zshrc` or `~/.bashrc`):

```bash
export PATH="$HOME/path/to/CCometixLine/target/release:$PATH"
alias ccline="ccometixline"
```

Verify it works:

```bash
ccline --version
```

## 2. Wire it into Claude Code

Edit `~/.claude/settings.json` (create if it doesn't exist):

```json
{
  "statusLine": {
    "type": "command",
    "command": "ccline",
    "padding": 0
  }
}
```

If `ccline` isn't in your `$PATH`, use the full path:

```json
{
  "statusLine": {
    "type": "command",
    "command": "/usr/local/bin/ccline",
    "padding": 0
  }
}
```

**How it works:** Every time Claude Code updates its display, it pipes JSON to your command's stdin and shows whatever your command prints to stdout. The JSON includes the model, workspace, transcript path, cost, output style, and effort level.

## 3. First Run — Interactive Setup

```bash
# Run without piped input to get the main menu:
ccline

# Or go directly to the TUI configurator:
ccline --config
# or
ccline -c
```

The main menu lets you:
- **Configuration Mode** — launch the full TUI
- **Initialize Config** — create `~/.claude/ccline/config.toml` with defaults
- **Check Configuration** — validate your config file

## 4. TUI Configurator — Full Keybinding Reference

### Navigation

| Key | Action |
|-----|--------|
| `Tab` | Switch between Segments panel and Settings panel |
| `Up/Down` | Navigate segments or settings fields |
| `Enter` | Toggle enabled/disabled, or open editor for the selected field |
| `Shift+Up/Down` | Reorder segments |

### Theming

| Key | Action |
|-----|--------|
| `1-4` | Quick-switch to themes (1=cometix, 2=default, 3=gruvbox, 4=nord) |
| `P` | Cycle through ALL themes (includes powerline-dark, powerline-light, powerline-rose-pine, powerline-tokyo-night) |
| `R` | Reset current theme to its defaults |

### Editing

| Key | Action |
|-----|--------|
| `E` | Edit separator character (e.g., ` \| `, `│`, ``) |
| `S` | Save config to `config.toml` |
| `W` | Write current config to the current theme file |
| `Ctrl+S` | Save as a NEW named theme |

> **S vs W vs Ctrl+S** — three distinct save operations:
> - `S` saves to `~/.claude/ccline/config.toml` — this is what ccline reads at runtime
> - `W` writes your current tweaks back into the theme file (e.g., `~/.claude/ccline/themes/cometix.toml`), so the theme itself is updated
> - `Ctrl+S` creates a brand new theme file with a name you choose

### Color Picker (when open)

| Key | Action |
|-----|--------|
| `Up/Down` | Navigate colors |
| `Tab` | Switch mode (16/256/RGB) |
| `R` | Jump to RGB input |
| `Enter` | Apply selected color |
| `Esc` | Cancel |

### Icon Selector (when open)

| Key | Action |
|-----|--------|
| `Up/Down` | Navigate icons |
| `Tab` | Toggle plain/nerd font |
| `C` | Enter custom icon input |
| `Enter` | Apply selected icon |
| `Esc` | Cancel |

### General

| Key | Action |
|-----|--------|
| `Esc` | Quit configurator |

The **live preview** at the top updates instantly as you toggle segments, change colors, or switch themes — no need to save first.

## 5. Available Segments

| Segment | What it shows | Default |
|---------|--------------|---------|
| **Model** | Claude model name + version (e.g., "Opus 4.6") | Enabled |
| **Directory** | Current working directory name | Enabled |
| **Git** | Branch, clean/dirty status, ahead/behind | Enabled |
| **Context Window** | Token usage % + count (e.g., "78.2% - 156.4k") | Enabled |
| **Usage** | Anthropic API 5h utilization % + reset time | Disabled |
| **Cost** | Session cost in USD | Disabled |
| **Session** | Duration + lines added/removed | Disabled |
| **Output Style** | Current output style name | Disabled |
| **Update** | Current ccline version / update indicator | Disabled |
| **Effort** | Current reasoning effort level (high/medium/low/max) | Disabled |
| **Extra Usage** | Bonus credit consumption ($used/$limit) | Disabled |

## 6. Tips & Tricks

### Quick theme previewing

Override theme from command line without changing config:

```bash
echo '{"model":{"id":"claude-opus-4-6","display_name":"Opus 4.6"},"workspace":{"current_dir":"/tmp"},"transcript_path":"/tmp/t.jsonl"}' | ccline -t powerline-tokyo-night
```

### Enabling the Usage segment (requires OAuth)

The Usage and Extra Usage segments need an Anthropic OAuth token. If you're logged into Claude Code with an Anthropic account (not an API key), the token is already stored. The segment auto-detects it from:
- macOS Keychain
- `~/.claude/.credentials.json` (Linux)
- GNOME Keyring

You can tune the API behavior per-segment in the TUI under Options, or directly in `config.toml`:

```toml
[[segments]]
id = "usage"
enabled = true
# ...
[segments.options]
api_base_url = "https://api.anthropic.com"
cache_duration = 180    # seconds between API calls
timeout = 2             # seconds before giving up
```

### Effort segment fallback chain

The Effort segment tries these sources in order:
1. Stdin JSON `effort_level` field (set when you use `/effort` in Claude Code)
2. `~/.claude/settings.json` -> `effortLevel` key
3. `CLAUDE_CODE_EFFORT_LEVEL` environment variable
4. Defaults to `"high"`

### Nerd Fonts

Plain/emoji icons work everywhere, but the NerdFont and Powerline style modes need a [Nerd Font](https://www.nerdfonts.com/) installed in your terminal. Popular choices: JetBrains Mono Nerd Font, FiraCode Nerd Font.

### Creating custom themes

1. Start from an existing theme (`P` to cycle)
2. Tweak colors, icons, segment order, enable/disable
3. `Ctrl+S` -> type a name -> now it's saved as `~/.claude/ccline/themes/yourname.toml`
4. It shows up in the theme selector on next launch

### All config files

```
~/.claude/ccline/
├── config.toml                 # Main config (what ccline reads)
├── themes/                     # Theme files
│   ├── cometix.toml
│   ├── default.toml
│   ├── gruvbox.toml
│   ├── minimal.toml
│   ├── nord.toml
│   ├── powerline-dark.toml
│   ├── powerline-light.toml
│   ├── powerline-rose-pine.toml
│   ├── powerline-tokyo-night.toml
│   └── your-custom-theme.toml
├── models.toml                 # Custom model definitions (optional)
├── .api_usage_cache.json       # Usage API cache (auto-managed)
└── .update_state.json          # Version check cache (auto-managed)
```

### The patcher (optional, advanced)

Disables context window warnings in Claude Code's UI:

```bash
ccline --patch $(which claude)
# Creates a .backup file automatically
```

## 7. Rebuilding After Changes

```bash
cargo build --release
# If you used a symlink, you're done. Otherwise:
sudo cp target/release/ccometixline /usr/local/bin/ccline
```

The statusline updates on the next Claude Code output — no restart needed.

## CLI Reference

```
ccline                    # Interactive main menu (when no stdin)
ccline -c / --config      # Launch TUI configurator directly
ccline -t <theme>         # Override theme for this invocation
ccline --patch <path>     # Patch Claude Code binary/cli.js
ccline --version          # Show version
ccline --help             # Show help
```
