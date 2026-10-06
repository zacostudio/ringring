# Changelog

All notable changes to RingRing are recorded here. The format follows Keep a Changelog. Versions raise the patch number only.

## [Unreleased]

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
- `scripts/build.sh <version>`: Developer ID signed, hardened-runtime, notarized and stapled build for Apple Silicon, with `--dry-run`.
- Development-only HTTP control surface (`dev-agent` feature, `RINGRING_DEV_AGENT=1`).
- README shows screenshots of the ring and the settings window (`assets/screenshots/`).

### Changed

- No 1px lines at rest anywhere. Text fields, the shortcut field and the icon button are filled surfaces without a border; the toast and the selected segment float on a lighter fill and a shadow; the ring, its hub and the confirm prompt have no edge stroke and separate from what is behind them by fill and shadow. A focused field shows an accent ring.

### Fixed

- A failure notification no longer holds up other work. Posting one took about 2 s on an async worker, so a shell slot's 1 s time limit could end at 2.5 s when another slot failed at the same moment. The notification is now posted from its own thread; the failure is recorded first.
- A stored shortcut that macOS refuses at launch is no longer only in the log. The failure is recorded in Recent runs under the ring's name (so the menu-bar icon and Settings point at it), and the ring's editor says the shortcut is not registered and why.
- An empty sub-ring or a missing sub-ring is recorded in Recent runs under the slot's name, not under the error sentence.

- Recording a ring's shortcut could freeze the app permanently. Global shortcut registration now runs in one place, on the main thread, one operation at a time, and the capture → register → save order is enforced in Rust.
- The app no longer comes to the front by itself the first time one of its windows appears. It comes to the front only when Settings is opened.
- `scripts/build.sh` runs its checks before it edits the version files, so a failed check leaves nothing edited.

### Technical

- The ring core (hit testing, session controller, NSPanel conversion, keystroke posting, shell runner, store logic) is lifted from Tome and adapted. Tome's "Tome feature" and "workflow" actions are removed from the model, validation, UI and translations.
- Development builds use the identifier `com.zacostudio.ringring.dev` and do not register global shortcuts or a login item.
