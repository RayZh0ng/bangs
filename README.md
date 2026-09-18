# Bangs

A dynamic notch for macOS and Windows, built with Tauri 2 + React by gxlself
(bundle id `com.gxlself.bangs`). It sits at the top center of the screen, shows live activity next to
the notch, and expands on hover.

- **Now playing**: artwork, progress and play/pause/skip/seek for the current system media session,
  with timed lyrics — the line being sung shows next to the collapsed notch, three lines in the panel
- **File shelf**: drop files on the notch to park them, drag them back out to other apps
- **Dev panel**: Claude Code and Codex CLI sessions with busy/waiting/idle status in one list, and
  the projects VS Code and Cursor have open, grouped per editor with their git branch. Click a row
  to open that project in the editor. Optional alerts open the panel when a session changes from
  busy to waiting or idle, and close it again after a few seconds if nobody looks.
- **Clipboard (macOS)**: history from gxlself's own Paste app (`gxlself.paste-tool`).
  Click text to put it back on the clipboard; image and file entries hand over to Paste's own panel.

Screens with a hardware notch get wings around it; other screens get a virtual notch (or a thin bar,
see the tray menu).

## Develop

Requires Node.js (`^20.19.0 || >=22.12.0`), pnpm and Rust. macOS also needs the Xcode command line tools.

```bash
pnpm install
pnpm tauri dev            # run
pnpm tauri build          # .app/.dmg on macOS, .msi/.exe on Windows
```

Build each platform on that platform (Windows installers cannot be produced on macOS).

The tray menu controls show/hide, expand on hover, idle bar, Claude alerts, display and launch at login.

Lyrics come from QQ Music's public lyric endpoint: only the track title and artist are sent, every
result is cached under the app cache directory, and the tray menu can turn the lookup off entirely.

The clipboard panel asks Paste for its panel with `open pasteg://panel`, which Paste answers in
`AppDelegate.application(_:open:)`; older Paste builds only get activated instead.

## How it works

| | macOS | Windows |
|---|---|---|
| Window | NSPanel (`tauri-nspanel`), non-activating, status level above the menu bar | Topmost, `focusable: false` (`WS_EX_NOACTIVATE`), no taskbar entry; hides during full-screen apps |
| Now playing | MediaRemote, loaded into `/usr/bin/perl` (see below) | `GlobalSystemMediaTransportControlsSessionManager` |
| Notch size | `NSScreen.safeAreaInsets` / `auxiliaryTop*Area` | Virtual only |
| Dev panel | Claude Code / Codex session state and VS Code / Cursor state | Same, using Windows application-data paths |
| Clipboard | Paste Core Data SQLite store, read-only | No Paste integration |
| Lyrics | QQ Music lyric endpoint, cached on disk | Same |

The host window is a fixed 640 x 280 transparent window in logical pixels. The visible notch animates
inside it, and a native thread polls the cursor every 33 ms (`src-tauri/src/geometry.rs`) to:

- make the window click-through everywhere except the current notch rect,
- emit hover and outside-click events,
- stream the pointer position, because WKWebView ignores mouse moves in a panel that is not key.
  CSS therefore uses `[data-hover]` (set by `src/lib/hover.ts`) instead of `:hover`.

**MediaRemote on macOS 15.4+** requires an Apple-entitled process. `src-tauri/native/macos/media_bridge.m`
is built by `src-tauri/build.rs` into `src-tauri/resources/libbangs_media.dylib`, loaded into
`/usr/bin/perl`, and talks to the app over stdio (JSON lines out, commands in). This relies on system
behavior Apple could change in a future release.

The dev integration polls every two seconds, reading `~/.claude/sessions/*.json`, the newest
`~/.codex/sessions` rollout logs (a session counts as running while its log has an unfinished
`task_started`), the editors' `User/globalStorage/storage.json`, and project `.git/HEAD` read-only. It checks running
processes to ignore stale sessions and closed editors. Opening a project uses the editor on macOS
or its `code` / `cursor` CLI on Windows, with a folder-reveal fallback.

The Paste integration opens the user's local Core Data store (`PasteTool.sqlite`) read-only and
polls the latest 24 entries every three seconds. It checks Paste's sandbox container, then
`~/Library/Application Support/Paste/`. The tab appears only when the store is readable.
Bangs never writes to the Paste database; copying text writes to the system clipboard.

```text
src/                       React UI
  App.tsx                  native events, drag/drop and agent alerts
  components/              Notch shell, compact/expanded views, music and shelf panels
    DevPanel.tsx           agent sessions and per-editor project rows
    PastePanel.tsx         clipboard history and copy/open actions
  store/                   zustand stores: notch state machine, media, shelf
    dev.ts                 agent sessions, workspaces and busy-to-idle/waiting detection
    paste.ts               clipboard history and copy feedback
  lib/layout.ts            notch sizes per mode (keep WINDOW in sync with geometry.rs)
  lib/hover.ts             pointer-driven [data-hover] workaround
    lyrics.ts              current lines and the line-at-time lookup
src-tauri/src/
  lib.rs                   app setup, bootstrap and commands
  geometry.rs              window placement, hit rect, cursor tracker, display watcher
  platform/{mac,win}.rs     window setup, cursor, notch metrics, editor launch, full-screen handling
  media/{mod,mac,win}.rs    shared media state and platform now-playing providers
  dev.rs                   read-only Claude/Codex/editor polling and project opening
  lyrics.rs                lyric lookup, LRC parsing and the on-disk cache
  paste.rs                 macOS read-only Paste store, text copy and panel hand-off
  paste_other.rs           unavailable Paste stub on other platforms
  shelf.rs                 file metadata, open/reveal, drag preview
  tray.rs, settings.rs      tray menu and persisted settings
scripts/generate-icons.swift  app/tray/drag icons (then `pnpm tauri icon src-tauri/icons/app-icon.png`)
```
