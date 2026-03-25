# tabby-zj

tabby-zj: Zellij sidebar plugin with grouped tab tree.

A port of the [Tabby](https://github.com/brendandebeasi/tabby) tmux plugin to Zellij. It provides a persistent, clickable vertical sidebar for managing your Zellij tabs and panes with color-coded grouping and rich activity indicators.

## Features

- **Grouped tab/pane tree** — Organize tabs into named groups with ANSI color support.
- **Mouse interactivity** — Left-click to switch tabs or focus panes.
- **Context menus** — Right-click for deep management of tabs, panes, and groups.
- **Keyboard navigation** — Navigate the tree with `j`/`k` (or arrows), `Enter` to activate, and `Esc` to clear.
- **Scrollable viewport** — Pinned area for clock and git status, with a scrollable tree for large sessions.
- **Activity indicators** — Support for busy, bell, and input alerts via the Zellij pipe system.
- **State persistence** — Remembers your group organization and collapsed states between sessions.
- **YAML configuration** — Fully customizable themes, grouping rules, and widget settings.
- **Sidebar collapse** — Expand or collapse the sidebar via mouse or pipe command to save space.
- **Hot-reload** — Refresh configuration on the fly without restarting your Zellij session.
- **Built-in widgets** — Async git status and a customizable clock.

## Requirements

- **Zellij** 0.43+
- **Rust** and `cargo` (for building from source)
- **Wasm target**: `wasm32-wasip1`

## Installation

Run the installation script to build the plugin and set up the default layout and config:

```bash
./install.sh
```

This script performs the following:
1. Builds the Wasm plugin in release mode.
2. Installs the plugin to `~/.config/zellij/plugins/tabby-zj.wasm`.
3. Installs a production layout to `~/.config/zellij/layouts/tabby-zj.kdl`.
4. Sets up a default config at `~/.config/tabby-zj/config.yaml`.

## Quick Start

Launch Zellij using the provided layout:

```bash
zellij --layout tabby-zj
```

## Configuration

The plugin is configured via `~/.config/tabby-zj/config.yaml`.

```yaml
# Tab grouping rules (first match wins)
groups:
  - name: Default
    pattern: ".*"
    theme:
      bg: "#3c3836"
      fg: "#ebdbb2"

# Sidebar display settings
sidebar:
  width: 25
  theme: catppuccin-mocha # Options: catppuccin-mocha, rose-pine-dawn
  sort_by: group           # Options: group, index
  show_panes: false        # Show individual panes in the tree
  show_empty_groups: false

# Indicators for pipe-driven alerts
indicators:
  busy:
    enabled: true
  bell:
    enabled: true
  input:
    enabled: true

# Sidebar widgets
widgets:
  clock:
    enabled: true
    format: "%H:%M:%S"
    show_date: true
  git:
    enabled: true
    interval_secs: 5
```

## Mouse Controls

| Action | Result |
|--------|--------|
| **Left Click** | Switch to tab / Focus specific pane |
| **Right Click** | Open context menu (Tab, Pane, or Group) |
| **Scroll** | Scroll the sidebar tree viewport |
| **Click Divider** | Expand / Collapse the sidebar |

## Keyboard Navigation

| Key | Action |
|-----|--------|
| `j` / `Down` | Move cursor down |
| `k` / `Up` | Move cursor up |
| `Enter` | Activate selection (switch tab/focus pane) |
| `Esc` | Clear selection / Dismiss context menu |
| `Any key` | Type when the inline rename prompt is active |

## Context Menus

Right-click on any item in the tree to open a context menu:

- **Tab Menu**: Rename tab, close tab, move to group, set marker, or change tab color.
- **Pane Menu**: Rename pane, split pane, or close pane.
- **Group Menu**: Create a new tab in the group, rename group, collapse/expand group, or delete group.

## Indicators via Pipe

Control the sidebar state and indicators from the terminal using `zellij pipe`.

| Command | Description |
|---------|-------------|
| `zellij pipe --plugin tabby-zj --name tabby -- "busy:1"` | Set busy indicator |
| `zellij pipe --plugin tabby-zj --name tabby -- "busy:0"` | Clear busy indicator |
| `zellij pipe --plugin tabby-zj --name tabby -- "bell:1"` | Set bell indicator |
| `zellij pipe --plugin tabby-zj --name tabby -- "input:1"` | Set input indicator |
| `zellij pipe --plugin tabby-zj --name tabby -- "collapse:1"` | Collapse sidebar |
| `zellij pipe --plugin tabby-zj --name tabby -- "toggle"` | Toggle sidebar collapse |
| `zellij pipe --plugin tabby-zj --name tabby -- "config"` | Hot-reload configuration |

*Note: You can target a specific pane with `indicator:1:%42` or by passing `--args pane_id=%42`.*

## Claude Code / OpenCode Integration

Wire up the provided hook script to get automatic busy/input indicators in the sidebar.

### Claude Code Setup

Add to `~/.claude/settings.json`:

```json
{
  "hooks": {
    "UserPromptSubmit": [{"type": "command", "command": "/path/to/tabby-zj/scripts/tabby-zj-hook.sh UserPromptSubmit"}],
    "Stop":             [{"type": "command", "command": "/path/to/tabby-zj/scripts/tabby-zj-hook.sh Stop"}],
    "Notification":     [{"type": "command", "command": "/path/to/tabby-zj/scripts/tabby-zj-hook.sh Notification"}]
  }
}
```

### OpenCode Setup

Add to `~/.config/opencode/opencode-notifier.json`:

```json
{
  "sound": false,
  "notification": false,
  "command": {
    "enabled": true,
    "path": "/path/to/tabby-zj/scripts/tabby-zj-hook.sh",
    "args": ["{event}"]
  }
}
```

Replace `/path/to/tabby-zj` with the actual path to your tabby-zj checkout.

## Development

### Dev Loop

Build and reload the plugin automatically while editing:

```bash
cargo watch -x 'build --target wasm32-wasip1' \
  -s 'zellij action start-or-reload-plugin file:target/wasm32-wasip1/debug/tabby_zj.wasm'
```

### Testing

Run the unit tests:

```bash
cargo test
```

## Themes

Switch between dark and light themes in your `config.yaml`:

- **catppuccin-mocha**: A soft dark theme (default).
- **rose-pine-dawn**: A clean light theme.

```yaml
sidebar:
  theme: catppuccin-mocha
```
