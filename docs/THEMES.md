# Theming System

Lazychat uses a flexible, component-based theming system designed for terminal compatibility and extensibility. This document covers the current implementation and the planned theming framework.

**Table of Contents**
- [Current Colors (Hardcoded)](#current-colors-hardcoded)
- [Themeable Elements](#themeable-elements)
- [Planned Theme Format](#planned-theme-format)
- [Example Themes](#example-themes)
- [Terminal Compatibility](#terminal-compatibility)
- [Implementation Roadmap](#implementation-roadmap)

---

## Current Colors (Hardcoded)

The current color system is defined in `src/ui/mod.rs`. All colors are constants that can be easily extracted into a configurable theme system.

### Core Theme Constants

```rust
// Borders and focus states
pub const BORDER_COLOR: Color = Color::Blue;      // Inactive panel borders
pub const BORDER_ACTIVE: Color = Color::Green;    // Active panel borders

// UI States
pub const SELECTED_BG: Color = Color::Rgb(30, 50, 80);  // Selection background (subtle blue)
pub const MUTED: Color = Color::DarkGray;               // Disabled/secondary text

// Status Indicators
pub const SUCCESS: Color = Color::Green;
pub const WARNING: Color = Color::Yellow;
pub const ERROR: Color = Color::Red;
pub const INFO: Color = Color::Cyan;
```

### Colors by Component

#### Session List (`src/ui/sessions.rs`)

| Element | Current Color | Usage |
|---------|---------------|-------|
| Status indicator - working | `Color::Cyan` | Actively processing (<10s) |
| Status indicator - active | `Color::Green` | Recent activity (<2 min) |
| Status indicator - idle | `Color::Yellow` | Waiting (2-30 min) |
| Status indicator - inactive | `Color::DarkGray` | Old (>30 min) |
| Status indicator - waiting | `Color::Magenta` | Waiting for user input |
| Selected session text | `Color::White` | Session name when selected |
| Unselected session text | `Color::Gray` | Session name when unselected |
| Metadata (time, count) | `MUTED` (DarkGray) | Timestamp and message count |

#### File Changes (`src/ui/mod.rs`)

| Element | Current Color | Usage |
|---------|---------------|-------|
| Modified (M) | `Color::Yellow` | Git status indicator |
| Added (A) | `Color::Green` | New files |
| Deleted (D) | `Color::Red` | Removed files |
| Renamed (R) | `Color::Magenta` | Renamed files |
| Untracked (?) | `Color::Gray` | Untracked files |
| File additions | `Color::Rgb(100, 180, 100)` | Line additions counter |
| File deletions | `Color::Rgb(180, 100, 100)` | Line deletions counter |
| Selected file | `Color::White` (bold) | Highlighted file |
| Unselected file | `Color::Gray` | Normal file text |
| Directory label | `Color::Blue` (bold) | Tree view directories |

#### Diff View (`src/ui/sessions.rs`)

| Element | Current Color | Usage |
|---------|---------------|-------|
| Added lines (+) | `Color::Green` | Diff additions |
| Removed lines (-) | `Color::Red` | Diff removals |
| Hunk headers (@@) | `Color::Cyan` | Unified diff headers |
| File headers | `Color::Yellow` | diff/index lines |
| Context lines | `Color::Gray` | Unchanged lines |

#### Todo Items (`src/ui/mod.rs`)

| Element | Current Color | Usage |
|---------|---------------|-------|
| In-progress icon | `Color::Rgb(255, 180, 180)` | Active todo indicator |
| In-progress text | `Color::Rgb(255, 180, 180)` | Active todo text |
| Completed icon (✓) | `MUTED` (DarkGray) | Completed todo |
| Completed text | `MUTED` (DarkGray) | Completed todo text |
| Pending icon (□) | `Color::Gray` | Pending todo |
| Pending text | `Color::Gray` | Pending todo text |

#### Chat Messages (`src/ui/sessions.rs`)

| Element | Current Color | Usage |
|---------|---------------|-------|
| User prefix (▶ You) | `Color::Cyan` (bold) | User message identifier |
| Claude prefix (◀ Claude) | `Color::Green` (bold) | Claude response identifier |
| User message text | `Color::White` | Message body for user |
| Claude message text | `Color::Gray` | Message body for Claude |
| Timestamp | `MUTED` (DarkGray) | Message timestamp |
| Tool status - completed | `SUCCESS` (Green) | Successful tool execution |
| Tool status - error | `Color::Red` | Failed tool execution |
| Tool status - pending | `WARNING` (Yellow) | Tool in progress |

#### Input Fields (`src/ui/mod.rs` and `src/ui/sessions.rs`)

| Element | Current Color | Usage |
|---------|---------------|-------|
| Filter input border | `Color::Yellow` | Active filter input |
| Rename input border | `Color::Yellow` | Active rename input |
| Filter input text | `Color::White` | User input text |
| Input placeholder | `Color::Gray` | Disabled or empty input |

#### Help Bar & Popup

| Element | Current Color | Usage |
|---------|---------------|-------|
| Help text | `Color::DarkGray` | Footer help bar text |
| Help keys | `Color::Yellow` | Keyboard shortcut keys |
| Help descriptions | `Color::Gray` | Key descriptions |
| Section headers | `INFO` (Cyan) (bold) | Help popup sections |

---

## Themeable Elements

Complete list of UI elements that should support theming:

### Structural Elements

- **Panel borders** - Inactive and active states
- **Panel backgrounds** - Optional background colors
- **Selection highlight** - Current item selection background
- **Focus indicator** - Visual indicator of active panel
- **Input boxes** - Border color, text color, active state

### Text Styling

- **Primary text** - Default foreground color
- **Secondary text** - Muted/metadata color
- **Emphasis text** - Bold or highlighted text
- **Error text** - Error messages and indicators
- **Success text** - Success messages and indicators
- **Warning text** - Warning messages and indicators
- **Info text** - Informational text

### Status Indicators

- **Working/active** - Currently processing
- **Idle** - Waiting for user or inactive
- **Completed** - Task finished successfully
- **Failed/error** - Task failed
- **Pending** - Waiting to be processed
- **Waiting** - Paused, waiting for input

### Semantic Colors

- **Git status colors** - Modified, added, deleted, renamed
- **Diff colors** - Added lines, removed lines, context
- **Message roles** - User vs. Claude distinguishing colors

---

## Planned Theme Format

### TOML Theme Configuration

Themes will be defined in TOML format for readability and ease of editing. Store theme files in `~/.config/lazychat/themes/` (following XDG spec).

**File structure:**
- `~/.config/lazychat/themes/default.toml`
- `~/.config/lazychat/themes/dark.toml`
- `~/.config/lazychat/themes/light.toml`
- `~/.config/lazychat/themes/nord.toml`

**Configuration location:**
- `~/.config/lazychat/config.toml` (main config, includes theme selection)

### Theme File Structure

```toml
# Theme metadata
[theme]
name = "Dark (Default)"
description = "A dark theme inspired by lazygit"
version = "1.0"
author = "lazychat"

# Border colors
[colors.borders]
inactive = { color = "blue" }           # Standard blue border
active = { color = "green" }            # Green when focused
input_active = { color = "yellow" }     # Yellow for input fields

# Selection and focus
[colors.selection]
background = { rgb = [30, 50, 80] }     # Subtle blue selection
text = "white"

# Muted/secondary elements
[colors.muted]
foreground = "dark_gray"

# Status indicators for sessions
[colors.status]
working = "cyan"                        # Actively processing
active = "green"                        # Recent activity
idle = "yellow"                         # Waiting/idle
inactive = "dark_gray"                  # Dormant/old
waiting = "magenta"                     # Waiting for user

# Git file status
[colors.git_status]
modified = "yellow"
added = "green"
deleted = "red"
renamed = "magenta"
untracked = "gray"

# Diff view
[colors.diff]
added_line = "green"
removed_line = "red"
hunk_header = "cyan"
file_header = "yellow"
context = "gray"

# Todo states
[colors.todos]
in_progress_text = { rgb = [255, 180, 180] }
in_progress_icon = { rgb = [255, 180, 180] }
completed_text = "dark_gray"
completed_icon = "dark_gray"
pending_text = "gray"
pending_icon = "gray"

# Chat messages
[colors.messages]
user_prefix = "cyan"
user_text = "white"
claude_prefix = "green"
claude_text = "gray"
timestamp = "dark_gray"

# Tool calls
[colors.tools]
success = "green"
error = "red"
pending = "yellow"

# Help text
[colors.help]
text = "dark_gray"
key = "yellow"
description = "gray"
section = "cyan"

# Semantic/semantic meaning colors
[colors.semantic]
success = "green"
error = "red"
warning = "yellow"
info = "cyan"

# Optional: Terminal compatibility settings
[terminal]
# Force 256 color mode instead of true color (24-bit)
force_256_color = false
# Enable transparency support (for terminals like Alacritty)
transparency = true
```

### Color Specification

Colors can be specified in multiple ways:

```toml
# Named colors (8 standard ANSI colors)
color = "red"
color = "green"
color = "yellow"
color = "blue"
color = "magenta"
color = "cyan"
color = "white"
color = "gray"
color = "dark_gray"
color = "black"

# Bright/light variants
color = "light_red"
color = "light_green"
# ... etc

# RGB (true color / 24-bit)
color = { rgb = [255, 100, 50] }

# 256-color mode index
color = { indexed = 42 }

# Reset to terminal default
color = "reset"
```

---

## Example Themes

### Default Theme (Lazygit-inspired)

```toml
# Default dark theme matching current hardcoded colors
[theme]
name = "Default (Dark)"
description = "Default lazychat theme inspired by lazygit"
version = "1.0"

[colors.borders]
inactive = "blue"
active = "green"
input_active = "yellow"

[colors.selection]
background = { rgb = [30, 50, 80] }
text = "white"

[colors.muted]
foreground = "dark_gray"

[colors.status]
working = "cyan"
active = "green"
idle = "yellow"
inactive = "dark_gray"
waiting = "magenta"

[colors.git_status]
modified = "yellow"
added = "green"
deleted = "red"
renamed = "magenta"
untracked = "gray"

[colors.diff]
added_line = "green"
removed_line = "red"
hunk_header = "cyan"
file_header = "yellow"
context = "gray"

[colors.todos]
in_progress_text = { rgb = [255, 180, 180] }
in_progress_icon = { rgb = [255, 180, 180] }
completed_text = "dark_gray"
completed_icon = "dark_gray"
pending_text = "gray"
pending_icon = "gray"

[colors.messages]
user_prefix = "cyan"
user_text = "white"
claude_prefix = "green"
claude_text = "gray"
timestamp = "dark_gray"

[colors.tools]
success = "green"
error = "red"
pending = "yellow"

[colors.semantic]
success = "green"
error = "red"
warning = "yellow"
info = "cyan"
```

### Light Theme

```toml
[theme]
name = "Light"
description = "A light theme for bright environments"
version = "1.0"

[colors.borders]
inactive = { rgb = [150, 150, 150] }
active = { rgb = [50, 200, 100] }
input_active = { rgb = [200, 150, 0] }

[colors.selection]
background = { rgb = [220, 220, 220] }
text = { rgb = [0, 0, 0] }

[colors.muted]
foreground = { rgb = [120, 120, 120] }

[colors.status]
working = { rgb = [0, 180, 200] }
active = { rgb = [0, 150, 0] }
idle = { rgb = [200, 150, 0] }
inactive = { rgb = [150, 150, 150] }
waiting = { rgb = [180, 0, 200] }

[colors.git_status]
modified = { rgb = [200, 150, 0] }
added = { rgb = [0, 150, 0] }
deleted = { rgb = [200, 0, 0] }
renamed = { rgb = [180, 0, 200] }
untracked = { rgb = [120, 120, 120] }

[colors.diff]
added_line = { rgb = [0, 150, 0] }
removed_line = { rgb = [200, 0, 0] }
hunk_header = { rgb = [0, 180, 200] }
file_header = { rgb = [200, 150, 0] }
context = { rgb = [80, 80, 80] }

[colors.messages]
user_prefix = { rgb = [0, 180, 200] }
user_text = { rgb = [0, 0, 0] }
claude_prefix = { rgb = [0, 150, 0] }
claude_text = { rgb = [80, 80, 80] }
timestamp = { rgb = [120, 120, 120] }

[colors.tools]
success = { rgb = [0, 150, 0] }
error = { rgb = [200, 0, 0] }
pending = { rgb = [200, 150, 0] }

[colors.semantic]
success = { rgb = [0, 150, 0] }
error = { rgb = [200, 0, 0] }
warning = { rgb = [200, 150, 0] }
info = { rgb = [0, 180, 200] }
```

### Nord Theme

```toml
# Inspired by the popular Arctic, north-bluish color palette
# https://www.nordtheme.com/
[theme]
name = "Nord"
description = "Arctic, north-bluish color palette"
version = "1.0"

[colors.borders]
inactive = { rgb = [76, 86, 106] }      # nord3
active = { rgb = [163, 190, 140] }      # nord14 (green)
input_active = { rgb = [235, 203, 139] } # nord13 (yellow)

[colors.selection]
background = { rgb = [46, 52, 64] }     # nord0 (darkest)
text = { rgb = [236, 239, 244] }        # nord4 (lightest)

[colors.muted]
foreground = { rgb = [216, 222, 233] }  # nord4

[colors.status]
working = { rgb = [136, 192, 208] }     # nord8 (frost)
active = { rgb = [163, 190, 140] }      # nord14 (green)
idle = { rgb = [235, 203, 139] }        # nord13 (yellow)
inactive = { rgb = [76, 86, 106] }      # nord3
waiting = { rgb = [191, 97, 106] }      # nord11 (red)

[colors.git_status]
modified = { rgb = [235, 203, 139] }    # nord13
added = { rgb = [163, 190, 140] }       # nord14
deleted = { rgb = [191, 97, 106] }      # nord11
renamed = { rgb = [180, 142, 173] }     # nord15 (purple)
untracked = { rgb = [216, 222, 233] }   # nord4

[colors.diff]
added_line = { rgb = [163, 190, 140] }  # nord14
removed_line = { rgb = [191, 97, 106] } # nord11
hunk_header = { rgb = [136, 192, 208] } # nord8
file_header = { rgb = [235, 203, 139] } # nord13
context = { rgb = [216, 222, 233] }     # nord4

[colors.messages]
user_prefix = { rgb = [136, 192, 208] } # nord8
user_text = { rgb = [236, 239, 244] }   # nord4 (brightest)
claude_prefix = { rgb = [163, 190, 140] } # nord14
claude_text = { rgb = [216, 222, 233] } # nord4
timestamp = { rgb = [76, 86, 106] }     # nord3

[colors.tools]
success = { rgb = [163, 190, 140] }     # nord14
error = { rgb = [191, 97, 106] }        # nord11
pending = { rgb = [235, 203, 139] }     # nord13

[colors.semantic]
success = { rgb = [163, 190, 140] }     # nord14
error = { rgb = [191, 97, 106] }        # nord11
warning = { rgb = [235, 203, 139] }     # nord13
info = { rgb = [136, 192, 208] }        # nord8
```

### Dracula Theme

```toml
# Inspired by the Dracula color scheme
# https://draculatheme.com/
[theme]
name = "Dracula"
description = "A dark theme based on the Dracula color scheme"
version = "1.0"

[colors.borders]
inactive = { rgb = [98, 114, 164] }      # purple
active = { rgb = [80, 250, 123] }        # green
input_active = { rgb = [241, 250, 140] } # yellow

[colors.selection]
background = { rgb = [68, 71, 90] }      # selection
text = { rgb = [248, 248, 242] }         # foreground

[colors.muted]
foreground = { rgb = [98, 114, 164] }    # comment

[colors.status]
working = { rgb = [139, 233, 253] }      # cyan
active = { rgb = [80, 250, 123] }        # green
idle = { rgb = [241, 250, 140] }         # yellow
inactive = { rgb = [98, 114, 164] }      # comment
waiting = { rgb = [255, 85, 85] }        # red

[colors.git_status]
modified = { rgb = [241, 250, 140] }     # yellow
added = { rgb = [80, 250, 123] }         # green
deleted = { rgb = [255, 85, 85] }        # red
renamed = { rgb = [189, 147, 249] }      # purple
untracked = { rgb = [98, 114, 164] }     # comment

[colors.diff]
added_line = { rgb = [80, 250, 123] }    # green
removed_line = { rgb = [255, 85, 85] }   # red
hunk_header = { rgb = [139, 233, 253] }  # cyan
file_header = { rgb = [241, 250, 140] }  # yellow
context = { rgb = [98, 114, 164] }       # comment

[colors.messages]
user_prefix = { rgb = [139, 233, 253] }  # cyan
user_text = { rgb = [248, 248, 242] }    # foreground
claude_prefix = { rgb = [80, 250, 123] } # green
claude_text = { rgb = [248, 248, 242] }  # foreground
timestamp = { rgb = [98, 114, 164] }     # comment

[colors.tools]
success = { rgb = [80, 250, 123] }       # green
error = { rgb = [255, 85, 85] }          # red
pending = { rgb = [241, 250, 140] }      # yellow

[colors.semantic]
success = { rgb = [80, 250, 123] }       # green
error = { rgb = [255, 85, 85] }          # red
warning = { rgb = [241, 250, 140] }      # yellow
info = { rgb = [139, 233, 253] }         # cyan
```

### Solarized Dark Theme

```toml
# Inspired by the Solarized color scheme
# https://ethanschoonover.com/solarized/
[theme]
name = "Solarized Dark"
description = "A precisely balanced dark color scheme based on Solarized"
version = "1.0"

[colors.borders]
inactive = { rgb = [101, 123, 142] }     # base01
active = { rgb = [133, 153, 0] }         # green
input_active = { rgb = [181, 137, 0] }   # yellow

[colors.selection]
background = { rgb = [7, 54, 66] }       # base02
text = { rgb = [131, 148, 150] }         # base0

[colors.muted]
foreground = { rgb = [88, 110, 117] }    # base01 (darker)

[colors.status]
working = { rgb = [42, 161, 152] }       # cyan
active = { rgb = [133, 153, 0] }         # green
idle = { rgb = [181, 137, 0] }           # yellow
inactive = { rgb = [101, 123, 142] }     # base01
waiting = { rgb = [220, 50, 47] }        # red

[colors.git_status]
modified = { rgb = [181, 137, 0] }       # yellow
added = { rgb = [133, 153, 0] }          # green
deleted = { rgb = [220, 50, 47] }        # red
renamed = { rgb = [108, 113, 196] }      # violet
untracked = { rgb = [101, 123, 142] }    # base01

[colors.diff]
added_line = { rgb = [133, 153, 0] }     # green
removed_line = { rgb = [220, 50, 47] }   # red
hunk_header = { rgb = [42, 161, 152] }   # cyan
file_header = { rgb = [181, 137, 0] }    # yellow
context = { rgb = [131, 148, 150] }      # base0

[colors.messages]
user_prefix = { rgb = [42, 161, 152] }   # cyan
user_text = { rgb = [131, 148, 150] }    # base0
claude_prefix = { rgb = [133, 153, 0] }  # green
claude_text = { rgb = [131, 148, 150] }  # base0
timestamp = { rgb = [101, 123, 142] }    # base01

[colors.tools]
success = { rgb = [133, 153, 0] }        # green
error = { rgb = [220, 50, 47] }          # red
pending = { rgb = [181, 137, 0] }        # yellow

[colors.semantic]
success = { rgb = [133, 153, 0] }        # green
error = { rgb = [220, 50, 47] }          # red
warning = { rgb = [181, 137, 0] }        # yellow
info = { rgb = [42, 161, 152] }          # cyan
```

---

## Terminal Compatibility

### Color Depth Support

Lazychat will automatically detect terminal capabilities and fall back gracefully:

#### True Color (24-bit RGB)

**Supported by:**
- iTerm2 (macOS)
- Terminal.app (macOS 10.14+)
- GNOME Terminal
- Alacritty
- Kitty
- WezTerm
- VS Code integrated terminal

**Configuration:**
```toml
[terminal]
# Themes can use full RGB colors
# Example: { rgb = [255, 100, 50] }
```

#### 256-Color Mode

**Supported by:**
- Most modern terminals (default for SSH, older terminals)
- tmux
- Screen

**Fallback strategy:**
1. If theme specifies `{ rgb = [255, 100, 50] }`, find nearest 256-color index
2. If theme specifies `{ indexed = 42 }`, use directly
3. If only named colors available, use ANSI color mapping

**Configuration:**
```toml
[terminal]
# Force 256-color mode even if terminal supports true color
force_256_color = false

# Or provide fallback indexed colors
[colors.borders]
inactive = { rgb = [76, 86, 106], indexed_fallback = 60 }
```

#### Standard ANSI (16 colors)

**Fallback for ancient terminals:**
- Xterm (classic)
- Linux console
- Some SSH sessions

**Color mapping:**
```
Named color       → ANSI index
"red"             → 1
"green"           → 2
"yellow"          → 3
"blue"            → 4
"magenta"         → 5
"cyan"            → 6
"white"           → 7
"light_red"       → 91
"light_green"     → 92
... etc
```

### Transparency Support

Some terminal emulators support transparency, allowing the background to show through. This is useful for themes with transparency effects.

**Supported by:**
- Alacritty (with `opacity` setting)
- WezTerm (with `window_background_opacity`)
- Kitty (with `background_opacity`)
- Some other modern terminals

**In themes:**
```toml
[terminal]
# Enable transparency support if available
transparency = true

# Can use color with alpha (if terminal supports it)
# This is a future enhancement
# color = { rgba = [30, 50, 80, 200] }
```

**Current limitation:** Ratatui uses Crossterm, which doesn't have built-in alpha channel support. Transparency can be achieved through:
1. Terminal emulator configuration (set globally)
2. Background color inheritance from terminal theme
3. Future: Custom VT-100 escape sequences if needed

### Terminal Environment Detection

The theme system will detect terminal capabilities:

```rust
// Pseudocode for terminal detection
fn get_color_mode() -> ColorMode {
    match env::var("COLORTERM") {
        Ok(val) if val == "truecolor" => ColorMode::TrueColor,
        _ => match env::var("TERM") {
            Ok(term) if term.contains("256color") => ColorMode::Color256,
            _ => ColorMode::Color16,
        }
    }
}
```

**Environment variables used:**
- `COLORTERM=truecolor` - Indicates true color support
- `TERM` - Terminal type (xterm-256color, xterm, etc.)
- `NO_COLOR` - Disable colors entirely (respect this convention)

---

## Implementation Roadmap

### Phase 1: Foundation (Current)

**Status:** Documented, not yet implemented

- [x] Analyze current hardcoded colors
- [x] Document all themeable elements
- [x] Design TOML format
- [ ] Create example themes
- **Next:** Build theme loader and parser

### Phase 2: Theme Loading (Planned)

**Target version:** 0.2.0

**What will be implemented:**

1. **Theme Parser**
   - Load TOML theme files from `~/.config/lazychat/themes/`
   - Validate theme structure and color values
   - Provide meaningful error messages for invalid themes

2. **Configuration File**
   - Add theme selection to `~/.config/lazychat/config.toml`
   - Example:
     ```toml
     [ui]
     theme = "default"
     # or: theme = "/path/to/custom/theme.toml"
     ```

3. **Built-in Themes**
   - Ship with: `default.toml`, `light.toml`, `nord.toml`
   - Optional: `dracula.toml`, `solarized-dark.toml`

4. **Runtime Theme Application**
   - Create `Theme` struct to hold all color values
   - Pass theme context through app initialization
   - Replace all hardcoded constants with theme lookups

5. **Color Fallback Logic**
   - Detect terminal color capabilities
   - Automatically convert RGB → 256-color → 16-color as needed
   - Handle `NO_COLOR` environment variable

### Phase 3: Dynamic Switching (Future)

**Target version:** 0.3.0+

**What will be possible:**

1. **Live Theme Switching**
   - Command to switch themes without restarting
   - Hot-reload theme files
   - Preview themes before applying

2. **Theme Editor**
   - Interactive color picker within TUI
   - Export custom themes
   - Live preview of changes

3. **Theme Marketplace**
   - Share themes in community
   - Download themes from repository
   - Version management

4. **Per-component Overrides**
   - Override individual colors for specific use cases
   - Compose themes from multiple sources

### Phase 4: Advanced Features (Vision)

**Possible future enhancements:**

1. **Animated Transitions**
   - Smooth color transitions when switching themes
   - Fade effects for status changes

2. **Context-Aware Colors**
   - Different colors based on session type or project
   - Time-of-day themes (dark at night, light during day)

3. **Accessibility**
   - High contrast modes
   - Colorblind-friendly themes (deuteranopia, protanopia, tritanopia)
   - Configurable color saturation

4. **Gradient Support**
   - Smooth color gradients for backgrounds
   - Terminal-appropriate gradient approximation

---

## Usage Examples

### Using a Built-in Theme

```bash
# Create config with theme selection
mkdir -p ~/.config/lazychat
cat > ~/.config/lazychat/config.toml << 'EOF'
[ui]
theme = "nord"

[terminal]
force_256_color = false
transparency = true
EOF

# Run lazychat (will load nord.toml theme)
lazychat
```

### Using a Custom Theme

```bash
# Create custom theme
mkdir -p ~/.config/lazychat/themes
cat > ~/.config/lazychat/themes/my-theme.toml << 'EOF'
[theme]
name = "My Custom Theme"
version = "1.0"

[colors.borders]
inactive = { rgb = [100, 100, 100] }
active = { rgb = [50, 200, 50] }
# ... rest of theme
EOF

# Reference in config
cat > ~/.config/lazychat/config.toml << 'EOF'
[ui]
theme = "my-theme"
EOF
```

### Theme Discovery

Once Phase 2 is complete, themes will be discoverable:

```bash
# List available themes
lazychat --list-themes

# Show theme info
lazychat --show-theme nord

# Create theme interactively
lazychat --create-theme
```

---

## Contributing Themes

To contribute a new built-in theme:

1. Create a TOML file in `themes/` directory
2. Ensure all required color sections are present
3. Test on multiple terminals (256-color and true color)
4. Test with colorblind simulators (optional but appreciated)
5. Submit PR with theme file and preview images

**Theme requirements:**
- Must include all color groups (borders, status, git_status, diff, etc.)
- Must include complete documentation in theme header
- Must test with `cargo run -- --validate-theme themes/your-theme.toml`
- Should include visual preview (screenshot) in PR

---

## References

- [Ratatui Color Documentation](https://docs.rs/ratatui/latest/ratatui/style/enum.Color.html)
- [256 Color Chart](https://cheat.sh/256)
- [True Color Support](https://github.com/termstandard/colors)
- [Solarized](https://ethanschoonover.com/solarized/)
- [Dracula](https://draculatheme.com/)
- [Nord](https://www.nordtheme.com/)
- [NO_COLOR Standard](https://no-color.org/)
- [Terminal Color Capabilities](https://github.com/alacritty/alacritty/wiki/FAQ)
