# FUN_004E48E0 — Color Combo Selection Restore (Session-Saved Color)

## Summary

`FUN_004E48E0` is a thin wrapper that determines which color value to restore
into a color combo and delegates the actual selection to `FUN_004E49A0`. It
reads the session-saved color index from `DAT_00A8B3B4`, then applies two
override rules: if `DAT_00A8B3B8 == -2` (row was closed when session was
saved), the color is overridden to `-2` (sentinel); if the game mode is
spectator/observer (3 or 4) AND both the slot is the local player AND the
slot is closed (`entry+0x6B == -1`), the color is overridden to `8` (spectator
color index). The resulting value is passed as the target color index to
`FUN_004E49A0`. Called from dialog init (`FUN_006AE6E0`) and an out-of-scope
caller.

## Address

`0x004E48E0` (verified via `decompile_function 0x004E48E0`)

## Active in YR

**Yes.** Called from `FUN_006AE6E0` (0x006AE6E0), the dialog init handler for
the YR offline Skirmish lobby dialog 0x102.

(confirmed via `get_function_callers 0x004E48E0`)

## Signature / Parameters

```c
void __fastcall FUN_004e48e0(undefined4 param_1, int param_2)
// param_1 = dialog 0x102 HWND (forwarded to FUN_004e49a0 as first arg)
// param_2 = slot index or color combo control ID (forwarded to FUN_004e49a0)
```

(verified via `decompile_function 0x004E48E0`)

## Behavioral Analysis

### Logic

```c
uVar1 = DAT_00a8b3b4;           // start: saved color index from session state
if (DAT_00a8b3b8 == -2) {
    uVar1 = 0xfffffffe;          // override: row was closed → sentinel -2
}
if (((g_GameMode == 3) || (g_GameMode == 4)) &&
    ((&DAT_00a8da90)[param_2] == DAT_00ac11b4) &&
    (*(int*)((&DAT_00a8da90)[param_2] + 0x6b) == -1))
{
    uVar1 = 8;                   // override: spectator mode + local player + closed slot → color 8
}
FUN_004e49a0(uVar1);
```

(verified via `decompile_function 0x004E48E0`)

### Override priority

The two overrides are applied sequentially — if both conditions fire, `8`
(spectator override) wins over `-2` (closed override) because it comes second.
In practice the closed-row sentinel and the spectator+local+closed conditions
describe overlapping scenarios:
- `DAT_00A8B3B8 == -2`: row was closed when session was last saved (persisted state)
- Spectator+local+closed: same slot but checked live against the current game mode

### `DAT_00A8B3B4` — session-saved color

`DAT_00A8B3B4` holds the color index that was saved from the prior session.
It is read and written as part of the `DAT_00A8B3F0`-area slot-persistence
array used by `FUN_006AE6E0` (init) to restore prior-session state.

### `DAT_00A8B3B8` — closed-row flag

`DAT_00A8B3B8` adjacent to `DAT_00A8B3B4` (4 bytes later in the slot
persistence region). Value `-2` (0xFFFFFFFE) indicates the slot was closed
when last saved. This matches the `-2` item-data sentinel written by
`FUN_004E4770`.

### Color index `8` in spectator mode

The literal value `8` is passed as a color index override for a spectator/
observer slot. The meaning of index `8` in the color table is not decoded
in this task — it likely maps to a neutral "spectator" color (grey or none)
in the owner-draw color swatch table at `DAT_008B4040`.

### FUN_004E49A0 (task #33)

`FUN_004E49A0` takes the computed color value, finds the matching item in
the color combo by iterating `CB_GETITEMDATA` (0x150) until a match is
found, then calls `CB_SETCURSEL` (0x14E) to select it. It also updates the
color assignment table at `DAT_008B4040` and refreshes all 8 color combos.

## Globals referenced

| Global | Address | Access | Role |
|--------|---------|--------|------|
| `DAT_00A8B3B4` | 0x00A8B3B4 | READ | Session-saved color index |
| `DAT_00A8B3B8` | 0x00A8B3B8 | READ | Closed-row color sentinel flag (-2 = closed) |
| `DAT_00A8DA90` | 0x00A8DA90 | READ | Array of per-slot session entry pointers |
| `DAT_00AC11B4` | 0x00AC11B4 | READ | Local player session entry pointer |
| `g_GameMode` | (symbolic) | READ | Current game mode; 3=spectator, 4=observer |

## Struct field accesses

| Pointer | Offset | Unit | Usage |
|---------|--------|------|-------|
| `(&DAT_00A8DA90)[param_2]` (session entry ptr) | `+0x6B` | int | Slot status; -1 = closed |

## Callers

| Address | Name | Role |
|---------|------|------|
| 0x005E3D10 | CDFileClass__Constructor (mislabeled) | out-of-scope |
| 0x006AE6E0 | FUN_006ae6e0 | Dialog init (task #1) |

(confirmed via `get_function_callers 0x004E48E0`)

## Callees

| Address | Name | Role |
|---------|------|------|
| 0x004E49A0 | FUN_004e49a0 | Color combo selection set + table update (task #33) |

(confirmed via `get_function_callees 0x004E48E0`)

## Out-of-scope refs

- `CDFileClass__Constructor` at 0x005E3D10 is a Ghidra mislabel (noted in manifest);
  actual role out of cell-UI scope.

## TS-filter

All in-scope callers are reachable from the YR offline Skirmish dialog path.
No TS-only gate. **TS-legacy score: 0.0.**

## Unverified (YELLOW)

- `DAT_00A8B3B4` as "session-saved color index" — inferred from context: it is
  read as the base value and overridden only in closed/spectator cases, consistent
  with a persisted prior-session color selection; not independently verified via
  write-site cross-reference.
- `DAT_00A8B3B8 == -2` as "row was closed when saved" — inferred from the
  value `-2` matching the color sentinel and the adjacent address to `DAT_00A8B3B4`
  in the slot-persistence block; not verified via write-site search.
- Color index `8` in spectator mode — value `8` is hardcoded; its meaning (which
  color entry it resolves to in the owner-draw swatch table) is not verified against
  `DAT_008B4040` or the color table decode (task #66).
- `g_GameMode` values 3 and 4 for spectator/observer — inferred from usage pattern;
  not verified against a game mode enum decode.
