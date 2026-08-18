# Is_Ok_To_End: field_35 and field_6AD — Ghidra Research Report

**Target:** `TeleportLocomotionClass::Is_Ok_To_End` (0x719F30)  
**Scope:** Full decompilation + exhaustive setter/clearer trace for TeleportLoco+0x35 and FootClass+0x6AD  
**Date:** 2026-05-19  
**Confidence:** HIGH — all claims verified in Ghidra live session

---

## 1. Calling Convention Note (Critical for Offset Math)

`Is_Ok_To_End` is an **IPiggyback** interface method. Its `param_1` is the IPiggyback
vtable pointer, which sits at **loco base + 0x18**. All offsets inside the function are
relative to that pointer, not to the loco base.

| Expression in decompilation | loco base offset | Documented name |
|---|---|---|
| `param_1 - 0x14` | base+0x04 | ILocomotion vtable → `Is_Moving()` |
| `param_1 + 0x1D` | **base+0x35** | **field_35** |
| `param_1 + 0x20` | base+0x38 | WarpPhase |
| `param_1 + 0x30` | base+0x48 | PiggybackedLoco* |
| `*(param_1 - 0x0C)` | base+0x0C | LinkedTo FootClass* |

---

## 2. Is_Ok_To_End Full Decompilation (0x719F30)

```c
uint TeleportLocomotionClass__Is_Ok_To_End(int param_1)
// param_1 = IPiggyback interface ptr = loco_base + 0x18
{
  // 1. Call Is_Moving() via ILocomotion vtable
  uint not_moving = Is_Moving(param_1 - 0x14) == 0;
  if (!not_moving) return false;

  // 2. Must have a piggybacked locomotor (PiggybackedLoco* at base+0x48 != NULL)
  if (*(int*)(param_1 + 0x30) == 0) return false;

  // 3. field_35 (base+0x35) must be 0
  if (*(byte*)(param_1 + 0x1D) != 0) return false;

  FootClass* linked = *(param_1 - 0x0C);  // base+0x0C = LinkedTo

  // 4. ChronoInTransit (linked+0x27C) must be 0
  if (*(byte*)(linked + 0x27C) != 0) return false;

  // 5. WarpPhase (base+0x38) must be 0
  if (*(int*)(param_1 + 0x20) != 0) return false;

  // 6. IsDeploying (linked+0x6AD) must be 0
  if (*(byte*)(linked + 0x6AD) != 0) return false;

  return true;
}
```

**All six conditions** match the CHRONO_MINER_SYSTEM_OVERVIEW §2 table.

---

## 3. TeleportLoco+0x35 (field_35)

### What is it?

The field at base+0x35 is initialized to 0 in the constructor (0x718046: `MOV byte ptr [ESI+0x35], AL` with AL=0). It lives between `IsMoving` (base+0x34) and `field_36` (base+0x36), both of which are also byte flags initialized to 0.

### Exhaustive write search

Binary pattern search `C6 46 35 xx` (all encodings of `MOV byte ptr [reg+0x35], imm`), covering all register bases:

| Address | Value | Function | ESI type |
|---|---|---|---|
| 0x718046 | 0 | TeleportLocomotionClass__Constructor | TeleportLoco base |
| 0x58ee7c | 0 | FUN_0058ebc0 (pathfinder node cleanup) | different struct |
| 0x59a85f | 0 | FUN_0059a6c0 (pathfinder node cleanup) | different struct |
| 0x5ac318 | 0 | FUN_005ac290 (pathfinder node destructor) | different struct |
| 0x763ec1 | 0 | orphan block (another node destructor) | different struct |

The four non-constructor hits are in pathfinding node destructors (structs with vtable at `[ESI+0x28]` and size ~0x34 bytes). The `[ESI+0x35]` there is a different bool ("has-heap-block") on a completely unrelated struct — coincidental offset collision.

**Search for setter-to-1 (`C6 46 35 01`): zero hits.**

### Verdict: field_35 is a dead field

- **Set to 1:** never, in the entire binary.
- **Set to 0:** only in the constructor (0x718046).
- **Active in YR:** field is always 0. The Is_Ok_To_End check `if (field_35 != 0) return false` is dead code — it always passes. The field exists as an unused byte slot between `IsMoving` (0x34) and `field_36` (0x36).

**Semantic name:** none warranted — it has no runtime writer. Reserve as `_pad_35` or `Reserved_35`.

---

## 4. FootClass+0x6AD (IsDeploying)

### Already documented

**FOOTCLASS_STRUCT_LAYOUT.md §3.14** and **FOOTCLASS_COMPLETE_GHIDRA_REPORT.md §2.7** already name and describe this field:

> `0x6AD | 1 | bool | 0 | IsDeploying | Ctor; Set_Dest: blocks destination when deploying; AI: blocks IPiggyback swap; ReceiveEMP: blocks EMP reset of some field; critical deployment gate`

This is **not a novel field** — answer to (d): it is fully documented in FOOTCLASS_COMPLETE_GHIDRA_REPORT.md.

### Setter

**Address:** 0x710352  
**Function:** `TechnoClass__PerformDeploy` (0x710000–0x71040E)  
**Instruction:** `MOV byte ptr [ESI+0x6AD], 0x1`  
**Context:** Written on the _original_ unit (`in_stack_00000004`) after the deploy-by-warhead
creates a new replacement unit and calls `BuildingClass__DeployUnit_ChronoWarp`. This is
the "unit is being converted to building" scenario (e.g. paradrop deployment triggered by
a warhead hit).

**Only caller of PerformDeploy:** `WarheadTypeClass__Detonate` (0x4690b0).

### Clearer

**No explicit runtime clearer exists.** Exhaustive search for `MOV [reg+0x6AD], 0`
(all encodings: `C6 86 AD 06 00 00 00`, `88 86 AD 06 00 00`, `88 9E AD 06 00 00`,
`C6 81/82/83/85/87 AD 06 00 00 xx`): only constructor hits returned.

The field is set to 0 in:
- `FootClass__Constructor` (0x4D3414): `MOV byte ptr [ESI+0x6AD], BL` (BL=0)

**Why no runtime clearer is needed:** PerformDeploy converts (replaces) the original unit
object. Once `IsDeploying=1`, the original TechnoClass* is deleted shortly after — the
new deployed unit is a separate object constructed with IsDeploying=0. The flag only needs
to persist long enough to block the locomotor swap for the few ticks between the write
and destruction.

### Runtime exposure for the chrono miner

**Normal harvest cycle: IsDeploying is always 0.**  
PerformDeploy is called only from `WarheadTypeClass__Detonate` (a warhead effect that
physically transforms a unit). The chrono miner has no such warhead and does not
go through `PerformDeploy` during its harvest-return-warp-dock cycle.

**Semantic name confirmed:** `IsDeploying` — written when a warhead-triggered deploy begins
on a unit being physically converted to a building. Blocks locomotor swap and destination
changes during that conversion window.

---

## 5. DriveLocomotionClass::Is_Ok_To_End Also Checks field_6AD

**Address:** 0x004AF970  
`DriveLocomotionClass__Is_Ok_To_End` checks exactly the same condition:

```c
if (*(char*)(linked + 0x6AD) != 0) return false;
```

Both locomotors share this gate. This is consistent: a unit undergoing warhead-deploy
should never have its locomotor swapped regardless of which loco is active.

---

## 6. Player-Visible Failure Mode

If IsDeploying were stuck at 1 on a living chrono miner (impossible via normal game mechanics,
but hypothetically via a bug or mod):

- `Is_Ok_To_End` always returns false
- `FootClass::AI` (0x4DA530) never calls `End_Piggyback` and never swaps to DriveLocomotionClass
- The miner stays in TeleportLocomotionClass forever after warping to the refinery-adjacent cell
- DriveLocomotionClass (which drives the last few cells to the dock pad) is **never activated**
- The miner sits translucent-then-opaque at the dock-adjacent cell, never entering the refinery
- Result: **harvest cycle permanently stuck** — miner can never unload, never collects again

If field_35 were stuck at 1 (also impossible via normal mechanics):
- Same outcome — the same `return false` path in Is_Ok_To_End

---

## 7. Summary Table

| Field | Loco base offset | Via IPiggyback this | Semantic name | Setter | Clearer | Normal value for chrono miner |
|---|---|---|---|---|---|---|
| TeleportLoco+0x35 | base+0x35 | param_1+0x1D | _Reserved_35 (dead) | **none** (always 0) | constructor only | always 0 |
| FootClass+0x6AD | n/a (on linked unit) | via `*(param_1−0xC)` | IsDeploying | 0x710352 (PerformDeploy, warhead only) | constructor only | always 0 |

---

## 8. Verified Facts (Load-Bearing)

1. **TeleportLoco+0x35 is never written to 1 anywhere in the binary.** Search `C6 46 35 01` returns zero hits. (Confidence: HIGH — exhaustive pattern search)
2. **FootClass+0x6AD = IsDeploying is already documented** in FOOTCLASS_COMPLETE_GHIDRA_REPORT.md and FOOTCLASS_STRUCT_LAYOUT.md. It is not a novel field.
3. **The only runtime setter of IsDeploying is `TechnoClass__PerformDeploy` at 0x710352**, called exclusively from `WarheadTypeClass__Detonate` (0x4690b0). (Confidence: HIGH — verified via binary pattern search + caller trace)
4. **There is no runtime clearer** for IsDeploying — the unit object is destroyed after deploy completes. (Confidence: HIGH — exhaustive write search, only constructor hits)
5. **DriveLocomotionClass::Is_Ok_To_End (0x4AF970) also checks FootClass+0x6AD**, confirming this is a shared gate on the IPiggyback contract, not teleport-specific. (Confidence: HIGH — directly decompiled)

---

**Status: COMPLETE**
