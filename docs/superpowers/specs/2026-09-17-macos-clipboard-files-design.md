# macOS Clipboard File Upload Design

## Goal

Allow PasteHop terminal hooks on macOS to upload files copied in Finder, including video files, and inject their remote paths without changing native paste behavior in local terminal sessions.

## Scope

- Support one or more readable local files represented by macOS pasteboard file URLs.
- Preserve each source file's extension and reuse the existing explicit-file naming, size validation, SSH upload, cleanup, and path-formatting behavior.
- Keep clipboard image support unchanged for screenshots and copied image pixels.
- Keep text and unsupported clipboard content on the terminal's native paste path.
- Keep Linux and Windows clipboard behavior unchanged.
- Do not add screen-recording controls, asynchronous transfers, progress UI, directories, or transport changes.

## Behavior

The terminal hook first resolves whether the active terminal is connected to a trusted SSH target. Clipboard inspection only happens after a remote target is resolved. Therefore local terminal windows always return `passthrough_key` and retain Kitty or WezTerm's native paste semantics.

In a remote terminal, clipboard content is resolved in this order:

1. Local file URLs: upload the original files and inject their remote paths.
2. Image pixels: encode one PNG, upload it, and inject its remote path.
3. Text or unsupported data: return `passthrough_key`.

If file URLs are present but invalid, unreadable, not regular files, too numerous, or too large, PasteHop reports the existing validation error. It must not fall back to a thumbnail image because that would silently upload different content from what the user copied.

## Architecture

`clipboard.rs` exposes one deep interface, `read_clipboard_content()`, returning either `ClipboardContent::Files(Vec<PathBuf>)` or `ClipboardContent::Image(ClipboardImage)`. The macOS pasteboard implementation remains private behind that interface. Other platforms return no file URLs and continue through `arboard` image handling.

`app.rs` maps clipboard files into the existing `prepare_explicit_uploads` flow and maps clipboard images into `prepare_clipboard_upload`. Both CLI `ph attach --clipboard` and terminal hooks share the same resolution behavior.

No MIME or extension allowlist is added. A Finder-copied `.mp4` behaves exactly like any other regular file. Existing `max_files`, `max_single_file_bytes`, and `max_total_bytes` settings remain authoritative.

## Error Handling

- No remote target: native paste, without inspecting or uploading clipboard files.
- No files or image: native paste in hooks; `ClipboardNotSupported` error for explicit `ph attach --clipboard`.
- Clipboard file URL cannot be decoded: report a clipboard-content error.
- Clipboard file fails existing validation: report the existing file/path/size error, with no image fallback.
- Upload failure: preserve the existing transport error behavior.

## Verification

- Unit tests cover file precedence, multiple-file preservation, image fallback, unsupported-content passthrough, original file extension naming, and size/count validation through the existing preparation interface.
- Existing unit tests must remain green on all platforms.
- macOS manual verification covers Finder-copying a video file, text paste passthrough, screenshot upload, and a local terminal's native paste behavior.

