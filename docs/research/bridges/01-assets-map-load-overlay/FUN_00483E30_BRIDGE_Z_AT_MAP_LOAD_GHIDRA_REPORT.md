# FUN_00483E30 — Bridge Z (cell+0x10E) at Map Load: Verification Report

**Date:** 2026-05-19
**Investigator:** Single-slot follow-up (read-only Ghidra MCP)
**Scope:** Does `FUN_00483E30 @ 0x00483E30` write `cell+0x10E` with `heightLevel+4`,
or with a fixed default, at map load?

---

## ONE-LINE VERDICT

**At map load, `cell+0x10E` is set to the literal constant `1000` — NOT to
`heightLevel + 4`. The `heightLevel + 4` formula exists ONLY in `Cell_ComputeZAdjust
@ 0x00484680`, which runs exclusively during LightningStorm / PsychicDominator ticks.**

---

## Evidence

### Fact 1 — `FUN_00483E30` writes `cell+0x10E` from `param_7` (not a computed formula)

Verified via `decompile_function 0x00483E30`:

```c
// In the "default initialisation" early-return branch (param_2 == 0, no valid tile):
*(undefined2 *)(param_1 + 0x10e) = 1000;

// In the normal path (LAB_00483f7a):
*(undefined2 *)(param_1 + 0x10e) = param_7;
```

`+0x10E` receives whichever value the caller passes as `param_7`.
There is no `heightLevel` read and no `+ 4` arithmetic inside this function.

### Fact 2 — Both map-load callers pass `1000` as `param_7`

`FUN_004AE450 @ 0x004AE450` (the iterator loop called from the background-init
path) — verified via `decompile_function 0x004AE450`:

```c
FUN_00483e30(0, 0x10000, 0, 1000, 1000, 1000);
//                              ^3   ^4    ^5   ^6 = param_7 → cell+0x10E = 1000
```

`MapClass__InitCellAttributes @ 0x00568BB0` — verified via
`decompile_function 0x00568BB0`:

```c
FUN_00483e30(0, 0x10000, 0, 1000, 1000, 1000);
```

Argument layout for the 6-arg call (positions 1-indexed, ignoring implicit `this`):

| Arg position | Value    | Written to       |
|--------------|----------|------------------|
| 1 (param_2)  | `0`      | branch selector  |
| 2 (param_3)  | `0x10000`| `cell+0x104`     |
| 3 (param_4)  | `0`      | `cell+0x108`     |
| 4 (param_5)  | `1000`   | `cell+0x10A`     |
| 5 (param_6)  | `1000`   | `cell+0x10C`     |
| 6 (param_7)  | `1000`   | **`cell+0x10E`** |

### Fact 3 — `Cell_ComputeZAdjust @ 0x00484680` DOES use `heightLevel + 4` for `+0x10E`

Verified via `decompile_function 0x00484680`. The key expressions (condensed):

```c
// sVar5 starts as a per-cell ambient Z baseline (from cell+0x108)
// Then the bridge-track calculation:
sVar5 = SCALE * (heightLevel + 4) - OFFSET;   // "+4" is the bridge elevation bonus
*(short *)(param_1 + 0x10e) += sVar5;          // applied on top of the baseline
// Final scaled + clamped write:
*(short *)(param_1 + 0x10e) = sVar2;           // scale by cell+0x104, clamp 0–2000
```

This function is called from the LightningStorm / PsychicDominator per-tick path
(confirmed in prior swarm `CELL_COMPUTE_ZADJUST_FORMULA_GHIDRA_REPORT.md`). It is
**not** called at map load.

### Fact 4 — The `heightLevel + 4` bonus is absent from `cell+0x10E` at map-load time

At map load every cell receives `cell+0x10E = 1000` (the neutral "no scale" sentinel,
matching the value for `+0x10A`, `+0x10C`, `+0x110`, `+0x112`, `+0x114`).
The `heightLevel + 4` bridge-bonus Z is only applied dynamically when
`Cell_ComputeZAdjust` fires (LightningStorm / PsychicDominator active).

### Fact 5 — Active in YR: **Conditional**

`FUN_00483E30` itself is active in YR (called at map load from both
`MapClass__InitCellAttributes` and `FUN_004AE450`). The `heightLevel + 4` formula
inside `Cell_ComputeZAdjust` is active only when a LightningStorm or PsychicDominator
superweapon is in progress (`FUN_0053A100` / `FUN_0053B400` / `FUN_0053A110` gate the
formula branch — verified via `decompile_function 0x00484680`).

---

## Recommended amendments to the slot-4 report
### (`HIGH_BRIDGE_UNDER_DECK_OCCLUSION_RENDER_GHIDRA_REPORT.md`)

The slot-4 claim:

> "`Cell_ComputeZAdjust @ 0x00484680` pre-computes three Z fields per cell **on map
> load**" using `heightLevel + 4` for `cell+0x10E`.

is incorrect on two points:

1. **Wrong function.** Map-load Z initialization is done by `FUN_00483E30`, not
   `Cell_ComputeZAdjust`.
2. **Wrong value.** `FUN_00483E30` writes `1000` (not `heightLevel + 4`) to
   `cell+0x10E` at map load. The `heightLevel + 4` formula belongs exclusively to
   `Cell_ComputeZAdjust` and runs only during superweapon ticks.

The under-deck occlusion *mechanism* (two-pass Z, bridge overlays having a higher Z
value than ground units) may still be correct — but the "how and when `+0x10E` gets
its elevated value" story must be revised: in a normal skirmish without LightningStorm
or PsychicDominator, `cell+0x10E` stays at `1000` the entire match.

**Implication for the Rust port:** Do NOT apply a `heightLevel + 4` bonus to
`ZAdjust_Bridge` at map load. Initialise all six Z fields (`+0x10A`–`+0x114`) to
`1000`. Only apply the `heightLevel + 4` formula during the LightningStorm /
PsychicDominator dynamic tick path when porting `Cell_ComputeZAdjust`.

---

## Summary table

| Question | Answer |
|---|---|
| Does `FUN_00483E30` write `cell+0x10E` with `heightLevel+4` at map load? | **No** |
| What value does it write? | **`1000`** (literal constant, both call sites) |
| Where does `heightLevel+4` appear? | `Cell_ComputeZAdjust @ 0x00484680` only |
| When does `Cell_ComputeZAdjust` run? | LightningStorm / PsychicDominator ticks only |
| In a normal YR match without superweapons, what is `cell+0x10E`? | `1000` throughout |

---

## Status: COMPLETE
