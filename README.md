# PasteHop

Terminal AI coding agents like Claude Code, Codex, and Pi can accept image
paths as input, but when you're SSH'd into a remote box your clipboard still
lives on your local machine. PasteHop uploads a local clipboard image, files
copied in macOS Finder, or explicit local files
to the remote host over SSH and pastes the resulting remote path into the
active terminal pane. The CLI binary is `ph`.

## What It Does

- Preserves the common paste flow in supported terminals (currently WezTerm and Kitty)
- For unsupoprted terminals, a simple command (can be alias'ed/keybinded) can be used for the same
- Uploads clipboard images, files copied in macOS Finder, or explicit files to a remote staging directory over SSH
- Pastes a remote path that the agent can consume immediately (can also copy the remote path to your clipboard)
- Uses the system `ssh` and `scp`; no remote daemon or server-side install required

## Direct Integration Terminals

PasteHop works across all terminals but for the below terminals, it can install a hook to directly inject its functionality so your workflow does not include any additional steps and you can keep using the terminal's native paste shortcut. Currently supported direct integration terminals are:

- WezTerm
- Kitty

## Install

Install the latest prebuilt binary from GitHub Releases:

```bash
curl -fsSL https://raw.githubusercontent.com/shantanugoel/pastehop/main/install.sh | sh
```

Installs the latest GitHub release for macOS or Linux and places `ph` in
`~/.local/bin` by default, or `/usr/local/bin` when run as root.

Or build and install from crates.io with Cargo:

```bash
cargo install pastehop
```

## Quick Start

### Option A: Auto-magic Ctrl+V (recommended)

This hooks into your terminal so that pressing `Ctrl+V` while in an SSH session
automatically uploads supported clipboard content and pastes the remote path.

```bash
# 1. Check that your environment is ready (ssh, scp, clipboard access)
ph doctor

# 2. Trust the SSH target that PasteHop is allowed to upload to
ph trust --host user@devbox

# 3. Install the integration for your terminal
ph install wezterm
# or
ph install kitty

# 4. That's it. Now:
#    - Copy an image, or copy one or more files in Finder on macOS
#    - Focus a remote SSH session in your terminal
#    - Press your terminal's normal paste shortcut
#    - PasteHop uploads the content and pastes the remote path
```

Kitty installs `Ctrl+V` and `Ctrl+Shift+V` on Linux, and `Cmd+V`, `Ctrl+V`, and `Ctrl+Shift+V` on macOS. Its remote target detection relies on an explicit `--host` override or parsing the foreground `ssh` or `kitten ssh` command line, so non-SSH shells fall back to normal paste.

### Option B: Manual via the command line

Use `ph attach` directly when you want explicit control, are scripting, or
don't use the WezTerm or Kitty integration.

```bash
# Trust the target once
ph trust --host user@devbox

# Upload a local file to a remote host and print the remote path
ph attach ./diagram.png --host user@devbox

# Upload an image, or files copied in Finder on macOS
ph attach --clipboard --host user@devbox

# Dry-run to see what would happen without actually uploading
ph attach ./diagram.png --host user@devbox --dry-run
```

### Option C: Add a shell alias or global shortcut

If you use the same remote host frequently, create a short alias for the
clipboard-upload flow. Replace `user@devbox` with your SSH target:

```bash
# bash / zsh
alias phclip='ph attach --clipboard --host user@devbox --copy-path'
```

```fish
# fish
alias phclip "ph attach --clipboard --host user@devbox --copy-path"
```

Add the alias to your shell startup file (`~/.zshrc`, `~/.bashrc`, or
`~/.config/fish/config.fish`), reload your shell, then run:

```bash
ph trust --host user@devbox
phclip
```

`--copy-path` copies the resulting remote path back to your local clipboard, so
you can paste it into the active terminal or chat input afterward.

For a true global hotkey, use a small wrapper script instead of a shell alias
because desktop shortcut managers do not load your interactive shell aliases.
If `ph` is installed somewhere other than `~/.local/bin/ph`, update the script
accordingly.

```bash
mkdir -p ~/.local/bin
cat > ~/.local/bin/phclip-remote <<'EOF'
#!/usr/bin/env sh
exec "$HOME/.local/bin/ph" attach --clipboard --host user@devbox --copy-path
EOF
chmod +x ~/.local/bin/phclip-remote
```

Then bind `~/.local/bin/phclip-remote` to a system-wide shortcut:

- macOS: create a Shortcut with a `Run Shell Script` action that runs
  `~/.local/bin/phclip-remote`, then assign a keyboard shortcut to that
  Shortcut.
- Linux: add a custom keyboard shortcut in your desktop environment that runs
  `~/.local/bin/phclip-remote`. In GNOME this lives under keyboard shortcuts;
  in KDE it is under custom shortcuts.

If you use WezTerm or Kitty, prefer `ph install wezterm` or `ph install kitty`
because they preserve the existing paste flow directly inside SSH sessions.
The alias or global shortcut approach is most useful for other terminals.

## Common Commands

### attach -- upload files or clipboard content to a remote host

```bash
# Trust the target once
ph trust --host user@devbox

# Upload a single file
ph attach ./screenshot.png --host user@devbox

# Upload multiple files at once
ph attach ./fig1.png ./fig2.png --host user@devbox

# Upload an image, or files copied in Finder on macOS
ph attach --clipboard --host user@devbox

# Upload and also copy the resulting remote path to your clipboard
ph attach ./photo.jpg --host user@devbox --copy-path

# Preview what would be uploaded without actually doing it
ph attach ./diagram.png --host user@devbox --dry-run

# Use a specific path format
ph attach ./spec.pdf --host user@devbox --profile at-path
ph attach ./spec.pdf --host user@devbox --profile quoted-path

# Override the default remote upload directory
ph attach ./data.csv --host user@devbox --remote-dir /tmp/uploads
```

### Clipboard behavior

Terminal hooks inspect the clipboard only after they identify a remote SSH target. In a local terminal, PasteHop returns control to the terminal's native paste action, so copying a Finder file continues to paste its local path normally.

In a remote terminal on macOS, clipboard content is handled in this order:

1. One or more files copied in Finder are uploaded as their original files, preserving extensions.
2. Copied image pixels, such as screenshots, are encoded and uploaded as PNG.
3. Text and unsupported clipboard content use the terminal's native paste action.

Files are not restricted by extension, so videos such as `.mov` and `.mp4` are supported. Existing `max_files`, `max_single_file_bytes`, and `max_total_bytes` settings still apply. If copied files fail validation, PasteHop reports the error rather than silently uploading a preview image.

### trust -- approve a remote host for uploads

```bash
ph trust --host user@devbox
ph trust --host user@devbox --remote-dir /srv/uploads
```

### install / uninstall -- set up or remove terminal paste integration

```bash
ph install wezterm
ph uninstall wezterm
ph install kitty
ph uninstall kitty
```

### doctor -- check that your environment is ready

```bash
ph doctor
```

### gc -- clean up expired remote uploads

```bash
ph gc --host user@devbox
ph gc --host user@devbox --dry-run
```

## Configuration

Configuration is optional. Editing the config file is only needed if you want to
pre-approve hosts, adjust size limits, or change the cleanup TTL. You can also
approve hosts with `ph trust --host ...`.

See [`config.example.toml`](config.example.toml) for all available options.


The config path can also be set via `PH_CONFIG_PATH` or `XDG_CONFIG_HOME`.

### Paths

- Config: `~/.config/pastehop/config.toml`
- Remote uploads: `~/.cache/pastehop/uploads/`

## License

MIT. See [LICENSE](LICENSE).
