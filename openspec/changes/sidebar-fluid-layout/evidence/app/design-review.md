# App against the boards (2026-10-29)

Screenshots in this folder come from the debug build against `fake_service.py`, through the app's screenshot hook (`G512_CONTROL_SCREENSHOT`, `_VIEW`, `_SIZE`, `_STYLE`). Boards and exports are listed in `../../design-plan.md` §9.

## Matches

| Screen | App | Board export |
|---|---|---|
| Keyboard › Lighting, 1280 | `kb-lighting-1280.png` | `critique/` (Settings-style 1280) |
| Keyboard › Lighting, 860 × 780 (no scroll) | `kb-lighting-860.png` | `03-storyboard-keyboard.png` |
| Key Maps, 1280 and 360 | `keymaps-1280.png`, `keymaps-360.png` | `05-keymaps-1280.png`, `05-keymaps-360.png` |
| Games, 1280 and 360 | `games-1280.png`, `games-360.png` | `05-games-1280.png`, `05-games-360.png` |
| Mouse › Lighting, 1280 | `mouse-lighting-1280.png` | `05-mouse-lighting-1280.png` |
| DPI & Profiles, 1280 and 360 | `dpi-1280.png`, `dpi-360.png` | `05-dpi-profiles-1280.png`, `05-dpi-profiles-360.png` |
| Buttons, 1280 and 860 | `buttons-1280.png`, `buttons-860.png` | `critique/` (Settings-style 1280), `03-storyboard-mouse.png` |
| Narrow list | `list-360.png` | `critique/` (narrow list), `03-storyboard-narrow.png` |
| Light style | `light-1280.png` | `05-light-1280.png` |
| No devices | `none-1280.png` | `05-unplugged-none-1280.png` |
| Mouse unplugged on Buttons → Keyboard › Lighting | `unplugged-mouse-1280.png` (frame after the unplug) | `05-unplugged-mouse-1280.png` |
| Keyboard unplugged on Key Maps → Mouse › Lighting | `unplugged-keyboard-1280.png` | — |
| Keyboard missing at start | `keyboard-missing-start-1280.png` | — |
| No profiles: DPI & Profiles and Buttons rows hidden | `no-profiles-1280.png` | — |
| Older service: Keyboard section only | `older-service-1280.png` | — |

## Differences, kept

- **Sidebar width.** The split view sizes it between 220 and 280 px; it is 280 px at 1280 (the boards used 240).
- **Sidebar icons.** These are Adwaita theme icons (`display-brightness`, `view-grid`, `input-gaming`, `find-location`, `input-mouse`), not the board's hand-drawn ones.
- **Key size on Key Maps.** Keys are 34 px at every width; the 1280 board drew 40 px. The card still fills the spare height around the grid.
- **No Devices icon.** It is the single `input-keyboard-symbolic` icon; the board showed a keyboard and a mouse.
- **DPI & Profiles narrow card.** It takes the spare height, so it's taller than the board's short card. The drawing and legend sit centred in it.
- **Buttons at 860 × 780.** The page scrolls by about one callout row; the storyboard expected no scrolling at 860 × 640 with smaller callouts.

## Not checkable here

The test window sits behind the user's other windows, so GTK runs no frame clock for it:
- animations (effect loops, the toast sliding in);
- the selection outline after selecting keys over AT-SPI. The hint reads "4 keys selected", but the frames show the state before that.

The toast is confirmed by the app's log, and the selection by the live hint over AT-SPI. Sidebar rows and libadwaita rows expose no AT-SPI actions, so opening pages by keyboard and "Edit the Gaming Map" need the hands-on pass.
