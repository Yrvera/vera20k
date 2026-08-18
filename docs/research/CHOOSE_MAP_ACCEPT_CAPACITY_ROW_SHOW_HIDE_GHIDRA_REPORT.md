# Choose Map Accept — Player Capacity Row Show/Hide

**Status:** COMPLETE  
**Date:** 2026-06-01  
**Scope:** FUN_006acee0 accepted-return path; helpers FUN_006addf0, FUN_006adf00, FUN_006ae080; waypoint-count function 0x005E6520 (mislabeled `CDFileClass__Constructor`).  
**Question:** Does accepting a different-capacity map in Choose Map visibly add/remove player rows in the setup shell (dialog 0x102), or does it only gate Start-position dropdown options?

---

## Label Drift Finding — 0x005E6520

The label `CDFileClass__Constructor @ 005E6520` in Ghidra is **stale and wrong**.  

Verified via `decompile_function 0x005E6520`: the function takes an integer map-list index in ECX (`__fastcall` param_1), opens the map's INI via `CCFileClass`, reads `[Waypoints]` keys 0–7 via `CCINIClass__ReadInt`, counts those that return != -1, and returns that count as `iVar2`. If no waypoints are present it falls back to `[RandomMap] NumPlayers=`. It contains a full INI reader stack (FileStraw, CCINIClass, GenericNode cleanup) — no CDFileClass constructor logic whatsoever.

**True identity:** A map-capacity reader — call it `GetMapWaypointCount(map_index: int) -> int`. It returns the number of player-start waypoints (0–7) defined in the specified map's INI file.

The address 0x005E6520 is real and is called by FUN_006addf0 at instruction `006ADE0E: CALL 0x005E6520` (verified via `get_assembly_context` xref from FUN_006addf0 entry). The doc's claim that this address computes player capacity from [Waypoints] 0..7 is **correct behavior description, wrong label**. The label is the stale artifact.

Active in YR: **Yes** — directly on the skirmish setup accepted path.

---

## FUN_006addf0 — Row Show/Hide Orchestrator

**Address:** 0x006ADDF0  
**Callers:** FUN_006acee0 (skirmish setup shell) and FUN_006ae6e0 (verified via `get_function_callers 0x006addf0`).  
**Callees:** `GetMapWaypointCount @ 005E6520`, FUN_006adf00 (show rows), FUN_006ae080 (hide rows), FUN_004E5940, FUN_004E5ED0, FUN_0069ADF0 (verified via `get_function_callees 0x006addf0`).

**Decompile summary** (verified via `decompile_function 0x006addf0`):

```
param_1 = dialog HWND (ECX, __fastcall)
param_2 = previous map index (EDX)
param_3 = new map index (stack)

if param_2 == -1:
    old_count = 8   // sentinel: no previous map → assume max
else:
    old_count = GetMapWaypointCount(param_2)   // old map's player count

new_count = GetMapWaypointCount(param_3)       // new map's player count
delta = new_count - old_count

if delta > 0:                    // new map supports more players
    FUN_006adf00(delta)          // SHOW delta more rows
elif delta < 0:                  // new map supports fewer players
    FUN_006ae080(...)            // HIDE rows beyond new_count
    
// Additional: if both maps valid and online, update dropdowns in rows 1..7
// Also re-applies team assignment defaults via FUN_004E5ED0
```

Active in YR: **Yes** — skirmish setup shell, always reachable on map accept.

---

## FUN_006adf00 — Show Rows

**Address:** 0x006ADF00  
**Verified via:** `decompile_function 0x006adf00`

Iterates over a range of opponent row indices. For each row index in [param_2 .. param_2+param_3), maps to dialog control IDs (0x50B, 0x50E, 0x516, 0x51A, 0x51B, 0x51C, 0x51D for rows 0–6), then calls:
- `GetDlgItem` for the opponent-type combo (0x50B etc.), team combo, difficulty combo, color combo, and start-position combo
- `ShowWindow(hwnd, 5)` — SW_SHOW on all 5-6 controls for each row

This makes entire opponent row groups **appear** in the dialog.

Active in YR: **Yes**.

---

## FUN_006ae080 — Hide Rows

**Address:** 0x006AE080  
**Verified via:** `decompile_function 0x006ae080`

Takes `param_2` = first row index to hide. Iterates from param_2 up to 7. For each row:
1. First pass: resets the opponent-type combo (sends CB_GETCOUNT → CB_GETITEMDATA → CB_SETCURSEL with -1 for "Close" entries), then calls `FUN_006ADC20` (likely combo clear/repopulate).
2. Second pass: calls `GetDlgItem` for all 5-6 controls in the row and calls `ShowWindow(hwnd, 0)` — SW_HIDE on every control in the row group.

This makes entire opponent row groups **disappear** from the dialog.

Active in YR: **Yes**.

---

## FUN_006acee0 — Call Site for FUN_006addf0

**Address:** 0x006ACEE0  
**Verified via:** `decompile_function 0x006acee0`

The `0x5AA` message case (choose-map accepted return from FUN_005E68A0). The exact call sequence after the map is accepted:

```
1. FUN_005E68A0() -> runs Choose Map dialog, returns 1 (accepted) or 2 (cancel)
2. If result == 2: restore DAT_00A8B250/DAT_00A8B254 to saved values, return.
3. Otherwise (accepted):
   a. CDFileClass__Constructor()          // clears/reinits something (label = stale)
   b. (*vtable+4)()                       // virtual call on DAT_00A8B23C
   c. FUN_004E4FC0 / FUN_004E5310 / FUN_004E5D60   // update dropdowns
   d. FUN_006ADDF0(DAT_00A8B254)         // <-- ROW SHOW/HIDE with NEW map index
   e. ShowWindow(param_1, 5)             // re-show setup shell
   ... (remaining setup: check load validity, rebuild random-map preview, etc.)
```

`DAT_00A8B254` at call site `d` holds the **new** map index (committed by `FUN_005E7160` inside the modal during accept). The **previous** map index was saved as `iVar14 = DAT_00A8B254` at function entry — that becomes `param_2` of FUN_006addf0 via EDX.

Active in YR: **Yes** — this is standard skirmish map selection.

---

## Verified Answer to the Key Question

**When the player accepts a map with a different waypoint count in Choose Map, gamemd DOES visibly add or remove opponent rows in the setup shell.**

- New map supports MORE players than old: `FUN_006ADF00` calls `ShowWindow(SW_SHOW)` on all control groups for the new rows.
- New map supports FEWER players than old: `FUN_006AE080` calls `ShowWindow(SW_HIDE)` on all control groups for the rows beyond the new cap.
- Equal capacity: delta = 0, neither show nor hide is called; rows unchanged.

This is **not** just a dropdown gating — the rows themselves are hidden/shown. The Rust implementation has a **real player-visible gap**.

Active in YR: **Yes, unconditionally on every map accept**.

---

## Rust Parity Gap

Current Rust (from swarm context):
- `commit_choose_map_selection` sets `selected_map_idx` + invalidates preview + restarts label reveals.
- Does NOT recompute player capacity or hide/show opponent rows.
- Renders a **fixed** set of opponent rows (layout.flags / shell.opponents in `src/app_skirmish_shell_render.rs:349-357`).
- Only validates capacity at launch (`src/ui/skirmish_shell/state/launch.rs:90-93`).
- Start-position dropdown options ARE clamped to map waypoint count (`src/ui/skirmish_shell/state/combos.rs:295-299`).

The gap: rows exceeding new map capacity remain visible after map change. In gamemd they are hidden immediately. The player can see (and interact with) rows that will be rejected at launch.

---

## Implementation Handoff

### Change Required
After `commit_choose_map_selection` accepts a new map, recompute its waypoint count and adjust the visible opponent row count in the setup shell immediately — hiding rows beyond the new cap, or showing rows up to the new cap if it increased.

### Handoff Items

1. **Waypoint count read:** Read `[Waypoints] WaypointN` (N = 0..7) from the accepted map's INI; count non-empty entries → `new_capacity: usize` (1–8). Fallback: if zero, read `[RandomMap] NumPlayers=`, default 8.

2. **Row show/hide on accept:** In the skirmish shell state, after map commit, set `opponent_row_count = new_capacity - 1` (max opponents = capacity - 1 for human player). Apply `ShowWindow` equivalents (Rust: set row visibility flags) immediately — do not defer to launch validation.

3. **Reset hidden rows:** When hiding a row, reset it to a default state (e.g., opponent-type = "Close"/inactive). gamemd resets the combo selection to the "Close" entry (CB_SETCURSEL -1 pattern in FUN_006AE080) before hiding — prevents re-show of a stale opponent type.

### Affected Rust Surface
- `src/ui/skirmish_shell/state/` — `commit_choose_map_selection`, capacity tracking
- `src/app_skirmish_shell_render.rs:349-357` — fixed opponent row range must become dynamic
- `src/ui/skirmish_shell/state/launch.rs:90-93` — launch validation remains, but is now a redundant safety net

### Acceptance Scenarios and Proposed Test Names

| Scenario | Test Name |
|---|---|
| Accept 2-player map when 4-player was selected: rows 2–3 disappear | `test_map_accept_shrinks_visible_opponent_rows` |
| Accept 8-player map when 2-player was selected: rows 2–7 appear | `test_map_accept_grows_visible_opponent_rows` |
| Accept same-capacity map: rows unchanged | `test_map_accept_same_capacity_no_row_change` |
| Cancel choose-map: rows unchanged, state restored | `test_map_accept_cancel_preserves_rows` |

Risk: medium. The fix touches shell render state that is currently static; mishandling the row-reset step could leave stale opponent configs attached to hidden rows that re-appear on a third map change.

---

## Negative Facts / Do Not Do

1. **Do NOT make row visibility a launch-time-only gate.** gamemd hides rows immediately on accept, not deferred. Verified: `FUN_006ADDF0` is called synchronously before `ShowWindow` re-shows the setup shell. (verified via `decompile_function 0x006acee0`)

2. **Do NOT treat `CDFileClass__Constructor @ 005E6520` as a CDFile class.** It is a map waypoint counter reading `[Waypoints]` from map INI. Verified via `decompile_function 0x005e6520`.

3. **Do NOT call the second CDFileClass-labeled function at 0x005E7BF0 for capacity.** That is a different function also mislabeled. Only 0x005E6520 is the waypoint counter used in this path.

4. **Do NOT show/hide rows independently per-control.** gamemd hides all 5-6 controls in a row group atomically via FUN_006ADF00/FUN_006AE080. Rust must match the atomic group hide. (verified via `decompile_function 0x006adf00` and `0x006ae080`)

5. **Do NOT assume "cancel" changes rows.** On modal result 2 (cancel), FUN_006acee0 restores `DAT_00A8B250/DAT_00A8B254` to pre-open values and returns without calling `FUN_006ADDF0`. Rows are not touched. (verified via `decompile_function 0x006acee0`)

---

## Remaining Uncertainty

- **Row group control IDs:** The mapping `row_index → {0x50B, 0x50E, 0x516, 0x51A, 0x51B, 0x51C, 0x51D}` is the combo IDs; the other 4-5 sibling controls (team, difficulty, color, start-pos) come from `FUN_004E3320/FUN_004E41D0/FUN_004E37D0/FUN_004E4E60/FUN_004E5940` return values, which are not further investigated here. Rust uses its own control layout — mapping these IDs to Rust widgets is straightforward but not done here.

- **`param_2` on first call (no previous map):** The decompile shows `FUN_006addf0(DAT_00A8B254)` as a single-arg pseudocode call. The true `__fastcall` passing of `param_2` (old map index via EDX) was inferred from assembly at 006ADE0E. If `param_2 == -1` at first shell open, `local_4 = 8` (all 8 rows assumed hidden initially). The exact first-open scenario for Rust (going from "no map selected" to first map accept) needs a check against `param_2 = -1` path — Rust should show rows 0..new_capacity-1 and hide the rest.

- **FUN_006AE6E0:** A second caller of FUN_006ADDF0 exists at `006AE6E0`. Its context (multiplayer lobby vs. skirmish) was not investigated — may duplicate logic for network game setup shell.

---

## Unverified

None in the material claims. All load-bearing facts are verified via live Ghidra decompile.
