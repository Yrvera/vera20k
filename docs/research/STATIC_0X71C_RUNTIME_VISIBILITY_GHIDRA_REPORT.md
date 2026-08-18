# Static 0x71C Runtime Visibility — Ghidra Report

Date: 2026-05-19

Scope: prove or disprove that Static control `0x71C` on main-menu dialog `0xE2`
draws visible content during the live YR main-menu lifetime. Re-investigation
of the open question carried by:

- `MAIN_MENU_DIALOG_0XE2_OWNERDRAW_VISUAL_FOLLOWUP_GHIDRA_REPORT.md` §2
- `MAIN_MENU_DIALOG_0XE2_FULL_VISIBLE_COMPOSITION_GHIDRA_REPORT.md` §"`0x71C` blank/transparent static"

This pass exhaustively traced every code site that names control id `0x71C` in
`gamemd.exe` (push-imm32, push-imm16, mov, cmp-eax, cmp-[mem]) and every
caller of every classifier function that pairs dialog `0xE2` with control
`0x71C`. No Rust code was changed. No Ghidra annotations were written.

## Executive Summary

**Active in YR: No** for any visible content on the standard `0xE2` main-menu
path. The control is fully invisible in the live YR code path. The classifier
functions that *would* treat `(0xE2, 0x71C)` as a SHP-animation control exist
in the binary but are orphan code with no callers and no sending site of the
custom messages required to enable them.

Specifically:

1. The only `push 0x71C` immediate in the entire `.text` segment is inside
   `ToggleMpScoreControls_0046DE20`, an MP-scoreboard show/hide helper that
   is not invoked anywhere Ghidra tracks and that lives in the MP-scoreboard
   ID space (`0x732`, `0x72F`, `0x798..0x79B`, `0x6D2`, ...), not the main
   menu.
2. Three classifier functions return non-zero for the pair `(dialog ∈ {0xE2,
   …}, ctrl = 0x71C)`: `FUN_00603240` (returns 1), `FUN_006033f0` (returns
   100), `FUN_00603870` (returns a SHP pointer). One of them
   (`FUN_006033f0`) is reachable from `OwnerDraw_Static_006153E0` case
   `0x4D3`, but only when the control's kind has already been set to 4 and a
   SHP attached. On the standard `0xE2` path, both fields start at zero and
   nothing ever sends a message that changes them.
3. No `push 0x4D3`, `push 0x4DA`, `push 0x4DB`, or `push 0x4DC` immediate is
   followed by `push 0x71C` in the binary. No `0x4D3` send site targets
   control `0x71C` on any dialog.
4. The only function that DOES write `[esi+0x70] = 4` (kind = SHP-anim) and
   attach a SHP for `(0xE2, 0x71C)` lives in the unnamed code region
   `0x0060A338..0x0060AAxx`, has zero xrefs from anywhere in `gamemd.exe`,
   and shares basic blocks with the live classifier family but is itself
   never called.

`0x71C` is also not in the tooltip table at `FUN_006040B0`: no
`STT:*` mapping uses control id `0x71C`. The `STT:MainButtonYuriWebSite`
tooltip is bound to control `0x55F`, not `0x71C`.

`0x71C` IS recognized by `ResizeShellChildControl_0060C0C0` via
`FUN_00608cd0`, so the static gets repositioned during the shell fullscreen
resize pass (via `FUN_0060b1d0`) — but reposition of an empty static does not
produce visible output. Its rect (~`92x54 px` at upper-right) sits over the
right-panel SHP stack which paints under it.

## Verified Findings

### 1. Exactly one `push 0x71C` immediate exists in the binary

Evidence:

- Byte pattern search `68 1C 07 00 00` returned exactly one match:
  `0x0046DF98`, inside `ToggleMpScoreControls_0046DE20 @ 0x0046DE20`.
- Byte pattern search `66 68 1C 07` (push imm16) returned no matches.

`ToggleMpScoreControls_0046DE20` is a contiguous chain of `GetDlgItem(parent,
ctrl_id)` + `ShowWindow(child, param_2)` calls over MP-scoreboard control
ids. Its `0x71C` branch is the **only** one with inverted visibility
(`ShowWindow(child, param_2 == 0)`), meaning `0x71C` is hidden when other
score controls are shown.

This helper has zero Ghidra-tracked callers
(`get_function_callers` returns empty), and its `param_1` is reached via
fastcall through register, so the lack of explicit callers suggests it is
either invoked from an MP-only path Ghidra has not resolved, or it is itself
dead. Either way it is **not on the standard `0xE2` initial main-menu code
path**: the surrounding control ids are not `0xE2` controls.

Confidence: High for "only `push 0x71C` site"; High for "not main-menu
relevant" based on neighboring control IDs.

Active in YR: Unknown for the helper itself; not relevant for `0xE2`.

### 2. Classifier predicates recognize `(0xE2, 0x71C)`

Evidence: four functions (one defined, three implicit), all decompiled in
this pass, return non-zero for `dialog_id ∈ {0x94, 0xD8, 0xF5, 0xE2, 0xD5,
0x101, 0x129, 0xD7, 0xBB, 0x100, 0xD6, 0x125, 0x122, 0x112, 0xE7, 0x116,
0x11D, 0x11C, 0xFE, 0x10F, 0x117, 0x114, 0xE6, 0xF3, 0xF4, 0x2BC, 0x10E,
0x108, 0xBC6, 0xBC7}` AND `ctrl_id == 0x71C`:

| Function | Returns when matched | Live caller |
|---|---|---|
| `FUN_00603240` | `1` (bool) | `0x0060A9E2` (orphan, no xref) |
| `FUN_006033f0` | `100` (timer interval, ms) | `OwnerDraw_Static_006153E0 @ 0x00615E6C`, case `0x4D3` |
| `FUN_00603870` | SHP/asset pointer (via `0x004A38D0`) | `0x0060A9B2` (orphan, no xref) |
| `FUN_00608cd0` | `true` (bool) | `ResizeShellChildControl_0060C0C0`, `FUN_0060A180` |

`FUN_00603240` and `FUN_00603870` exist as defined functions but their only
known callers are inside the unnamed `0x0060A338..` region which has no
xrefs from anywhere.

Confidence: High; all four functions decompiled and the `0xE2` membership
read directly out of the disjunction.

Active in YR: `FUN_006033f0`, `FUN_00608cd0` Yes (live callers exist);
`FUN_00603240`, `FUN_00603870` No (orphan callers only).

### 3. The orphan SHP-attach path

Evidence: byte pattern `C7 46 70 04 00 00 00` (`mov [esi+0x70], 4`) returns
four matches at `0x0060A928, 0x0060A949, 0x0060A96A, 0x0060A987`, all inside
an unnamed function body that:

- writes `[esi+0x70] = 4` (kind = SHP-anim);
- conditionally calls `FUN_00603870` at `0x0060A9B2` to fetch the SHP
  pointer for the current `(dialog, ctrl)` pair;
- calls `FUN_00603240` at `0x0060A9E2` as a boolean gate;
- stores SHP and filename into the static's record at offsets `+0x78`,
  `+0x74`.

This is exactly the routine that would arm `(0xE2, 0x71C)` for SHP-animated
visible output. The full function entry is not marked in Ghidra
(`get_function_by_address` returns null at every plausible head address from
`0x0060A300` through `0x0060A7C0`), and `get_xrefs_to` returns no references
to any address in the `0x0060A300..0x0060AAxx` range.

The two non-orphan paths that share basic blocks with this region — at
`0x006035E2` (target of a call from `0x0060A994`) and `0x0060385A` (target
from `0x0060A99C`) — both land **inside the bodies** of
`FUN_006033f0` / `FUN_00603870` mid-function, not at their entries. This is
consistent with shared-tail-block code that the live classifiers never
execute.

Confidence: High that this region exists. High that it is uncalled by any
function Ghidra has analyzed. Medium that it is truly unreachable —
indirect-call (vtable/callback) is not fully excluded by static xref alone,
though the surrounding code style does not look like a vtable target.

Active in YR: No (no caller in `0xE2` path).

### 4. `OwnerDraw_Static_006153E0` has no other SHP-kind-4 entry for `0x71C`

Evidence: `OwnerDraw_Static_006153E0` decompiled in full. Its custom message
switch handles: `0x47, 0xF, 0x113, 0x2/3/5, 0x497, 0x498, 0x4B1, 0x4B2,
0x4B4, 0x4D3, 0x4D4, 0x4D5, 0x4D6, 0x4D7, 0x4DF, 0x4E0, 0x4E1, 0x4E2, 0x4E3,
0x4E4, 0x4EE, 0x4F0`. The `0x497` init path sets `piVar11[0x1C] = 0` (kind),
`piVar11[0x1E] = 0` implicitly, `piVar11[0x2B] = 0xC` (text color flags),
`piVar11[0x3B] = DAT_00AC18A4` (text color).

Case `0x4D3` is the only path that reads `FUN_006033f0()` (timer interval),
and it short-circuits to return `0` unless `piVar11[0x1E] != 0` AND
`piVar11[0x1C] == 4`. Neither field is written by any case in
`OwnerDraw_Static_006153E0` for kind = 4. The kind-1 (text-anim) and kind-2
/ kind-3 / kind-4 paint paths exist in the `WM_PAINT` handler, but they all
require the same pre-conditions to ever execute.

No message in the binary (verified by `push 0x4D8/0x4DA/0x4DB/0x4DC/0x4DD`
searches) targets control `0x71C`.

Confidence: High.

Active in YR: the `0x4D3` handler is live for other controls (e.g.
score-dialog `0x6EA/0x6EB/0x6EC` are armed by `push 0x4D3` sites at
`0x0052EE1E / 0x0052EE58 / 0x0052EE98`); no equivalent send site exists for
`0x71C` on any dialog.

### 5. `ResizeShellChildControl_0060C0C0` repositions `0x71C` on `0xE2` but does not paint it

Evidence: `ResizeShellChildControl_0060C0C0` decompiled in full. After
ruling out style-`0xB`/button branches, it calls `FUN_00608cd0` which
returns true for `(0xE2, 0x71C)`. The matched branch invokes `FUN_0060b1d0`,
which performs:

- `GetWindowRect(parent, &local_20); GetWindowRect(self, &local_10)`
- right-edge anchor: `x = ((parent.right - inset) - control_width - centering)`
- y is preserved (vertical inset only at high res)
- `MoveWindow(self, x, y, w, h, 0)`

This is the same right-panel anchor used by control `0x71D` (bottom-right
version text). Reposition of an empty static does not produce visible
output; the control's `WM_PAINT` for kind = 0 with empty text validates
the rect and exits without drawing.

Confidence: High.

Active in YR: Yes for the reposition; the visible output is still nothing.

### 6. No tooltip text and no parent-side paint for `0x71C`

Evidence:

- `FUN_006040B0` (tooltip text table) was decompiled. Search of the full
  decompilation text for `0x71c` and `0x71C` yielded no matches.
  `STT:MainButtonYuriWebSite` maps to control `0x55F`.
- `MainMenuDialog0xE2_Proc_00531F60`, `FUN_00531CC0`, `FUN_0052B9B0`,
  `FUN_00622B50`, and `WM_PAINT_Handler @ 0x00621E90` do not call
  `GetDlgItem(parent, 0x71C)` and do not include `0x71C` in any
  SendMessage/SendDlgItemMessage. Confirmed by absence of additional
  `push 0x71C` immediates and by full decompilation of those functions.

Confidence: High.

Active in YR: Yes for the absence (verified-negative).

## What the parent paints under the `0x71C` rect

The control's rect of `447,29,61,33 DLU` (`671,47,92,54 px` at 800x600)
sits inside the right-panel area. During parent `WM_PAINT`, `RightPanel__Draw
@ 0x0072E450` paints the standard right-panel SHP stack (`SDTP.SHP`,
`SDBTNBKGD.SHP`, `SDBTNANM.SHP` frame 10 conditional, `SDBTM.SHP`, and the
left-panel cap SHP) plus the generic shell background. Whatever the player
sees inside that rect is the SHP-stack composite, not anything driven by the
static itself.

Confidence: High; verified in
`MAIN_MENU_DIALOG_0XE2_FULL_VISIBLE_COMPOSITION_GHIDRA_REPORT.md`.

## TS-vs-YR Filter

The orphan SHP-attach path at `0x0060A338..0x0060AAxx` and the score-control
helper at `ToggleMpScoreControls_0046DE20` are best-fit candidates for
Tiberian Sun legacy code:

- The dialog set in the classifier disjunction includes `0xBC6/0xBC7/0x108`
  (Skirmish era), `0x129/0x125/0x122/0x117/0x116/0x114/0x112/0x10F/0x10E`
  (alt-shell dialogs), and many other shell ids. The same predicate is
  consumed by live functions (`OwnerDraw_Static_006153E0` case `0x4D3`,
  `ResizeShellChildControl_0060C0C0`), but never in conjunction with a
  message that actually attaches a SHP to `0x71C`.
- `ToggleMpScoreControls_0046DE20` carries score-dialog control IDs that are
  not part of standard YR skirmish-end flow; nothing visible to the user
  triggers it on the `0xE2` path.

The classifier infrastructure looks like it once supported a per-dialog
animated SHP at control `0x71C` (some kind of shared sidebar/right-panel
animation gadget) that has been removed at the call-site level but still
appears in the data tables. Without a live caller, this code is dormant.

Confidence: Medium for "TS legacy"; High for "not active in standard YR
main menu".

Active in YR: No.

## Conclusion

Question (a) — "what visible content does `0x71C` on `0xE2` render?" — is
answered: **nothing** during the live YR main-menu lifetime.

Question (b) — "is the control unused/invisible by default in YR?" — is
answered: **yes**, with verified evidence: no message attaches a SHP, sets
kind, or alters text; no `GetDlgItem(_, 0x71C)` call is made by any
main-menu function; the only `push 0x71C` site is in a score-controls
helper that is itself unreachable from the main menu; the orphan SHP-attach
path has no callers. The static's WM_PAINT for kind = 0 with empty title and
no SHP/movie just validates its rect.

**Recommendation for Rust port:** keep the existing behavior — do not draw
anything for `0x71C` on the initial main menu. The control's rect overlaps
the right-panel SHP stack; the parent-paint stack already covers that area.

## Active in YR Summary

- Static `0x71C` visible output: **No.**
- Static `0x71C` reposition via `FUN_0060b1d0`: **Yes** (no visible effect).
- Classifier infrastructure for `(0xE2, 0x71C)`: **Yes** (live but
  dormant — never fed the prerequisite SHP-attach message).
- `ToggleMpScoreControls_0046DE20` toggling `0x71C`: **No** on main-menu
  path; helper itself has no Ghidra-tracked callers.

## Open Questions (out of scope, listed for swarm reconciliation)

1. Sibling controls `0x71A` (RA2TS movie) and `0x71B` (per other reports,
   appears in score / MP contexts) — explicitly out of this scope; flagged
   for sibling reports.
2. The unnamed function body at `0x0060A338..0x0060AAxx` — possibly worth a
   targeted reachability pass with `decompiler_callgraph` + runtime
   watchpoint, but not required to resolve this question.
3. `ToggleMpScoreControls_0046DE20` is referenced by no Ghidra-tracked
   caller; either a vtable / dispatch-table entry or a true dead function.
   Reachability not required for the `0xE2` answer.

## Sources

Ghidra functions decompiled or read in this pass (read-only):

- `OwnerDraw_Static_006153E0 @ 0x006153E0`
- `ResizeShellChildControl_0060C0C0 @ 0x0060C0C0`
- `FUN_00602B90` (paired classifier sibling for context)
- `FUN_00602490` (paired classifier sibling for context)
- `FUN_00603240` (returns 1 for `(0xE2, 0x71C)`)
- `FUN_006033F0` (returns 100 ms for `(0xE2, 0x71C)`)
- `FUN_00603870` (returns SHP pointer for `(0xE2, 0x71C)`)
- `FUN_00608CD0` (returns true for `(0xE2, 0x71C)`)
- `FUN_0060A180` and `FUN_0060A250` (visible-count helpers)
- `FUN_0060B1D0` (right-edge reposition routine)
- `FUN_006040B0` (tooltip table — full text searched for `0x71c`)
- `ToggleMpScoreControls_0046DE20`

Raw memory inspected for unnamed function bodies:

- `0x00602DC0..0x00602F80` (sibling unnamed classifier body)
- `0x006031A0..0x006031E0` (sibling unnamed classifier body)
- `0x0060A300..0x0060AAA0` (orphan SHP-attach body)

Byte-pattern searches across `.text`:

- `68 1C 07 00 00` (push imm32 0x71C) → 1 match
- `66 68 1C 07` (push imm16 0x71C) → no matches
- `3D 1C 07 00 00` (cmp eax, 0x71C) → 8 matches
- `B8/B9/BA 1C 07 00 00` (mov reg, 0x71C) → no matches
- `81 F8/F9/FA/FB 1C 07 00 00` (cmp reg, 0x71C, 16-bit) → no matches
- `81 7D/7C ?? 1C 07 00 00` (cmp [mem], 0x71C) → no matches
- `81 BD ?? ?? ?? ?? 1C 07 00 00` (cmp [ebp+disp32], 0x71C) → no matches
- `66 3D / 66 81 F8 / 66 81 7D / 66 81 7C` (16-bit cmp variants) →
  no matches
- `C7 46 70 04 00 00 00` (`mov [esi+0x70], 4`) → 4 matches in orphan body
- `68 D3 04 00 00` (push 0x4D3) → 7 matches, none paired with `push 0x71C`
- `68 D8/DA/DB/DC 04 00 00` (push 0x4D8..0x4DC) → matches exist but none
  paired with `push 0x71C`

Prior reports referenced:

- `MAIN_MENU_DIALOG_0XE2_OWNERDRAW_VISUAL_FOLLOWUP_GHIDRA_REPORT.md`
- `MAIN_MENU_DIALOG_0XE2_FULL_VISIBLE_COMPOSITION_GHIDRA_REPORT.md`
- `MAIN_MENU_SIDEBAR_GHIDRA_REPORT.md`
- `MAIN_MENU_VISUAL_ASSETS_GHIDRA_REPORT.md`
- `SKIRMISH_OWNERDRAW_CALLBACKS_FOLLOWUP_GHIDRA_REPORT.md`
