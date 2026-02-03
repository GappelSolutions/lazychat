# Theming System

Lazychat supports full theme customization via TOML configuration files.

## Quick Start

1. Copy the example config:
   ```bash
   mkdir -p ~/.config/lazychat
   cp config.example.toml ~/.config/lazychat/config.toml
   ```

2. Edit the colors to your liking

3. Restart lazychat

## Config File Locations

Lazychat looks for config in this order:
1. `~/.config/lazychat/config.toml`
2. `~/.lazychat.toml`
3. `./lazychat.toml` (current directory)

## Theme Options

```toml
[theme]
# Border colors
border = "#5c6370"           # Inactive panel borders
border_active = "#98c379"    # Active panel borders

# Selection
selected_bg = "#1e3250"      # Background for selected items

# Session status indicator colors
status_working = "#56b6c2"   # Actively processing (<10s)
status_active = "#98c379"    # Recent activity (<2 min)
status_idle = "#e5c07b"      # Waiting (2-30 min)
status_inactive = "#5c6370"  # Old (>30 min)
status_waiting = "#c678dd"   # Needs user input

# Diff view colors
diff_add = "#98c379"         # Added lines
diff_remove = "#e06c75"      # Removed lines
diff_hunk = "#61afef"        # Hunk headers (@@)

# Text colors
text = "#abb2bf"             # Primary text
text_muted = "#5c6370"       # Secondary/disabled text
highlight = "#61afef"        # Highlighted text
```

## Preset Themes

### One Dark (Default)
The default theme, inspired by Atom's One Dark.

### Dracula
```toml
[theme]
border = "#6272a4"
border_active = "#50fa7b"
selected_bg = "#44475a"
status_working = "#8be9fd"
status_active = "#50fa7b"
status_idle = "#f1fa8c"
status_inactive = "#6272a4"
status_waiting = "#ff79c6"
diff_add = "#50fa7b"
diff_remove = "#ff5555"
diff_hunk = "#8be9fd"
text = "#f8f8f2"
text_muted = "#6272a4"
highlight = "#bd93f9"
```

### Nord
```toml
[theme]
border = "#4c566a"
border_active = "#a3be8c"
selected_bg = "#3b4252"
status_working = "#88c0d0"
status_active = "#a3be8c"
status_idle = "#ebcb8b"
status_inactive = "#4c566a"
status_waiting = "#b48ead"
diff_add = "#a3be8c"
diff_remove = "#bf616a"
diff_hunk = "#81a1c1"
text = "#eceff4"
text_muted = "#4c566a"
highlight = "#81a1c1"
```

### Gruvbox
```toml
[theme]
border = "#665c54"
border_active = "#b8bb26"
selected_bg = "#3c3836"
status_working = "#83a598"
status_active = "#b8bb26"
status_idle = "#fabd2f"
status_inactive = "#665c54"
status_waiting = "#d3869b"
diff_add = "#b8bb26"
diff_remove = "#fb4934"
diff_hunk = "#83a598"
text = "#ebdbb2"
text_muted = "#665c54"
highlight = "#fe8019"
```

### Tokyo Night
```toml
[theme]
border = "#565f89"
border_active = "#9ece6a"
selected_bg = "#24283b"
status_working = "#7dcfff"
status_active = "#9ece6a"
status_idle = "#e0af68"
status_inactive = "#565f89"
status_waiting = "#bb9af7"
diff_add = "#9ece6a"
diff_remove = "#f7768e"
diff_hunk = "#7aa2f7"
text = "#c0caf5"
text_muted = "#565f89"
highlight = "#7aa2f7"
```

### Catppuccin Mocha
```toml
[theme]
border = "#585b70"
border_active = "#a6e3a1"
selected_bg = "#313244"
status_working = "#89dceb"
status_active = "#a6e3a1"
status_idle = "#f9e2af"
status_inactive = "#585b70"
status_waiting = "#cba6f7"
diff_add = "#a6e3a1"
diff_remove = "#f38ba8"
diff_hunk = "#89b4fa"
text = "#cdd6f4"
text_muted = "#585b70"
highlight = "#89b4fa"
```

### Iceberg Dark
```toml
[theme]
border = "#444b71"
border_active = "#b4be82"
selected_bg = "#1e2132"
status_working = "#89b8c2"
status_active = "#b4be82"
status_idle = "#e2a478"
status_inactive = "#444b71"
status_waiting = "#a093c7"
diff_add = "#b4be82"
diff_remove = "#e27878"
diff_hunk = "#84a0c6"
text = "#c6c8d1"
text_muted = "#444b71"
highlight = "#84a0c6"
```

### Sakura
Based on [omarchy-sakura-theme](https://github.com/bjarneo/omarchy-sakura-theme)
```toml
[theme]
border = "#4a3c45"
border_active = "#F29B9A"
selected_bg = "#0d0509"
status_working = "#E8C099"
status_active = "#F29B9A"
status_idle = "#D4A882"
status_inactive = "#4a3c45"
status_waiting = "#D1B399"
diff_add = "#F29B9A"
diff_remove = "#E85F6F"
diff_hunk = "#D9A56C"
text = "#f0eaed"
text_muted = "#4a3c45"
highlight = "#D1B399"
```

## Color Format

Colors must be in hex format with 6 digits:
- `#rrggbb` - e.g., `#98c379`

The `#` prefix is optional but recommended for clarity.

## Terminal Compatibility

### True Color (24-bit)
Most modern terminals support true color. Lazychat uses RGB colors which display best with true color support.

**Terminals with true color:**
- iTerm2
- Alacritty
- Kitty
- WezTerm
- Windows Terminal
- GNOME Terminal (3.x+)
- macOS Terminal.app (limited)

### 256 Color Fallback
If your terminal doesn't support true color, RGB colors will be approximated to the nearest 256-color palette color.

### Testing True Color
```bash
# If you see a smooth gradient, you have true color
awk 'BEGIN{
  for(i=0;i<256;i++)
    printf "\033[48;2;%d;0;0m \033[0m",i
  print ""
}'
```

## Creating Custom Themes

1. Start with a preset that's close to what you want
2. Adjust colors using a tool like:
   - [coolors.co](https://coolors.co)
   - [color.adobe.com](https://color.adobe.com)
3. Test with transparent backgrounds if you use them
4. Consider contrast for readability

### Tips
- Keep status colors distinct (don't use similar hues)
- Selection background should be subtle but visible
- Test diff colors against both dark and light code
- Muted text should be readable but clearly secondary
