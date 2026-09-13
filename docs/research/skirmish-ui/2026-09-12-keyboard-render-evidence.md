# Keyboard A3 render evidence and implementation boundary

2026-09-12. Original-byte research and implementation candidate; production
validation and fresh independent review remain pending. Original gamemd.exe
SHA256 `1cdd1180e49024fbda8ad568caac2e86e856063ff67ab38f62b7d2c7bb84298c`.
Addresses below were decoded from the pinned image with Capstone. No Ghidra
names, comments or analysis boundaries were changed in this pass.

## Frame and parent transaction

`60C5C2` admits A3 to mode1 at `60C7B7`. Common paint `621FC0` queries
`69BBE0` on `A8B238`; that getter reads byte+30D8 (`A8E310`). True calls
`72F540(surface,rect,false)` at `621FD5`. False calls `72E450` then `72E730`.
Both A3 and D5 take `60CF00`'s default background binding at `60D20B..224`:
converter `72E280`/`B0FBCC`, small `B0FB50`, large `B0FA04`. The D5/D6/D7
overlay allow-lists at `60CB49`, `60C989`, `60CD19` exclude A3. The original
A3 extended resource, independently reparsed at `BEF560`, has nineteen
children and no static71C warning animation. Launcher frame reuse must not
transplant the D5 warning child.

BBB52C/notification0 at `4E2396..4E23AD` requires `A8E9A0==1` and writes
state4/result1. Its owner `4E1D9A..4E1DAB` applies `4E1DE0`, then persists
`5FAD10`, before destruction. Dispatcher `48CA03..12` calls `5FBEF0` and
returns state5. This is the same parent transaction as Sound.

## Category, group and capture

Resource4C7 is style50000313, DLU63,83,138,146. With supplied6x13 metrics,
its x/y/width are95/135/207 before ordinary signed offsets. Original
`61745C` uses a24px closed paint face; `618B17..44` sets selection-item height
font+2 (19) and dropdown-row height font+6 (23). Arrow placement reserves20px.
The renderer adds207px to the existing size-driven combo atlas builder and
selects it explicitly; it does not stretch another face's borders.

Group6E9 style50000007 selects `61E700` at `60FE62..6F`. Text begins at
x+10/top (`61E879`), in colorAC18A4. Border top is y+trunc(fontHeight/2)
(`61E828..843`); stock17 gives8. Its two nested rings split the top around
the caption: left segment throughx+8, right resumes atx+textWidth+12
(`61EA50..EAD1`). Outer top/left usesAC1B98 (dark), inner usesAC1B94
(light); opposite edges swap them. Mixed corners average the two colors.
This polarity differs from the outer light ring of plain6208F0.

Hotkey4CD is DLU64,245,124,14, giving96,398,186,23 before offsets.
`775690` translates GetWindowRect without changing exclusive endpoints;
`61ED86..88` increments derived width/height. Thus border input is187x24,
and6208F0 border2 occupies191x28 atx-2/y-2. Text uses the original rectangle
inset4, fontcontext+64 andAC18A4. Although `61EF37` passes0x24, **620F60
ignores that fourth argument**: `62100B..62102E` sends the inset origin,
width/height and three zeros to434CD0. Interpreting0x24 as a vertical-center
directive without reading the callee was incorrect. The candidate paints
atx+4/y+4 with ordinary wrapping/clipping, rather than adding centering.

## Shared implementation and remaining checks

The existing physical compositor now accepts either sidebar or launcher art
and an explicit popup layer. Existing in-game consumers retain their wrapper
and draw order. Launcher Options and Keyboard share only generic background,
right panel and lower strip construction; the640 background follows the native
small-art branch, so D5 at640 is an affected validation neighbor. No Keyboard
binding ownership was added to the renderer.

Title694 uses the existing paint-driven kind1 state and Path-A reveal. The
renderer records a receipt; the frame owner must acknowledge it only after
successful presentation, and reset state for each newly created dialog.
Detailed first-show transition timing remains part of common shell work.
The exact original classifier is `602501` (A3) to `602626` (child694),
returning true; `60A61A..60A63A` assigns kind1, count1 and inactive0.
`600D11` to `600EFF`/`600F08` supplies30ms; `601651` to `60183F`/
`601D0F` supplies increment1; `601D91` to `601F7F`/`6023DA` supplies
highlight range8. These branches do not distinguish the two A3 entry routes.

Candidate geometry tests cover group caption gaps/polarity and the hotkey
extra-pixel border. Neither test is a native whole-frame comparison. Required
validation includes both entry routes, category popup overlay/rows, command
selection/details, hotkey display, title progression, held buttons, Back and
the affected D5/Sound/shared-list neighbors. No build, runtime capture or native
frame parity is claimed by this implementation pass.

## Captured list repeat correction

Fresh review found that holding an A3 Team-list arrow only changed one row.
B8 used the same incomplete gesture. The original shared owner `61C690`
starts capture and a500ms timer on scrollbar button down at `61D383..61D3B2`.
Timer `61D215..61D2C8` clears arrow visuals, checks capture and the current
pointer, steps the admitted arrow once, and rearms25ms even when the pointer
is outside the arrows or the list is already at its end. Crossing to the
opposite arrow changes repeat direction. Reentry resumes on the next scheduled
tick; it does not accumulate missed steps. Release `61D2D3..61D310` clears
capture and kills the timer. Thumb down `61D4AD..61D4C2` captures without
recentering; track down `61D4C4..61D511` changes position once. The timer also
remains active after either of those initial hits.

`ui/shell/list.rs::ListScrollInteraction` now owns this capture, timer deadline
and pressed-arrow projection for A3 and B8. Their input owners poll it through
the event-loop wake deadline, and release/focus loss cancel it. Category
replacement also cancels A3's obsolete list gesture. Existing saved-browser
consumers remain unchanged. Four deterministic regressions cover timing,
outside/reentry, direction/end clamping, thumb down and track capture; they
derive expected timing from the decoded instructions, not an executed timer
oracle. Existing shared geometry separately compares450 native fixture cases.
The bounded pointer admission remains the shared scrollbar rectangle; native
timer arithmetic outside the control/window is not claimed exhaustive here.
Build and ordinary held-arrow runtime validation belong to the integrated
candidate receipt, rather than this source-only implementation note.
