# FUN_004E5310 — Start-Position Combo Population Loop (All 8 Rows)

## Summary

Iterates all 8 player-slot rows (0–7) and populates each row's start-position
combo by choosing one of two paths — identical dispatch pattern to `FUN_004E4820`
(task #31, color combo) and `FUN_004E49A0` (task #33). For each slot, if game
mode is 3 or 4 (spectator/observer) AND the slot is the local player or closed,
calls `FUN_004E5260` (start-position sentinel loader). Otherwise calls
`FUN_004E50C0` (normal start-position population). Called from dialog init and
WM_COMMAND dispatcher.

## Address

`0x004E5310` (verified via `decompile_function 0x004E5310`)

## Active in YR

**Yes.** In-scope callers include `FUN_006ACEE0` (0x006ACEE0, WM_COMMAND dispatcher,
YR-active anchor) and `FUN_006AE6E0` (0x006AE6E0, dialog init, YR-active anchor).
(Callers confirmed via `get_function_callers 0x004E5310`)

## Signature / Parameters

```c
void FUN_004e5310(void)
// no parameters — reads HWND and session state from globals
```

(verified via `decompile_function 0x004E5310`)

## Behavioral Analysis

### Main loop (identical structure to FUN_004E4820, task #31)

```c
iVar1 = 0;
do {
    if (((g_GameMode == 3) || (g_GameMode == 4)) &&
        ((&DAT_00a8da90)[iVar1] == DAT_00ac11b4 ||
         *(int *)((&DAT_00a8da90)[iVar1] + 0x6b) == -1)) {
        FUN_004e5260();    // sentinel: start-pos sentinel/locked state
    } else {
        FUN_004e50c0();    // normal: populate full start-position list
    }
    iVar1++;
} while (iVar1 < 8);
```

(verified via `decompile_function 0x004E5310`)

### Branch conditions

Identical to `FUN_004E4820` (task #31):

1. `g_GameMode == 3 || g_GameMode == 4`: spectator or observer game mode
2. `(&DAT_00A8DA90)[slot] == DAT_00AC11B4`: this slot is the local spectating player
3. `*(int*)(entry + 0x6B) == -1`: this slot is closed/inactive

The sentinel path fires when (1) AND (2 OR 3). All other rows get normal population.

### Callees

- `FUN_004E5260` — start-position sentinel loader (out of scope for task #39;
  analogous to `FUN_004E4770`, task #30)
- `FUN_004E50C0` — normal start-position population (out of scope; analogous to
  `FUN_004E45A0`, task #29)

## Globals Referenced

| Global | Address | Access | Role |
|---|---|---|---|
| `g_GameMode` | symbolic | READ | Game mode; 3=spectator, 4=observer |
| `DAT_00A8DA90` | `0x00A8DA90` | READ | Per-slot session entry pointer array |
| `DAT_00AC11B4` | `0x00AC11B4` | READ | Local player session entry pointer |

(confirmed via `decompile_function 0x004E5310`)

## Callers

In-scope YR-active callers:
- `FUN_006ACEE0` @ `0x006ACEE0` — WM_COMMAND dispatcher (anchor task #2)
- `FUN_006AE6E0` @ `0x006AE6E0` — dialog init (anchor task #1)

Out-of-scope caller: `CDFileClass__Constructor @ 005E3D10`.

(confirmed via `get_function_callers 0x004E5310`)

## Callees

- `FUN_004E5260` (0x004E5260) — start-pos sentinel loader (out of scope)
- `FUN_004E50C0` (0x004E50C0) — normal start-pos population (out of scope)

(confirmed via `get_function_callees 0x004E5310`)

## Relationship to Color System

This function is the start-position analogue of `FUN_004E4820` (task #31). The
three-way parallel (country/color/start-position) pattern in the dialog init
and WM_COMMAND handler shows each row has: country combo, color combo, and
start-position combo — each with the same spectator/observer guard loop.

## Out-of-scope refs

- `FUN_004E5260` — start-pos sentinel; analogue of `FUN_004E4770` (task #30)
- `FUN_004E50C0` — normal start-pos population; analogue of `FUN_004E45A0` (task #29)
- `g_GameMode`, `DAT_00A8DA90`, `DAT_00AC11B4` — session globals

## Unverified (YELLOW)

- `g_GameMode == 3` = spectator, `== 4` = observer: same inference as task #31.
- `DAT_00A8DA90` as "per-slot session entry pointer array": same inference as #31.
- `+0x6B` on session entry = slot status: same inference as #31.
