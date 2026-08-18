# TARGETDEATH — InfantryClass Death Sequence Deferred Removal (Re-Swarm 2026-05-28)

**Slot**: 3  
**Swarm**: Target death mid-pass active-vector removal timing (2026-05-28T13:20+02:00)  
**Status**: COMPLETE  
**Confidence**: HIGH (all claims verified from live Ghidra decompilation in this session)

---

## Verdict

Infantry death is DEFERRED. A lethal hit does **not** remove the infantry from the active
vector mid-pass. The unit stays alive (IsAlive = TRUE) until its own AI tick's
`DoType_Sequencer` runs through the death animation and calls `FootClass::UnInit`, which
clears IsAlive and enqueues the object on `PendingDeleteList`. The active-vector remover
`FUN_0055BAE0` checks a separate +0x98 pending-delete flag, not IsAlive at +0x90, so no
removal occurs until after the death animation finishes on a future tick.

The Rust `entity.dying = true` pattern in `src/sim/combat/mod.rs` matches this deferred
behavior. No DRIFT on the infantry death path.

---

## Call chain summary

```
TechnoClass::ReceiveDamage  (0x00701900)
  └─ ObjectClass::ReceiveDamage  (0x005F5390)
       ├─ when Health→0:
       │    vtable+0xDC(1)  →  FootClass::Destroy(1)  (0x004D9720)  [broadcasts BREAK, no UnInit]
       │    vtable+0xE0     →  TechnoClass::RecordKill (0x00702D40)
       │    returns 1/2/3   [NOT 4, NOT UnInit]
       └─ caller (TechnoClass) death block on return value ≥ 1 AND Health==0:
            slave cleanup, garrison release, CaptureManager::FreeAll, death scream,
            debris anims, Broadcast_Radio_ToAll(3), StopFiring,
            passenger-kill loop, death weapon (FUN_0070D690)
            NO Queue_Mission, NO Do_Action, NO UnInit
            sets uStack_a4 = 4, returns

-- ReceiveDamage returns; infantry is still alive in the active vector --

InfantryClass::AI  (0x0051BAB0)    [infantry's own next AI tick]
  └─ FootClass::AI  (0x004DA530)
       └─ TechnoClass::AI_Update  (0x006F9E50)
            └─ MissionClass::Mission_Dispatch  (0x005B3060)
                 └─ ... [various mission cases, none of which are MISSION_DIE for infantry]
  └─ IsAlive guard:  if (param_1[0x24] == 0) return;
  └─ InfantryClass::DoType_Sequencer  (0x00520AE0)
       ├─ frame-counter check: if frame_counter < anim_length → wait (break)
       └─ death-sequence cases when frame_counter ≥ anim_length:
            cases 0xB, 0xC, 0xD, 0xE, 0xF, 0x14, 0x15, 0x24:
              (**(code**)(*param_1 + 0xF8))()  →  FootClass::UnInit  (0x004DE5D0)
                └─ ObjectClass::UnInit  (0x005F65F0)
                     ├─ vtable+0xD4 = Limbo
                     ├─ *(param_1 + 0x24) = 0   ← IsAlive cleared HERE (ONLY HERE)
                     └─ append to PendingDeleteList @ 0x00B0F69C
```

---

## Verified facts

### 1. TechnoClass::ReceiveDamage death block — no synchronous removal

Decompiled `0x00701900`. When `ObjectClass::ReceiveDamage` returns with return-value ≥ 1
(damage threshold crossed) **and** `this->Health == 0`, the death block executes:

- Slave cleanup, garrison cleanup, `CaptureManager::FreeAll`
- Death scream sound
- Debris animations
- `vtable+0x280(3)` → `RadioClass::Broadcast_Radio_ToAll(3)` (BREAK signal)
- `vtable+0x3A0()` → StopFiring
- Passenger-kill loop
- `FUN_0070D690(0)` → death weapon trigger
- Sets `uStack_a4 = 4`

**No `Queue_Mission` call. No `Do_Action` call. No `UnInit` call.**
IsAlive at byte +0x90 (`param_1[0x24]`) is NOT cleared by this function.

*Verified: decompile_function 0x00701900*

### 2. ObjectClass::ReceiveDamage — no UnInit, no mission assignment

Decompiled `0x005F5390`. When `Health == 0`:

- Calls `vtable+0xE0` → `TechnoClass::RecordKill` (0x00702D40)
- Calls `vtable+0xE4` → kill notification/stats
- Sets `iVar3 = 4`
- Calls `vtable+0xDC(1)` → `FootClass::Destroy(1)` (0x004D9720)

Returns 1, 2, or 3 based on damage threshold (never 4 — that is set in TechnoClass caller).
**No `UnInit` call. No `Queue_Mission(5)` call.** IsAlive remains TRUE.

*Verified: decompile_function 0x005F5390*

### 3. FootClass::Destroy and ObjectClass::Destroy — no removal

`FootClass::Destroy @ 0x004D9720` (param_2=1 path): calls `vtable+0x274(3)` → 
`RadioClass::Transmit_Radio_ToFirst(3)` (BREAK), then `ObjectClass::Destroy(1)`.

`ObjectClass::Destroy @ 0x005F5280`: deselect logic + `Detach_From_All_Lists`.
**Neither function calls UnInit or removes from the active vector.**

*Verified: decompile_function 0x004D9720, 0x005F5280*

### 4. IsAlive cleared ONLY in ObjectClass::UnInit

`ObjectClass::UnInit @ 0x005F65F0`:

```c
void __fastcall ObjectClass__UnInit(int *param_1) {
    // ... bomb defuse, EMPPassengers, Detach_From_All_Lists ...
    (**(code **)(*param_1 + 0xd4))();          // vtable+0xD4 = Limbo
    *(undefined1 *)(param_1 + 0x24) = 0;       // ← IsAlive CLEARED HERE
    // ... append to PendingDeleteList @ 0x00B0F69C ...
}
```

Field layout (param_1 is `int*`, so `param_1 + 0x24` = byte offset +0x90):
- `+0x90` = IsAlive (cleared here, and **nowhere else** in the ReceiveDamage chain)

After ObjectClass::UnInit the unit is on `PendingDeleteList @ 0x00B0F69C`.

*Verified: decompile_function 0x005F65F0*

### 5. DoType_Sequencer drives death animation and calls UnInit on completion

`InfantryClass::DoType_Sequencer @ 0x00520AE0`:

Frame-counter guard: `param_1[0x3E] < anim_length` → break (waits next tick).

Death cases — when frame counter ≥ animation length:

| DoType value | Meaning | Action |
|---|---|---|
| 0x0B | Die1 | `(*param_1 + 0xF8)()` = FootClass::UnInit |
| 0x0C | Die2 | same |
| 0x0D | Die3 | same |
| 0x0E | Die4 | same |
| 0x0F | Die5 | same |
| 0x14 | Tumble | same |
| 0x15 | FireDance | same |
| 0x24 | (extended death) | same |

Cases 0x1B–0x1E (CRAWL/PRONE transitions) do **not** call UnInit — they are garrison/prone
animation transitions.

*Verified: decompile_function 0x00520AE0*

### 6. vtable+0xF8 = FootClass::UnInit confirmed

InfantryClass vtable base: `0x007EB058`  
vtable+0xF8 is at address `0x007EB150`  
`read_memory(0x007EB150, 4)` → bytes `[0xD0, 0xE5, 0x4D, 0x00]` = `0x004DE5D0` = `FootClass::UnInit`

*Verified: read_memory at 0x007EB150*

### 7. FootClass::UnInit → ObjectClass::UnInit → PendingDeleteList

`FootClass::UnInit @ 0x004DE5D0`: calls `CaptureManagerClass::FreeAll`,
`BuildingClass::DeployUnit_ChronoWarp`, `FUN_006EA870`, then `ObjectClass::UnInit`.
`ObjectClass::UnInit` clears IsAlive (+0x90) and appends to `PendingDeleteList @ 0x00B0F69C`.

*Verified: decompile_function 0x004DE5D0*

### 8. Active-vector remover checks +0x98, not IsAlive at +0x90

`FUN_0055BAE0` (LogicClass active-vector remover):  
Checks `*(char*)(param_2 + 0x98)` — this is a separate pending-delete flag set when the
unit is appended to PendingDeleteList. It is **not** IsAlive at +0x90.

Therefore: IsAlive becoming FALSE (in ObjectClass::UnInit) does not by itself cause the
active-vector sweep to fire during the current attacker pass. Removal only happens after
the next periodic sweep that sees the +0x98 flag.

*Verified: decompile_function 0x0055BAE0*

### 9. InfantryClass::AI — correct call order

`InfantryClass::AI @ 0x0051BAB0`:

1. `FootClass::AI()` (direct call, not vtable)
2. `if (param_1[0x24] == 0) return;`  ← IsAlive guard (byte +0x90)
3. `InfantryClass::DoType_Sequencer()`
4. `FootClass::Locomotion_AI()`

IsActive flag: skipped when DoType ∈ {0xB, 0xC, 0xD, 0xE, 0xF, 0x14, 0x15, 0x22, 0x23, 0x24}.

*Verified: decompile_function 0x0051BAB0*

### 10. Mission 5 for infantry is NOT MISSION_DIE

Initial assumption (from prior session context): Mission 5 = MISSION_DIE.  
**Refuted by three independent callers:**

1. `FUN_00521756` (unlimbo callback, `0x00521756`): human-player infantry → `Queue_Mission(5)` + `Do_Action(0x21=STAND_IDLE)` = "start guarding." Not dying.
2. `FUN_00522E70` (harvest mission): when ore exhausted → `Queue_Mission(5)` = return to guard.
3. `MissionClass::Mission_Dispatch @ 0x005B3060`: both case 5 AND case 6 call `vtable+0x21C` = `FUN_0051F620` (garrison/capture/enter handler), not a death mission handler.

`FUN_0051F620 @ 0x0051F620`: thin wrapper — calls `FUN_00521320()`, if returns -1 → `FootClass::Mission_Guard()`. The main branch of `FUN_00521320` checks DoType 0x1B–0x1E (CRAWL/PRONE states) and handles garrison-enter logic.

Mission 5 for infantry = human-player guard/patrol mission.

*Verified: decompile_function + get_function_callers for FUN_00521756, FUN_00522E70, 0x005B3060, 0x0051F620, 0x00521320*

### 11. vtable+0x4A0 = TechnoClass::IdleAnimDispatch (not recursive AI)

`TechnoClass::AI_Update @ 0x006F9E50` ends with `(*param_1->vtable + 0x4A0)(0)`.
`read_memory(0x007EB4F8, 4)` → `0x0070D990` = `TechnoClass::IdleAnimDispatch`.  
Not a recursive call to `InfantryClass::AI`. Not a death-mission dispatcher.

*Verified: read_memory at 0x007EB4F8*

---

## State transitions — exact field values

| Event | Health | IsAlive (+0x90) | DoType | In active vector |
|---|---|---|---|---|
| Before lethal hit | > 0 | TRUE | normal | YES |
| After `ReceiveDamage` returns | 0 | **TRUE** | unchanged | YES — NOT removed |
| During death animation (multiple ticks) | 0 | **TRUE** | 0xB–0xF / 0x14 / 0x15 / 0x24 | YES |
| DoType_Sequencer: last animation frame reached | 0 | **TRUE** | death value | YES |
| FootClass::UnInit → ObjectClass::UnInit | 0 | **→ FALSE** | death value | → PendingDeleteList |
| Next active-vector sweep (FUN_0055BAE0) | 0 | FALSE | death value | REMOVED |

---

## Rust parity assessment

**MATCH — no DRIFT.**

`src/sim/combat/mod.rs` lines ~977–1009 (death handling):

```rust
if has_animation {
    // Infantry/SHP units: mark dying, trigger death animation.
    let inf_death: u8 = killing_warhead
        .as_ref()
        .map(|(wh, _)| wh.inf_death)
        .unwrap_or(1);
    if let Some(entity) = entities.get_mut(dead_id) {
        entity.dying = true;           // ← stays in EntityStore, not yet despawned
        entity.attack_target = None;
        entity.movement_target = None;
        entity.selected = false;
        if let Some(ref mut anim) = entity.animation {
            use crate::sim::animation::death_sequence_for_inf_death;
            anim.switch_to(death_sequence_for_inf_death(inf_death));
        }
    }
    despawned_ids.push(dead_id);       // ← notifies callers for aggro/targeting cleanup
} else {
    // Structures and voxel vehicles: immediate despawn.
    ...
    entities.remove(dead_id);
    ...
}
```

- `entity.dying = true` keeps the entity in `EntityStore` through the animation, matching
  gamemd's "stays alive in active vector" behavior.
- `src/sim/animation.rs` collects `dying_finished` IDs when `anim.finished` is true, matching
  gamemd's DoType_Sequencer completing the animation and calling FootClass::UnInit.
- Immediate `entities.remove` is reserved for structures and vehicles — matching gamemd's
  synchronous vehicle removal (see slot-2 report).

The only note: `despawned_ids.push(dead_id)` on the `has_animation` branch signals the
combat loop about the kill even though the entity is still present. This is for targeting
cleanup only and does not remove the entity from the simulation — matching gamemd intent.

---

## InfantryClass vtable reference (relevant entries)

Base: `0x007EB058`

| Offset | Address in vtable | Points to | Function |
|---|---|---|---|
| +0x5C | 0x007EB0B4 | 0x0051BAB0 | InfantryClass::AI |
| +0xDC | 0x007EB134 | 0x004D9720 | FootClass::Destroy |
| +0xE0 | 0x007EB138 | 0x00702D40 | TechnoClass::RecordKill |
| +0xF8 | 0x007EB150 | 0x004DE5D0 | FootClass::UnInit ← death self-removal |
| +0x1E8 | 0x007EB240 | 0x005B35E0 | MissionClass::Queue_Mission |
| +0x21C | 0x007EB274 | 0x0051F620 | Garrison/capture mission handler |
| +0x280 | 0x007EB2D8 | 0x0065ACE0 | RadioClass::Broadcast_Radio_ToAll |
| +0x4A0 | 0x007EB4F8 | 0x0070D990 | TechnoClass::IdleAnimDispatch |
| +0x558 | 0x007EB5B0 | 0x0051D6F0 | InfantryClass::Do_Action |

All entries verified via `read_memory` at the listed vtable addresses in this session.

---

## Relationship to slot-2 (vehicles)

Slot-2 found that **vehicles are removed SYNCHRONOUSLY** inside `ReceiveDamage` — the
vehicle vtable+0x16C route calls `vtable+0xF8` (UnInit) inline during the damage call.

Infantry is the **opposite**: DoType_Sequencer drives asynchronous self-removal over
potentially many ticks of death animation. The same vtable+0xF8 slot (FootClass::UnInit)
is the removal mechanism in both cases, but the TIMING is entirely different:

| Type | Removal timing | Mechanism |
|---|---|---|
| Vehicle | Synchronous inside ReceiveDamage | vtable+0x16C → vtable+0xF8 inline |
| Infantry | Deferred — after death animation completes | DoType_Sequencer → vtable+0xF8 per AI tick |
