# Docs and workspace checks (tasks 7.1, 7.2)

Checked on 2026-10-28, on commit `acaad89` plus the README and review edits.

## 7.1 README

- **New section, "The app":**
  - the two views, and each view's own device state;
  - the two drawings: click the keyboard for Key Maps; click a mouse button or its callout to change it;
  - the colour lift;
  - motion, and Reduce Animation;
  - where the mouse art comes from (`g512-control/data/*.svg`).
- **Updated:** the Mouse, Buttons and Key Maps sections now refer to the view, the callouts and the drawing.
- **Render check:** Gitea's Markdown API (`POST /api/v1/markdown/raw`) rendered the README with all nine `##` sections, 16 code blocks and 43 list items. "The app" section rendered with its list and bold labels.
- **udev:** the README's `sudo install … 70-g512-control.rules` and the app's "No access" command (`udev::install_command`) install the same two rule lines (keyboard `c33c` and mouse `c08b`). Both then reload udev and re-trigger hidraw. The unit test `installs_every_rule_line` checks the app's side.

## 7.2 Workspace checks (GNOME 50 SDK, `target/sdk.sh`)

| Check | Result |
|---|---|
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `cargo test --workspace` | 44 (g512-control) + 51 + 91 + 0 passed, 2 ignored (hardware) |
| `openspec validate split-keyboard-mouse-views --strict` | valid |
| `openspec validate --specs --strict` | 4 passed, 0 failed (one info note about a long requirement in quick-settings-tile) |
| Flatpak build (`org.flatpak.Builder`, the repo's manifest, into `target/fp-build`, not installed) | exit 0, no warnings. The binary embeds both SVGs (`id="DPI_LEDs"` appears twice). |
