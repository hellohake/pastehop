# macOS Clipboard File Upload Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Upload files copied in macOS Finder through PasteHop's existing remote-terminal paste shortcut while preserving native local-terminal paste behavior.

**Architecture:** Add a deep clipboard-content interface that resolves macOS file URLs before image pixels. Reuse the existing explicit-file preparation and transport path for files, while retaining the existing PNG materialization path for image pixels and native passthrough for unsupported content.

**Tech Stack:** Rust 2024, arboard's macOS file-list adapter, tempfile, existing PasteHop staging and transport modules.

## Global Constraints

- Local terminal sessions must return native paste before clipboard inspection.
- Remote clipboard priority is files, then image pixels, then native text paste.
- File clipboard content preserves the original file and extension; no MIME or extension allowlist.
- Invalid file clipboard content must fail explicitly and must not fall back to an image thumbnail.
- Linux and Windows behavior must remain unchanged.
- Existing size and file-count settings remain authoritative.
- Screen recording, background transfer, progress UI, directories, and transport changes are out of scope.

---

### Task 1: Model clipboard files and read macOS file URLs

**Files:**
- Modify: `src/clipboard.rs`
- Modify: `src/errors.rs`

**Interfaces:**
- Produces: `ClipboardContent::{Files(Vec<PathBuf>), Image(ClipboardImage)}`
- Produces: `read_clipboard_content() -> Result<ClipboardContent, PasteHopError>`

- [ ] Add failing tests showing that file paths take precedence over image data and preserve multiple paths in order.
- [ ] Run the focused clipboard tests and confirm failure because file clipboard content is not modeled.
- [ ] Reuse `arboard.get().file_list()` behind a macOS-only private helper.
- [ ] Preserve `arboard` image fallback and rename the unsupported-content error without changing its exit code.
- [ ] Run focused clipboard tests and confirm they pass.

### Task 2: Route clipboard files through explicit upload preparation

**Files:**
- Modify: `src/app.rs`
- Modify: `src/staging.rs` only if a small shared preparation helper is required

**Interfaces:**
- Consumes: `read_clipboard_content()` and `ClipboardContent`
- Consumes: existing `prepare_explicit_uploads()` and `prepare_clipboard_upload()`
- Produces: one or more `PreparedUpload` values for either clipboard variant

- [ ] Add failing tests for mapping clipboard files to explicit uploads and images to PNG uploads.
- [ ] Run focused tests and confirm the file branch is absent.
- [ ] Implement one preparation helper used by both `handle_attach` and `execute_hook`.
- [ ] Ensure hooks return passthrough only for genuinely unsupported clipboard content, not invalid copied files.
- [ ] Run focused application and staging tests and confirm they pass.

### Task 3: Document behavior and verify cross-platform compilation

**Files:**
- Modify: `README.md`

**Interfaces:**
- Documents: Finder file copy behavior, local passthrough, content priority, and existing size limits

- [ ] Update README examples and behavior notes without describing unsupported recording or async transfer features.
- [ ] Run `cargo fmt --all -- --check`.
- [ ] Run `cargo clippy --locked --all-targets -- -D warnings`.
- [ ] Run `cargo test --locked` and confirm all tests pass.
- [ ] Run `cargo check --locked --target x86_64-unknown-linux-gnu` when the target is available, confirming macOS-only dependencies are correctly gated.

### Task 4: Build and replace the installed binary

**Files:**
- Replace: `/Users/bytedance/.local/bin/ph`
- Backup: `/Users/bytedance/.local/bin/ph.pre-clipboard-files-<timestamp>`

**Interfaces:**
- Produces: installed `ph` with unchanged CLI surface and expanded `--clipboard` semantics on macOS

- [ ] Run `cargo build --release --locked`.
- [ ] Compare the built binary architecture and version output with the installed binary.
- [ ] Copy the installed binary to a timestamped backup without overwriting prior backups.
- [ ] Install the release binary through a temporary sibling path and atomic rename.
- [ ] Run `~/.local/bin/ph --version`, `~/.local/bin/ph doctor`, and dry-run clipboard/file checks.
- [ ] Verify the installed binary checksum matches `target/release/ph`.

