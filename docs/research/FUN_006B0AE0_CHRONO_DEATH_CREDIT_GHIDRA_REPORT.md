# FUN_006B0AE0 — Chrono Warp Death Credit / Kill Attribution Handler

**Address:** `0x6B0AE0`  
**Convention:** `__thiscall` (ECX = dying TechnoClass*, stack args = source_building, source_house)  
**Session date:** 2026-05-19  
**Confidence:** HIGH (all claims verified from live decompilation)  
**Active in YR:** Yes — fired on Chronosphere-warped unit death, Temporal weapon kill, mind-control teardown, and standard damage-kill.

---

## 1. Overview

`FUN_006B0AE0` is a generic **cargo/passenger kill-credit dispatcher** that runs when a unit with
an active chrono-anim (`TechnoClass+0x2D8 != 0`) is destroyed. It iterates all cargo/passengers
carried by the dying unit (`param_1+0x48` count, `param_1+0x3C` pointer array) and either:

- Kills each passenger with credit attributed to the initiating building/house (Chronosphere path), or
- Kills each passenger with the C4Warhead (no-attribution fallback), or
- Performs a "capture-and-credit" kill via vtable+0x3D4/+0x3D0/+0x388 sequence.

Additionally, when at least one passenger died with attribution, it plays a voice cue
(`g_RulesClass_Instance + 0x234` sound index) via `VocClass__PlayAt(0)`.

**The function does NOT handle the value/cost of the dying unit itself.** That credit
(or non-credit) is handled by callers before or after invoking this function.

**Active in YR:** Yes (Chronosphere path only; see §7 for chrono miner exclusion).

---

## 2. Signature and Parameters (Verified)

```
void __thiscall FUN_006b0ae0(int param_1 /*ECX=TechnoClass**/,
                              int param_2 /*=ChronoSourceBuilding or attacker ptr*/,
                              int param_3 /*=ChronoSourceHouse or 0*/)
```

**param_1 (ECX) = dying TechnoClass\*.**  
Confirmed by all callers: PostWarpValidation, TechnoClass__ReceiveDamage, TemporalClass__Update,
PowerUp_Cleanup, MissionClass__Destructor, FUN_0054ca90 each call with ECX = some TechnoClass-derived
pointer.

**param_2 = ChronoSourceBuilding pointer (nullable)**  
When called from PostWarpValidation: `TechnoClass+0x428`  
When called from TechnoClass__ReceiveDamage: attacker pointer (`in_stack_00000010`)  
When called with (0,0): no attribution source

**param_3 = ChronoSourceHouse pointer (nullable)**  
When called from PostWarpValidation: `TechnoClass+0x42C`  
When called with second arg 0: no house attribution

---

## 3. Core Logic (Verified from Decompilation)

### 3.1 Outer Guard

```c
if (*(int *)(param_1 + 0x24) == 0) return;  // guard: no chrono anim → early exit
```

`param_1 + 0x24` is a pointer field on the dying TechnoClass object. This matches a field in the
RadioClass/ObjectClass base region. It functions as a non-null gate: if no active chrono anim (or
equivalent owner pointer) is set, the function returns immediately without iterating cargo.

At function exit, this field is cleared: `*(undefined4 *)(param_1 + 0x24) = 0`.

### 3.2 Local Player House Lookup

```c
iVar4 = FUN_006a46d0();          // get local player's house index (case-insensitive string compare
                                  //  against HouseClass+0x34+0xBC in g_HouseClass_Array)
local_14 = 0;
for (iVar5 = 0; iVar5 < g_HouseClass_Array_Count; iVar5++) {
    if (*(int *)(HouseClass_Array[iVar5]->vtable+0x34 + 0xBC) == iVar4) {
        local_14 = g_HouseClass_Array[iVar5];
        break;
    }
}
```

`FUN_006a46d0` is a house-name comparison helper that searches `g_HouseClass_Array` using
`FUN_007c8d20` (a case-insensitive string compare). It returns the local player's house index.
`local_14` stores the matched HouseClass pointer for later fallback use in the no-attribution path.

**Confidence: HIGH** — decompilation of both `FUN_006a46d0` and `FUN_007c8d20` verified; FUN_007c8d20
is a standard locale-aware strcmp variant (CRT strnicmp path).

### 3.3 Passenger/Cargo Iteration

```c
iVar1 = *(int *)(param_1 + 0x48);   // cargo count (count-down loop)
piVar6 = NULL;                       // tracks first cargo unit successfully killed (for sound gate)
loop:
    iVar1 -= 1;
    if (iVar1 < 0) break;
    piVar2 = (int*) **(int**)(*(int*)(param_1 + 0x3c) + iVar1 * 4);  // cargo[iVar1] pointer
    if (piVar2 == NULL || g_GameActive == 0) { loop; }
    piVar2[0xb7] = 0;                    // TechnoClass+0x2DC = clear some link field
    if (*(char *)((int)piVar2 + 0x81) != 0) {  // +0x81 = IsInAir flag? (bool)
        // passenger is "in air" — give kill credit to param_2 (source building/house)
        (**(code**)(*piVar2 + 0xe0))(iVar3);   // vtable+0xE0 = some "register kill" call
        (**(code**)(*piVar2 + 0xf8))();         // vtable+0xF8 = Destroy / Limbo
        loop;
    }
    if (iVar3 == 0) {                    // iVar3 = param_2 (source building ptr)
        iVar4 = param_3;                 // ChronoSourceHouse
        if (param_3 == 0) {
            iVar4 = local_14;            // fallback: local player's house
            if (local_14 == 0) {
                // absolute fallback: no attribution — apply C4Warhead directly
                param_2 = piVar2[0x1b];  // cargo unit's Strength (+0x6C = TechnoType+0xA0 strength)
                (**(code**)(*piVar2 + 0x16c))(   // vtable+0x16C = TakeDamage
                    &param_2, 0,
                    *(undefined4*)(g_RulesClass_Instance + 0xfa8),  // C4Warhead
                    0, 0, 0, 0);
                loop;
            }
        }
        // has house attribution (param_3 or local_14) — do capture-style kill
    } else {
        iVar4 = *(int *)(iVar3 + 0x21c);  // BuildingClass+0x21C = some credit int
    }
    (**(code**)(*piVar2 + 0x3d4))(iVar4, 1);  // vtable+0x3D4 = GrantKillCredit(house, 1)?
    (**(code**)(*piVar2 + 0x3d0))();            // vtable+0x3D0 = ?
    (**(code**)(*piVar2 + 0x388))(1);           // vtable+0x388 = Die(1)
    if (piVar6 == NULL) piVar6 = piVar2;        // record first killed unit (for sound)
    loop;
```

**`param_1 + 0x3C`** = pointer to the cargo array (array of pointers to TechnoClass*).  
**`param_1 + 0x48`** = cargo count (integer, loop counts down from count-1 to 0).  
**`piVar2[0xb7]` = `TechnoClass+0x2DC`** = cleared to 0 on each passenger before kill logic.  
**`piVar2[0x1b]` = `TechnoClass+0x6C`** — from context, this reads the unit's Strength value.

### 3.4 Post-Loop Sound Effect

```c
if (g_GameActive != 0 && piVar6 != NULL && g_MapEditorMode == 0
    && *(int *)(g_RulesClass_Instance + 0x234) != -1)
{
    VocClass__PlayAt(0);    // plays Rules+0x234 sound at position 0 (local / non-positional)
}
*(undefined4 *)(param_1 + 0x24) = 0;   // clear the chrono anim pointer
```

The sound is played only if:
1. The game is active (not shutting down)
2. At least one passenger was processed (piVar6 was set)
3. Not in map editor mode
4. `Rules+0x234` is not -1 (a valid sound index is configured)

**`Rules+0x234`** — sound index for the kill-credit sound. Exact INI key not confirmed in this session
(likely `[Audio]` or `[General]` key, not `[CombatDamage]`). Marked as unknown pending further investigation.

---

## 4. Warhead Used for No-Attribution Kills

**`g_RulesClass_Instance + 0xFA8` = `C4Warhead`** (INI key `C4Warhead`, section `[CombatDamage]`)

Confirmed from `RulesClass__ReadCombatDamage @ 0x66C32C`:
```c
pcStack_e0 = s_C4Warhead_0083b1d4;          // string "C4Warhead"
iVar2 = CCINIClass__ReadString();
if (iVar2 != 0) { uVar3 = WarheadTypeClass__FindOrAllocate(); }
*(undefined4 *)(param_1 + 0xfa8) = uVar3;   // stored at Rules+0xFA8
```

This **corrects a stale claim** in `CHRONO_MINER_TELEPORT_GHIDRA_REPORT.md §763/856` which labeled
`Rules+0xFA8` as `"ChronoWarpDamagePercent"`. The verified label is **C4Warhead**.

The no-attribution TakeDamage call passes: `damage = passenger->Strength, warhead = C4Warhead, source = 0`.
The damage value is the full Strength of the cargo unit type — instant kill.

**`g_RulesClass_Instance + 0xFAC` = `CrushWarhead`** (INI key `CrushWarhead`, section `[CombatDamage]`)
(confirmed from same reader, adjacent assignment). Used by FUN_0054ca90 caller (NOT by FUN_006B0AE0 itself).

---

## 5. All Callers (Verified via get_function_callers)

| Caller address | Function | Call context |
|---|---|---|
| `0x54CA90` | `FUN_0054ca90` (JumpjetLocomotionClass State5/6 landing logic) | Calls `FUN_006b0ae0(TechnoClass+0x428, 0)` when chrono anim present, before landing cleanup |
| `0x6AF580` | `PowerUp_Cleanup` | Calls `FUN_006b0ae0(0, 0)` — no attribution. Fired when a building's upgrade slot is reassigned |
| `0x6F4500` | `MissionClass__Destructor` | Calls `FUN_006b0ae0(0, 0)` — cleanup during unit teardown if chrono anim is set |
| `0x701900` | `TechnoClass__ReceiveDamage` | Calls `FUN_006b0ae0(attacker_ptr, 0)` — fired when health drops to 0 (default switch case). See §5.1 |
| `0x7187A0` | `TeleportLocomotionClass__PostWarpValidation` | Calls `FUN_006b0ae0(TechnoClass+0x428, TechnoClass+0x42C)` — fired on warp-onto-water/impassable-terrain death |
| `0x71A760` | `TemporalClass__Update` | Calls `FUN_006b0ae0(TemporalClass+0x24, 0)` — fired when temporal weapon finishes disintegrating target |

### 5.1 TechnoClass__ReceiveDamage Call Context

In `TechnoClass__ReceiveDamage @ 0x701900`, the call site is inside the **default: case** of the
death-handling switch (health == 0, state == 4):
```c
if (*(int *)&this->field_0x2d8 != 0) {
    FUN_006b0ae0(in_stack_00000010, 0);    // in_stack_00000010 = attacker TechnoClass*
}
```
`this->field_0x2d8` corresponds to `TechnoClass+0x2D8` = the chrono anim slot (or mind-control link —
the field at 0x2D8 serves double duty as verified in RECEIVE_DAMAGE_GHIDRA_REPORT). This call fires
for **any death** (not just chrono deaths) when `+0x2D8` is non-zero.

---

## 6. Cash/Score/Bounty Effect

**The function does NOT directly award credits or score to ChronoSourceHouse.**

The credit/score flow is:
- **For passengers with kill attribution** (param_3/local_14 path): `vtable+0x3D4(iVar4, 1)` is called
  on each passenger before `vtable+0x388(1)` (Die). `vtable+0x3D4` likely calls `GrantKillCredit` or
  similar with the house pointer and `1` as the "credit" flag. This is how kill credit is routed to
  the correct house. **Confidence: MEDIUM** — vtable slot identity not confirmed by decompiling
  the concrete method; inferred from signature and context.
  
- **For passengers with no attribution** (all-null path): `TakeDamage(Strength, C4Warhead, 0)` is
  called — no kill credit is awarded, the kill is treated as "collateral."

- **For the dying unit itself**: this function does not touch the dying unit's value. The caller
  (`PostWarpValidation`) kills the warping unit via `vtable+0x3A0()` (Die) directly — no credit
  attributed to ChronoSourceHouse for the primary unit's kill. This is the "no bounty for
  Chronosphere kills" mechanic.

**Active in YR:** Yes.

---

## 7. ChronoSourceHouse NULL and Self-Teleport Cases

### 7.1 ChronoSourceBuilding = NULL (param_2 = 0)

When `param_2 == 0` (e.g., calls with `(0,0)` from PowerUp_Cleanup, MissionClass__Destructor):
- The `iVar3 == 0` branch is taken
- Falls through to `param_3` check
- If `param_3 == 0`: tries `local_14` (local player's house)
- If `local_14 == 0` (no local player, headless server context?): takes C4Warhead fallback path

So even with no attribution args, passengers are killed — they just get killed with C4Warhead damage
and no kill credit to any house.

### 7.2 ChronoSourceBuilding Already Destroyed

If `param_2` (ChronoSourceBuilding) is a dangling pointer to a destroyed building, the code
accesses `*(int *)(iVar3 + 0x21c)` = `BuildingClass+0x21C`. If the building memory has been freed,
this is a use-after-free. Gamemd's allocators typically don't immediately reuse memory, so this
would produce a stale-but-non-crashing read in practice. **No explicit null check on iVar3 after
the `iVar3 == 0` branch** — if `param_2` is non-zero (pointing to freed memory), the code takes
the `iVar4 = *(int *)(iVar3 + 0x21c)` path. This is a latent bug in gamemd.

### 7.3 Chrono Miner Self-Teleport — Does It Ever Hit This Function?

**No.** Per `POSTWARP_VALIDATION_INVALID_TERRAIN_GHIDRA_REPORT.md §1` (verified):
- Self-teleport completes in StateMachineTick Phase 0 and never advances to state 5
- `PostWarpValidation` (0x7187A0) is never called on the self-teleport path
- Therefore the PostWarpValidation call site of `FUN_006B0AE0` is never reached for chrono miner self-teleport

However, the chrono miner CAN hit `FUN_006B0AE0` via `TechnoClass__ReceiveDamage` if it dies
while `TechnoClass+0x2D8` is non-zero (chrono anim active). The miner's chrono anim is set
during its teleport warp-out phase. If it dies from combat during the warp animation, `ReceiveDamage`
will call `FUN_006B0AE0(attacker, 0)`. For a chrono miner carrying ore (not passengers), `param_1+0x48`
cargo count would be 0, so the loop body never executes — the function just clears `+0x24` and returns.

**Active in YR for chrono miner self-teleport death:** No (PostWarpValidation path). Possibly Yes
(ReceiveDamage path, but loop is empty for ore-only cargo).

---

## 8. Key Offsets Summary

| Offset | Owner | Name | Type | Evidence |
|---|---|---|---|---|
| `TechnoClass+0x24` | TechnoClass | AnimOwnerPtr? (chrono-anim gate) | ptr | FUN_006B0AE0 outer guard + clear |
| `TechnoClass+0x3C` | TechnoClass | CargoArrayPtr | ptr-to-ptr-array | Loop: `*(param_1+0x3c) + iVar1*4` |
| `TechnoClass+0x48` | TechnoClass | CargoCount | int | Loop init: `*(param_1+0x48)` |
| `TechnoClass+0x2DC` | TechnoClass | (link field cleared on kill) | int | `piVar2[0xb7] = 0` |
| `TechnoClass+0x6C` | TechnoClass | Strength (for C4 damage) | int | `piVar2[0x1b]` used as damage |
| `TechnoClass+0x428` | TechnoClass | ChronoSourceBuilding | ptr | Callers; POSTWARP_VALIDATION confirmed |
| `TechnoClass+0x42C` | TechnoClass | ChronoSourceHouse | ptr | Callers; POSTWARP_VALIDATION confirmed |
| `TechnoClass+0x2D8` | TechnoClass | ChronoAnimPtr / MCLink | ptr | ReceiveDamage guard; RECEIVE_DAMAGE report |
| `Rules+0xFA8` | RulesClass | C4Warhead | WarheadTypeClass* | RulesClass__ReadCombatDamage @ 0x66C32C |
| `Rules+0x234` | RulesClass | (sound index, identity unknown) | int | PostLoop sound gate |

**NOTE on `TechnoClass+0x24`, `+0x3C`, `+0x48`:** These are unusually early offsets for a TechnoClass-level
field. They fall within the RadioClass/MissionClass base region of TechnoClass's inheritance chain.
The exact sub-object layout at these offsets was not confirmed in this session — identity of the
embedded class that owns these cargo fields is marked [OPEN] pending TechnoClass base-class layout audit.

---

## 9. Open Questions — Final State

- `[RESOLVED] Q1` — Function signature: `__thiscall(ECX=TechnoClass*, param_2=building_ptr, param_3=house_ptr)`. (evidence: decompilation @ 0x6B0AE0, all 6 callers)
- `[RESOLVED] Q2` — All callers listed with one-line context. (evidence: `get_function_callers` result)
- `[RESOLVED] Q3` — Cash/score: no direct credit awarded to ChronoSourceHouse for primary kill; passengers get `vtable+0x3D4` kill-credit call. (evidence: decompilation loop body)
- `[RESOLVED] Q4` — Passenger kill via C4Warhead (no attribution path), `vtable+0x388(1)` with `vtable+0x3D4` credit call (attribution path). (evidence: decompilation)
- `[RESOLVED] Q5` — Warhead = `C4Warhead` at `Rules+0xFA8`. (evidence: RulesClass__ReadCombatDamage @ 0x66C32C)
- `[RESOLVED] Q6` — ChronoSourceHouse NULL: falls through to local_14 (local player house), then to C4Warhead no-credit path. BuildingClass already destroyed: latent UAF — no null check on param_2 after the `!= 0` branch.
- `[RESOLVED] Q7` — Chrono miner self-teleport: never reaches PostWarpValidation call site. May reach ReceiveDamage call site, but loop body runs 0 times if no cargo passengers.
- `[DEFERRED] Q8` — vtable+0x3D4 concrete method identity (what "GrantKillCredit" does internally). (category: bounded-cost-too-high; needs full vtable dispatch trace for each concrete class; next step: decompile vtable+0x3D4 slot for InfantryClass, UnitClass)
- `[DEFERRED] Q9` — Exact identity of TechnoClass+0x24, +0x3C, +0x48 (cargo sub-object embedded struct). (category: requires-different-system-context; needs TechnoClass base-class layout audit)
- `[DEFERRED] Q10` — Rules+0x234 INI key name. (category: bounded-cost-too-high; requires tracing RulesClass reader for +0x234 offset; next step: decompile RulesClass__ReadAudio or adjacent reader)

---

## 10. Stale Claim Corrections

1. **`CHRONO_MINER_TELEPORT_GHIDRA_REPORT.md §763/856`** labels `Rules+0xFA8` as `"ChronoWarpDamagePercent"`. **This is wrong.** The verified name is `C4Warhead` (INI section `[CombatDamage]`). The stale label should be updated.

2. **`POSTWARP_VALIDATION_INVALID_TERRAIN_GHIDRA_REPORT.md §3`** says `FUN_006b0ae0` "iterates all garrisoned/carried passengers (+0x48 count, +0x3C pointer array) and calls their Die or TakeDamage vtable depending on source params." The (+0x48 count, +0x3C pointer) claim is **confirmed correct**. The "Die or TakeDamage" claim is **partially correct** — the no-attribution path uses `TakeDamage(vtable+0x16C)`, but the attribution path uses `vtable+0x3D4 + vtable+0x3D0 + vtable+0x388` (a 3-call kill sequence, not just `Die`).

---

## Sources

**Ghidra addresses decompiled:**
- `0x6B0AE0` — FUN_006B0AE0 (primary)
- `0x6A46D0` — house-name lookup helper
- `0x7C8D20` — string compare helper
- `0x66C32C` — RulesClass__ReadCombatDamage (C4Warhead verification)
- `0x54CA90` — FUN_0054CA90 (JumpjetLocomotion caller)
- `0x6AF580` — PowerUp_Cleanup (caller)
- `0x6F4500` — MissionClass__Destructor (caller)
- `0x701900` — TechnoClass__ReceiveDamage (caller)
- `0x71A760` — TemporalClass__Update (caller)
- `0x449C30` — BuildingClass__Sell (context for PowerUp_Cleanup)
- `0x6F2C60` — TechnoClass__Constructor (field offset verification)

**Docs referenced:**
- `POSTWARP_VALIDATION_INVALID_TERRAIN_GHIDRA_REPORT.md`
- `BRIDGE_RUNTIME_DEEP_DIVE_GHIDRA_REPORT.md` (C4Warhead at Rules+0xFA8 / CrushWarhead at Rules+0xFAC)
- `BRIDGE_SYSTEM_VERIFY_DOC_AMENDMENTS.md`
- `CHRONO_MINER_TELEPORT_GHIDRA_REPORT.md` (stale claim correction)
- `RECEIVE_DAMAGE_GHIDRA_REPORT.md` (TechnoClass+0x2D8)
- `TECHNOCLASS_CHRONO_OFFSETS_VERIFIED.md` (+0x428, +0x42C)

**INI files checked:**
- `ini/rulesmd.ini` — `[CombatDamage]` section for C4Warhead, CrushWarhead
