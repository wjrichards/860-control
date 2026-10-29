# Proposal: state the licence

## Why

The project is public on GitHub and GitLab, but it has no licence text. `Cargo.toml` and the AppStream metainfo say GPL-3.0-or-later, yet there is no `LICENSE` file, so both sites show "no license", and the GPL expects its full text to ship with the code. Without it, nobody may reuse the code. The repo also tracks OpenSpec's MIT-licensed tooling in `.claude/` without its notice.

Decided with the owner (2026-10-29, explore session):
- **One licence for everything:** GPL-3.0-or-later for the Rust crates (`g512-hidpp`, `g512d`, `g512-control`), the GNOME Shell tile, the scripts and the assets (the G502 ink drawings, the app and macros icons, the screenshots).
- **Copyright holder:** Warren Richards. Commits co-authored with Claude belong to him under Anthropic's terms.
- **Not split:** `g512-hidpp` stays GPL too. Warren can still contribute his own code to libratbag (MIT) or OpenRGB (GPL-2.0) under their licences himself.
- **The app icon** (`data/io.github.g512control.Control.svg`) was hand-written in this project (`bfe7304`, 2026-10-22), so it is covered too.

## What Changes

- **`LICENSE`:** the full, unmodified text of the GNU GPL version 3, at the repo root.
- **README:** a **License** section saying "Copyright (C) 2026 Warren Richards", that the whole project is GPL-3.0-or-later with no warranty, and pointing to `LICENSE`. It adds a trademark line: the project isn't affiliated with or endorsed by Logitech, and "Logitech", "G512" and "G502 HERO" are named only to identify the hardware it works with.
- **`.claude/` is no longer tracked:** the 12 OpenSpec skill and command files are removed from the index and `/.claude/` is ignored. They stay on disk and keep working locally; they're tooling, not part of the product, and are MIT-licensed by their own project.
- **Unchanged:** the AppStream metadata licence stays CC0-1.0, as AppStream recommends, and `project_license` stays GPL-3.0-or-later. `Cargo.toml` already says GPL-3.0-or-later. No per-file SPDX headers for now; the root `LICENSE` covers every file.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None. Licensing changes no behaviour, so this change has no spec deltas (`skip_specs: true`).

## Impact

- **New:** `LICENSE`.
- **Edited:** `README.md` and `.gitignore`.
- **Removed from the index only:** `.claude/commands/opsx/*.md` and `.claude/skills/openspec-*/SKILL.md`. They're kept on disk, and their history stays in git.
- **Mirrors:** GitHub and GitLab get the change with the next push of `main`, after which both should show "GPL-3.0".
- **No code, build or package changes.**
