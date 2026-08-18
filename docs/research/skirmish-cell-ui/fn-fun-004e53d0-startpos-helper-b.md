# FUN_004E53D0 — Start-Position Slot Sentinel Dispatcher

## Summary

Determines whether a given slot should receive the start-position sentinel (locked
to position 8) or the normal start-position update, then dispatches to
`FUN_004E5480`. Takes a dialog-related `param_1` (likely HWND or control ID) and
a slot index `param_2`. If game mode is 3 or 4 (spectator/observer) AND the slot
is simultaneously the local player AND the slot is closed (status +0x6B == −1),
passes position value `8` to `FUN_004E5480`; otherwise passes `0xFFFFFFFE`
(available sentinel). Sole in-scope callers are `FUN_006AE6E0` (dialog init)
and `CDFileClass__Constructor` (mislabeled; out of scope).

## Address

`0x004E53D0` (verified via `decompile_function 0x004E53D0`)

## Active in YR

**Yes.** In-scope caller `FUN_006AE6E0` (0x006AE6E0, dialog init, YR-active anchor).
(Callers confirmed via `get_function_callers 0x004E53D0`)

## Signature / Parameters

```c
void __fastcall FUN_004e53d0(undefined4 param_1, int param_2)
// param_1 = dialog HWND or combo control ID (exact type undetermined)
// param_2 = slot index (0–7)
```

(verified via `decompile_function 0x004E53D0`)

## Behavioral Analysis

### Full decompile

```c
void __fastcall FUN_004e53d0(undefined4 param_1, int param_2)
{
    undefined4 uVar1;

    uVar1 = 0xfffffffe;   // default: available sentinel
    if (((g_GameMode == 3) || (g_GameMode == 4)) &&
        ((&DAT_00a8da90)[param_2] == DAT_00ac11b4) &&
        (*(int *)((&DAT_00a8da90)[param_2] + 0x6b) == -1)) {
        uVar1 = 8;        // sentinel: start position locked to index 8
    }
    FUN_004e5480(uVar1);
}
```

(verified via `decompile_function 0x004E53D0`)

### Condition difference vs FUN_004E5310

`FUN_004E5310` (task #39) uses OR between the local-player check and the closed-slot
check: `(entry == DAT_00AC11B4) || (entry+0x6B == -1)`.

`FUN_004E53D0` uses AND: `(entry == DAT_00AC11B4) && (entry+0x6B == -1)`.

The sentinel fires here only when the slot is simultaneously the local player AND
closed — a narrower condition. This may be intentional: `FUN_004E53D0` targets a
single slot (per-call) rather than iterating all 8, and the AND gate avoids
setting a hard position value 8 for merely closed (non-local) slots.

### Start position index 8

When the sentinel fires, `uVar1 = 8` is passed to `FUN_004E5480` (task #41). In
the 0-based start-position system (9 entries, indices 0–8), index 8 is the last
position. This appears to lock the spectating local player to the last/overflow
start-position slot rather than a real position.

### FUN_004E5480 (task #41)

`FUN_004E5480` is the start-position sentinel applier — the callee here. It receives
either `0xFFFFFFFE` (available; do normal update) or `8` (locked; set to position 8).

## Globals Referenced

| Global | Address | Access | Role |
|---|---|---|---|
| `g_GameMode` | symbolic | READ | Game mode; 3=spectator, 4=observer |
| `DAT_00A8DA90` | `0x00A8DA90` | READ | Per-slot session entry pointer array |
| `DAT_00AC11B4` | `0x00AC11B4` | READ | Local player session entry pointer |

(confirmed via `decompile_function 0x004E53D0`)

## Callers

In-scope YR-active callers:
- `FUN_006AE6E0` @ `0x006AE6E0` — dialog init (anchor task #1)

Out-of-scope caller: `CDFileClass__Constructor @ 005E3D10` (mislabeled; actual role
is map start-count helper, task #59).

(confirmed via `get_function_callers 0x004E53D0`)

## Callees

- `FUN_004E5480` (0x004E5480) — start-position sentinel applier (task #41)

(confirmed via `get_function_callees 0x004E53D0`)

## Out-of-scope refs

- `g_GameMode`, `DAT_00A8DA90`, `DAT_00AC11B4` — session globals
- `FUN_004E5480` — covered by task #41

## Unverified (YELLOW)

- `param_1` type (`undefined4`): Ghidra could not resolve the type. Likely
  a dialog HWND passed through to `FUN_004E5480`, but not confirmed.
- Start position index `8` as "last/overflow position": inferred from the 9-entry
  0-based table (task #37 shows 9 entries); the meaning of locking to index 8
  specifically is not verified against the start-position data at `0x822BA4`.
- AND vs OR condition difference vs `FUN_004E5310`: noted above; the intentionality
  of the AND is inferred, not confirmed against caller context.
