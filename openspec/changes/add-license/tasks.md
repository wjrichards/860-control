# Tasks

## 1. Licence text

- [x] 1.1 Add `LICENSE` at the repo root with the full, unmodified GNU GPL version 3 text. Take it from https://www.gnu.org/licenses/gpl-3.0.txt, or a verified local copy such as `/usr/share/licenses/add-determinism/LICENSE.GPL3`. Done when `sha256sum LICENSE` matches gnu.org's `gpl-3.0.txt` and the file starts "GNU GENERAL PUBLIC LICENSE / Version 3, 29 June 2007".

## 2. README

- [x] 2.1 Add a `## License` section after "Things to know". It says:
  - "Copyright (C) 2026 Warren Richards";
  - that the code, the GNOME Shell tile, the scripts and the drawings and icons are free software under the GNU General Public License, version 3 or (at your option) any later version, with no warranty, see `LICENSE`;
  - and the trademark line: "860 Control is not affiliated with or endorsed by Logitech. Logitech, G512 and G502 HERO are trademarks of their owners and are named only to identify the hardware it works with."

  Done when the section renders on the Gitea branch and the README's other text is unchanged.

## 3. Third-party tooling

- [x] 3.1 Stop tracking `.claude/`:
  - `git rm -r --cached .claude` (the 12 OpenSpec files stay on disk);
  - replace `/.claude/worktrees/` in `.gitignore` with `/.claude/`.

  Done when `git ls-files .claude` prints nothing, `ls .claude/skills` still lists the OpenSpec skills, and `git status` shows no untracked `.claude/` files.

## 4. Check and publish

- [x] 4.1 Check every place that states a licence agrees:
  - `Cargo.toml` (`license = "GPL-3.0-or-later"`);
  - `data/io.github.g512control.Control.metainfo.xml` (`project_license` GPL-3.0-or-later, `metadata_license` CC0-1.0);
  - `LICENSE`;
  - the README section.

  Done when `appstreamcli validate --no-net data/io.github.g512control.Control.metainfo.xml` still passes and `grep -rn "license" Cargo.toml */Cargo.toml` shows only GPL-3.0-or-later.
- [ ] 4.2 After the merge, push `main` to the `github` and `gitlab` remotes. Done when both repo pages show "GPL-3.0" as the licence.
