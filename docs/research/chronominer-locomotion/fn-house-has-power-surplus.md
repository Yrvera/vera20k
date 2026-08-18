# HouseClass__HasPowerSurplus — 0x0050e1b0

**Proposed Ghidra label:** `HouseClass__HasPowerSurplus` (existing name authoritative — labeler skip rename, add plate comment only)

## Summary

One-liner predicate: returns `true` if `HouseClass+0x2d8` (the power surplus field) is greater than zero. Called by `TeleportLocomotionClass__PostWarpValidation` as a power-availability gate: the unit may only warp if its owning house has a power surplus. Also called by `UnitClass__PerCellProcess`.

Function size: 15 bytes (0x0050e1b0–0x0050e1bf), consistent with a single field comparison returning bool.

## Active in YR

**Yes.** Called by `TeleportLocomotionClass__PostWarpValidation` at `0x007188a6` and by `UnitClass__PerCellProcess` at `0x00739f38` (confirmed via `get_xrefs_to 0x0050e1b0`). `PostWarpValidation` is the pre-warp gate check that runs every tick when a unit is armed for warp.

## Decompilation

Source: `decompile_function 0x0050e1b0`

```c
bool __thiscall HouseClass__HasPowerSurplus(HouseClass *this)
{
  return 0 < *(int *)&this->field_0x2d8;
}
```

`this` = HouseClass*. `field_0x2d8` is an `int` (signed) holding the current power surplus (output power minus drain). Returns `true` when surplus > 0.

## Callers

| Address | Function | Role of gate |
|---|---|---|
| `0x007188a6` | `TeleportLocomotionClass__PostWarpValidation` | Power gate: warp only if house has power surplus |
| `0x00739f38` | `UnitClass__PerCellProcess` | General per-cell power check for unit behavior |

## Struct Field Accesses

### HouseClass fields

| Byte Offset | Access | Purpose |
|---|---|---|
| +0x2d8 | `*(int *)&this->field_0x2d8` | Power surplus (output − drain); positive = surplus exists |

The field is accessed as `int` (signed comparison against 0), so negative values (power deficit) return false.

## Globals / Enums / INI Keys Referenced

None — function accesses only the HouseClass field.

## Out-of-Scope Refs

| Symbol | Address | Reason |
|---|---|---|
| `UnitClass__PerCellProcess` | `0x00739f38` | General per-cell logic; not teleport-specific |

## Unverified (YELLOW)

- **HouseClass+0x2d8 field name**: Ghidra shows `field_0x2d8` (unnamed). The field is accessed as a signed int compared against 0. Interpreted as "power surplus" based on the function name and caller context (PostWarpValidation power gate). Independent verification of this field's write sites (power calculation function) was not done in this session.
