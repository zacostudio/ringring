# Changelog

All notable changes to RingRing are recorded here. The format follows Keep a Changelog. Versions raise the patch number only.

## [Unreleased]

### Added

- Quick shortcut: each ring can have a second shortcut, F1–F24 with or without ⌃ ⌥ ⌘ ⇧. Hold it, move toward a slot and release to run that slot — the hold behavior the single shortcut had before. A bare F1–F12 takes that key from every other app; Settings warns about it. Normal and quick shortcuts share one conflict check across all rings.
- Export files carry the quick shortcut (`quickShortcut`). Older files import without one.

### Changed

- A sub-ring slot is named after the ring it opens. Renaming the slot renames the ring (and every slot that opens it), and renaming the ring renames those slots. Before, the two names drifted apart: the slot showed the new name while the chooser and the sidebar kept the old one. Existing sub-ring slots take their ring's name on upgrade.
- The ring's shortcut now only shows the ring. Releasing the key runs nothing, wherever the cursor is; pick with a click, a number key, or arrows and Enter. Before, a quick press while the mouse drifted could run a slot before the ring was even visible. **Existing shortcuts lose hold-to-run** — set a quick shortcut to get it back.

## [0.1.3] - 2026-10-06

### Added

- An "open app" slot can pass arguments to the app. The field sits under the app chooser. On macOS they are split on spaces (quotes keep one together) and reach the app only when it is started, not when it is already running. On Windows the text is passed as written, and shortcuts and batch files take it too. A slot without arguments opens as before.

## [0.1.2] - 2026-10-06

### Fixed

- Windows: Settings no longer shows sentences written for macOS ("log in to your Mac", "menu bar", "login shell", "another Mac"). Eleven sentences have a Windows version, and the Permissions page shows only what applies on Windows.

## [0.1.1] - 2026-10-06

## [0.1.0] - 2026-10-06

### Added

- Ring menu around the cursor: hold a ring's global shortcut, move toward a slot and release to run it; tap the shortcut to keep the ring open and pick with a click, a number key, or arrows and Enter.
- Slot actions: open an app, file, folder or web address (one chooser picks files and folders); run a shell command (login shell, time limit, process group ended on timeout, output kept even when the limit passes); send keystrokes to the frontmost app; open a sub-ring (up to three levels, no cycles).
- "Ask before running" per slot. On by default for shell commands. The prompt focuses Cancel.
- Menu-bar icon with a menu: open a ring at the cursor, Settings, Pause shortcuts, Launch at login, About, Quit. No Dock icon and no window on launch.
- Settings window: ring editor built around the dial (4–8 slots, action editor, icon picker, move a slot one step, in-place sub-ring creation), shortcut capture with conflict detection, General (language ko/en/ja, theme system/light/dark, pause, launch at login), Permissions (Accessibility status with a link to System Settings), Recent runs, Import & export, About.
- First launch opens Settings with a getting-started state and an optional starter ring (Finder, Safari, Terminal, System Settings, Downloads).
- Import and export of rings as one JSON file. Import only adds rings, shows a summary first, and turns "Ask before running" on for every imported shell, keystroke, open-file and open-app slot. A shortcut the OS refuses is dropped only on imported rings.
- Recent runs: the last 20 shell command results and failed actions, kept in memory only. Shell output is never written to disk, the log or a notification.
- Failed actions are findable without notifications: the menu-bar icon shows `!`, the menu starts with "Show Failed Runs", and Settings marks Recent runs, until the page is opened.
- Text typed in Settings is saved before leaving: another slot, ring or page, closing the window, or quitting. Text that cannot be saved keeps the window open and shows the reason with "Discard input".
- The confirm prompt shows when a long command continues below.
- Storage in one SQLite file with migrations from version 1. Log file under `~/Library/Logs/<identifier>/RingRing.log`. Single instance.
- `scripts/build.sh <version>`: release build for Apple Silicon (`.app` and `.dmg`, not signed, not notarized), with `--dry-run`.
- Development-only HTTP control surface (`dev-agent` feature, `RINGRING_DEV_AGENT=1`).
- README shows screenshots of the ring and the settings window (`assets/screenshots/`).
- Windows build. The ring shows at the cursor without taking focus; shell commands run through `cmd` and a timeout ends the whole process tree; keystrokes go through `SendInput`; launch at login uses the `HKCU` Run key; the icon lives in the notification area; shortcuts read as `Ctrl+Alt+G`; the starter ring holds File Explorer, Edge, Terminal, Notepad and Downloads; an NSIS installer target.

### Changed

- No 1px lines at rest anywhere. Text fields, the shortcut field and the icon button are filled surfaces without a border; the toast and the selected segment float on a lighter fill and a shadow; the ring, its hub and the confirm prompt have no edge stroke and separate from what is behind them by fill and shadow. A focused field shows an accent ring.

### Fixed

- Windows: the ring no longer shows a title bar strip when a tap gives it the keyboard.
- Windows: the buttons in the ring editor's header (Try it, the shortcut field, delete) are clickable; the window's drag strip covered them.
- A failure notification no longer holds up other work. Posting one took about 2 s on an async worker, so a shell slot's 1 s time limit could end at 2.5 s when another slot failed at the same moment. The notification is now posted from its own thread; the failure is recorded first.
- A stored shortcut that macOS refuses at launch is no longer only in the log. The failure is recorded in Recent runs under the ring's name (so the menu-bar icon and Settings point at it), and the ring's editor says the shortcut is not registered and why.
- An empty sub-ring or a missing sub-ring is recorded in Recent runs under the slot's name, not under the error sentence.

- Recording a ring's shortcut could freeze the app permanently. Global shortcut registration now runs in one place, on the main thread, one operation at a time, and the capture → register → save order is enforced in Rust.
- The app no longer comes to the front by itself the first time one of its windows appears. It comes to the front only when Settings is opened.
- `scripts/build.sh` runs its checks before it edits the version files, so a failed check leaves nothing edited.

### Technical

- The ring core (hit testing, session controller, NSPanel conversion, keystroke posting, shell runner, store logic) is lifted from Tome and adapted. Tome's "Tome feature" and "workflow" actions are removed from the model, validation, UI and translations.
- Development builds use the identifier `com.zacostudio.ringring.dev` and do not register global shortcuts or a login item.
