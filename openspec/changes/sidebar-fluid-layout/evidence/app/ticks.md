# Effect loops follow what is on screen (task 5.2), 2026-10-29

Debug build against fake_service.py (keyboard Ripple, mouse Static), G512_CONTROL_TICK_LOG=1. Each line: frames drawn so far, the page, the width, and which drawings are mapped. A 120 Hz display, so one running loop adds about 225 frames per 2 s.

## keyboard-lighting-860
```
03:45:07 ticks: 320 (animations true) page keyboard-lighting width Medium 860px mapped [kb]
03:45:09 ticks: 559 (animations true) page keyboard-lighting width Medium 860px mapped [kb]
03:45:11 ticks: 782 (animations true) page keyboard-lighting width Medium 860px mapped [kb]
```
## keymaps-860
```
03:45:18 ticks: 0 (animations true) page keymaps width Medium 860px mapped []
03:45:20 ticks: 0 (animations true) page keymaps width Medium 860px mapped []
03:45:22 ticks: 0 (animations true) page keymaps width Medium 860px mapped []
```
## list-360
```
03:45:26 ticks: 303 (animations true) page list width Narrow 360px mapped [list-kb list-ms]
03:45:28 ticks: 542 (animations true) page list width Narrow 360px mapped [list-kb list-ms]
03:45:30 ticks: 761 (animations true) page list width Narrow 360px mapped [list-kb list-ms]
```
## buttons-1280
```
03:45:37 ticks: 0 (animations true) page buttons width Full 1280px mapped [ms-buttons]
03:45:39 ticks: 0 (animations true) page buttons width Full 1280px mapped [ms-buttons]
03:45:41 ticks: 0 (animations true) page buttons width Full 1280px mapped [ms-buttons]
```
## keymaps-1280
```
03:45:45 ticks: 0 (animations true) page keymaps width Full 1280px mapped []
03:45:47 ticks: 0 (animations true) page keymaps width Full 1280px mapped []
03:45:49 ticks: 0 (animations true) page keymaps width Full 1280px mapped []
```
