# FUN_004E4820 — Color Combo Population Loop (All 8 Rows, Init Path)

## Summary

`FUN_004E4820` iterates all 8 player-slot rows (0–7) and populates each row's
color combo by choosing one of two paths. For each slot it checks whether
game mode is 3 or 4 (spectator/observer) AND whether the session player entry
is the local player (`DAT_00a8da90[slot] == DAT_00ac11b4`) or the slot is
closed (`*(int*)(entry + 0x6B) == -1`). If either condition is true it calls
`FUN_004E4770` (sentinel loader — loads "Random Color" with item-data -2).
Otherwise it calls `FUN_004E45A0` (normal color population path). Sole caller:
`FUN_006AE6E0` (dialog init handler).

## Address

`0x004E4820` (verified via `decompile_function 0x004E4820`)

## Active in YR

**Yes.** Sole caller is `FUN_006AE6E0` (0x006AE6E0), the dialog init handler
for the YR offline Skirmish lobby dialog 0x102. No TS-only gate.

(confirmed via `get_function_callers 0x004E4820`)

## Signature / Parameters

```c
void FUN_004e4820(void)
```

No parameters — reads dialog HWND and session state from globals.

(verified via `decompile_function 0x004E4820`)

## Behavioral Analysis

### Main loop

```c
iVar1 = 0;
do {
    if (((g_GameMode == 3) || (g_GameMode == 4)) &&
        ((&DAT_00a8da90)[iVar1] == DAT_00ac11b4 ||
         *(int *)((&DAT_00a8da90)[iVar1] + 0x6b) == -1))
    {
        FUN_004e4770();    // sentinel: "Random Color", item-data -2
    }
    else {
        FUN_004e45a0();    // normal: populate full color list
    }
    iVar1++;
} while (iVar1 < 8);
```

(verified via `decompile_function 0x004E4820`)

### Spectator/observer branch condition

The check has two parts joined by AND:
1. `g_GameMode == 3 || g_GameMode == 4` — game is in spectator or observer mode
2. `(&DAT_00a8da90)[iVar1] == DAT_00ac11b4` — this slot's session entry pointer
   matches the local-player session entry pointer (i.e., the slot belongs to the
   local player who is spectating), OR
   `*(int*)(entry + 0x6B) == -1` — the slot's status field at offset `+0x6B`
   equals `-1`, meaning the slot is closed/inactive.

Only when BOTH the game mode is spectator AND this specific slot is the local player
or closed does the sentinel path fire. All other slots in spectator mode still
get the normal color population via `FUN_004E45A0`.

### Normal branch

`FUN_004E45A0` (task #29) — populates the full color list for a row from the
color table, with item-data set to color indices. Not decoded here.

### Sentinel branch

`FUN_004E4770` (task #30, decoded) — resets the combo to a single entry with
string 0x237 and item-data `-2`. This is the observable state for a closed or
spectator-observing slot: the color combo shows one locked entry and cannot
be changed.

## Globals referenced

| Global | Address | Access | Role |
|--------|---------|--------|------|
| `g_GameMode` | unknown (referenced symbolically) | READ | Current game mode; 3=spectator, 4=observer |
| `DAT_00A8DA90` | 0x00A8DA90 | READ | Array of per-slot session entry pointers |
| `DAT_00AC11B4` | 0x00AC11B4 | READ | Local player's session entry pointer |

Note: `g_GameMode` symbol name is as resolved in decompile; exact address not
confirmed in this session.

## Struct field accesses

| Pointer | Offset | Unit | Usage |
|---------|--------|------|-------|
| `(&DAT_00A8DA90)[slot]` (session entry ptr) | `+0x6B` | int | Slot status; -1 = closed |

## Callers

| Address | Name | Role |
|---------|------|------|
| 0x006AE6E0 | FUN_006ae6e0 | Dialog init (task #1) |

(confirmed via `get_function_callers 0x004E4820`)

## Callees

| Address | Name | Role |
|---------|------|------|
| 0x004E4770 | FUN_004e4770 | Color sentinel loader (task #30, decoded) |
| 0x004E45A0 | FUN_004e45a0 | Normal color combo population (task #29) |

(confirmed via `get_function_callees 0x004E4820`)

## Out-of-scope refs

- `FUN_004E45A0` — full color population; out of current task scope (task #29 in progress by another decoder)

## TS-filter

Sole caller is the YR dialog init. No TS-only gate. **TS-legacy score: 0.0.**

## Unverified (YELLOW)

- `g_GameMode == 3` = spectator, `== 4` = observer — inferred from usage pattern
  alongside `DAT_00AC11B4` (local player pointer); specific mode-code enum values
  not verified against a global enum decode in this session.
- `DAT_00A8DA90` as "array of per-slot session entry pointers" — inferred from
  loop index access and comparison with `DAT_00AC11B4` (local player entry);
  struct definition not independently decoded.
- Offset `+0x6B` on session entry = slot status field with -1 meaning "closed" —
  inferred from context with type-code -1 (Closed) in init flow; not independently
  verified via struct layout decode.
