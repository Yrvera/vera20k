# struct-BuildingClass-IC-fields

## Scope

Building-specific IC state fields referenced in `BuildingClass__IronCurtain` (0x00457c90):
`+0x6df` (gate byte), `+0x528`, `+0x52c`, `+0x530`, `+0x540`.

## Source functions

All offsets derived from:
- `decompile_function 0x00457c90` (BuildingClass__IronCurtain — primary reference)
- `decompile_function 0x0043b740` (BuildingClass__Constructor — initialization)

## Field table

| Offset | Type | Size | Init | IC behavior | Purpose (confidence) |
|---|---|---|---|---|---|
| `+0x528` | i32 | 4 | `g_CurrentFrameCounter` | Set to current frame | Frame timestamp — VERIFIED |
| `+0x52c` | i32 | 4 | unknown | Set to `local_8` (stack) | Unknown — YELLOW |
| `+0x530` | i32 | 4 | 0 | Cleared to 0 | Unknown cleared value — YELLOW |
| `+0x540` | i32 | 4 | 0 | Cleared to 0 | Unknown cleared value — YELLOW |
| `+0x6df` | u8 | 1 | 0 | Cleared to 0 (gate check) | Gate flag — VERIFIED cleared |

## Detailed field analysis

### `BuildingClass + 0x528` (frame timestamp on IC apply)

**Constructor** (`decompile_function 0x0043b740`): `param_1[0x14a] = g_CurrentFrameCounter`
- `param_1` is `int*`, so `param_1[0x14a]` = byte offset `0x14a × 4 = 0x528`. ✓
- Initialized to `g_CurrentFrameCounter` at construction time.

**IC apply** (`decompile_function 0x00457c90`):
```c
uVar1 = g_CurrentFrameCounter;
*(undefined4 *)((int)this + 0x528) = uVar1;
```
Written to current frame counter when the IC gate fires.

**Purpose (VERIFIED)**: A frame timestamp recording when the IC reset block ran. Parallel to `TechnoClass + 0x18c` (the TechnoClass IC apply frame), but building-specific. Likely used to compute building IC duration or track the onset of the building's IC-specific state.

### `BuildingClass + 0x52c` (write from local_8 — likely vestigial)

**Constructor**: Not explicitly set in `BuildingClass__Constructor`. May be set by parent `TechnoClass__Constructor`.

**IC apply**:
```c
*(undefined4 *)((int)this + 0x52c) = local_8;
```
`local_8` is Ghidra's name for an uninitialized stack slot. In the decompilation, `local_8` is never assigned before this write — it holds whatever was on the stack frame at `[EBP - 8]` or similar. This write is almost certainly **vestigial or erroneous** — it writes stack garbage, not a meaningful value.

**Purpose (YELLOW — Unverified)**: Unknown. The write of `local_8` suggests this field is either:
(a) a dead write to a field not meaningful for the IC path, or
(b) intended to write a frame counter duration that was left uninitialized in this code path.

**Action**: Read downstream consumers of `+0x52c` to determine if it matters.

### `BuildingClass + 0x530` (cleared on IC apply)

**Constructor** (`decompile_function 0x0043b740`): `param_1[0x14c] = 0`
- `param_1[0x14c]` = byte offset `0x14c × 4 = 0x530`. Initialized to 0. ✓

**IC apply**: Set to 0 (reset).

**Purpose (YELLOW — Unverified)**: Unknown. The pattern of "set to 0 on IC apply" alongside `+0x528` (set to current frame) and `+0x540` (set to 0) suggests these form a timer trio: start frame, end frame, elapsed counter, or similar. Without reading all consumers, the semantic cannot be determined.

### `BuildingClass + 0x540` (cleared on IC apply)

**Constructor** (`decompile_function 0x0043b740`): `param_1[0x150] = 0`
- `param_1[0x150]` = byte offset `0x150 × 4 = 0x540`. Initialized to 0. ✓

**IC apply**: Set to 0 (reset).

**Purpose (YELLOW — Unverified)**: Unknown. May be an accumulator (damage taken since IC started, or production count paused) that is cleared on IC re-apply.

### `BuildingClass + 0x6df` (IC gate byte)

**Constructor** (`decompile_function 0x0043b740`):
```c
*(undefined1 *)((int)param_1 + 0x6df) = 0;
```
Initialized to 0 at construction. ✓

**IC apply** (`decompile_function 0x00457c90`):
```c
if (*(char *)((int)this + 0x6df) != '\0') {
    *(undefined1 *)((int)this + 0x6df) = 0;
    // ... reset block
}
```
Read as gate: when non-zero, triggers the building-specific IC reset block and clears itself.

**What sets it?**: NOT found in BuildingClass__IronCurtain (it is only cleared there). NOT found in the constructor (initialized to 0). The setter must be elsewhere.

**Context**: The byte sits in a cluster of single-byte flags at `+0x6dd`, `+0x6de`, `+0x6df`, `+0x6e1`, `+0x6e2`, `+0x6e3` — all initialized to 0 in the constructor. This is a building-state flag cluster. Likely candidates for the setter:
- A building's production/idle state transition
- An animation phase completion callback
- The IC super-weapon dispatch path (StartFidget mislabeled at 0x004deae4) — should be checked

**Purpose (YELLOW — Unverified setter)**: The gate byte `+0x6df` gates the "IC with building-specific reset" path. When cleared (normal), `BuildingClass__IronCurtain` skips the reset block and delegates directly to `TechnoClass__IronCurtain`. When set, it first resets the building's IC-adjacent state fields before delegating. The purpose of the reset is not fully clear without identifying what set `+0x6df`.

## Adjacent context from constructor

The constructor initializes these in a contiguous block (verified via `decompile_function 0x0043b740`):

```c
*(undefined1 *)((int)param_1 + 0x6dd) = 0;
*(undefined1 *)((int)param_1 + 0x6de) = 0;
*(undefined1 *)((int)param_1 + 0x6df) = 0;  // gate byte
*(undefined1 *)(param_1 + 0x1b8) = 0;       // = +0x6e0
*(undefined1 *)((int)param_1 + 0x6e1) = 0;
...
```

This places `+0x6df` inside a byte-flag cluster, NOT a 4-byte aligned struct field group.

## Out-of-scope references

- Setter of `+0x6df` — needs follow-up; recommend scope-explorer investigate StartFidget (0x004deae4) and building mission state functions for the setter.
- `+0x52c` downstream readers — needed to confirm vestigial/active.
- `g_CurrentFrameCounter` — covered by task #14.

## Summary

Of the five IC-related BuildingClass fields, only `+0x528` (frame timestamp, initialized to current frame, set again on IC apply) has fully confirmed purpose. `+0x530` and `+0x540` are consistently zeroed and likely cleared accumulators. `+0x52c` receives a probable stack-garbage write (vestigial). `+0x6df` is a gate byte that enables the building-specific IC reset path, cleared on IC apply, but its setter is unidentified.

## Active in YR: Yes (the fields are accessed in the active BuildingClass__IronCurtain vtable path)
