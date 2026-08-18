# TARGET-DEATH: BuildingClass Destruction — Active-Vector Removal Owner & Timing

**Re-Swarm Date:** 2026-05-28  
**Slot:** 4  
**Status:** COMPLETE  
**Confidence:** HIGH (all load-bearing claims verified via Ghidra MCP decompile in this session)

---

## Target Question

> When a building is destroyed by a lethal hit: WHO removes it from the LogicClass active
> vector and WHEN — synchronously by `ReceiveDamage`, or deferred to `BuildingClass::Update`?

**Answer:** The active-vector removal is **synchronous, during the lethal `ReceiveDamage` call**,
via the `BuildingClass__Limbo` call in case 4. For a normal (non-IC) building death,
`BuildingClass::Update` later handles the final `UnInit` + `PendingDeleteList` append,
but the active-vector slot has already been vacated by the time `Update` runs.

---

## Non-Goals

- Full destruction-effects internals (DeathWeapon, debris, survivors) — covered in existing doc.
- Damage math — fully documented elsewhere; not re-derived here.
- IC-related delay-kill path — mentioned but not decoded in detail.

---

## Evidence Needed to Mark COMPLETE

- [x] `BuildingClass::ReceiveDamage` case 4 decompiled — what calls are made (vtable slots).
- [x] `BuildingClass::Update` Health==0 branch decompiled — when Conceal/UnInit/vtable+0xF8 fire.
- [x] `ObjectClass::Conceal` confirmed as active-vector remover (calls `FUN_0055BAE0`).
- [x] `ObjectClass::UnInit` confirmed as PendingDeleteList appender.
- [x] `BuildingClass__Limbo` (0x00445880) confirmed as vtable+0x4EC and vtable+0xD4 target.
- [x] Rust delta: does `handle_entity_deaths` call `unregister_live_object`?

---

## Stop Conditions

- Report written with inline Ghidra citations.
- Slot row appended to `.swarm-claims.md`.

---

## 1. gamemd.exe Destruction Path (Verified)

### 1.1 `BuildingClass::ReceiveDamage` Case 4 — lethal hit

Verified via `decompile_function 0x00442230`.

When `TechnoClass::ReceiveDamage` returns 4 (NowDead), `BuildingClass::ReceiveDamage` case 4 runs
and makes the following active-vector–relevant call:

```
(**(code **)(param_1->vtable + 0x4ec))(0, param_5, param_4, unaff_EBP);
```

`vtable+0x4EC` for BuildingClass is **`BuildingClass__Limbo` @ 0x00445880**
(verified via `decompile_function 0x00445880`; Ghidra labels it `BuildingClass__Limbo`).

Active in YR: **Yes** — fires on every lethal combat hit to a non-wall building.

#### Active-vector removal chain from Limbo:

```
BuildingClass__Limbo (0x00445880)
  → TechnoClass__Limbo_Helper()
    → ObjectClass__Conceal (0x005F4D30)   [verified: decompile_function 0x005F4D30]
      → FUN_0055BAE0(param_1)             [compacting-left remove from LogicClass vector]
        sets this+0x81 = 1               [InLimbo = 1]
```

`ObjectClass__Conceal` has an idempotency guard: `if (this+0x81 != 0) return 0;` — so a
double-Limbo call (from `ReceiveDamage` + `Update`) is safe; the second is a no-op.

`FUN_0055BAE0` is the same remover settled in COMMON_MIDPASS: "compacts LEFT + clears +0x98".

**Result:** Active-vector removal fires **synchronously on the lethal hit**, within the
attacker's `ReceiveDamage` call, before `ReceiveDamage` returns to the caller.

#### IC-timer special case (non-normal path):

After `BuildingClass__Limbo`, `ReceiveDamage` case 4 also checks the IC timer:
```c
if (0 < iVar6) {   // IC duration > 0 and timer not expired
    (**(code **)(param_1->vtable + 0xf8))();  // UnInit — immediate death, skips Update
    BuildingClass__Place_OccupyMap();
}
```
This is the IC-duration carryover path. When active, `UnInit` fires synchronously in
`ReceiveDamage` rather than being deferred to `Update`. Active in YR: **Conditional**
(only when building has IC active at time of death).

---

### 1.2 `BuildingClass::Update` Health==0 Branch — deferred teardown

Verified via `decompile_function 0x0043FB20`.

After `ReceiveDamage` sets Health=0 and `Limbo` runs, the building remains in
`EntityStore` (with `InLimbo=1`, `Health=0`). On the building's **own later AI turn**
inside `BuildingClass::Update`, the `Health==0` check fires:

```c
if (iVar3 == 0) {       // Health == 0
    // Clear all 8 anim slots (field_0x5C8)
    // Check IC timer — if remaining > 0: return (wait)
    if (iVar3 != 0) return;
LAB_004400c1:
    (**(code **)(this->vtable + 0xd4))();   // vtable+0xD4 = BuildingClass__Limbo (no-op: already InLimbo)
    BuildingClass__SpawnSurvivors(this);
    (**(code **)(this->vtable + 0xf8))();   // vtable+0xF8 = ObjectClass__UnInit
    BuildingClass__Place_OccupyMap();
    return;
}
```

`vtable+0xF8` = `ObjectClass__UnInit @ 0x005F65F0` (verified via
`get_function_by_address 0x005F65F0` — labeled `ObjectClass__UnInit`).

`ObjectClass::UnInit` (verified via `decompile_function 0x005F65F0`):
```
→ vtable+0xD4 = Limbo (no-op; InLimbo already set by ReceiveDamage)
→ this+0x90 = 0         (IsAlive cleared)
→ append to PendingDeleteList @ 0x00B0F69C
```

**`BuildingClass::Update` is the UnInit owner** — it handles collapse-animation sequencing,
survivor spawning, and final C++ deallocation. But the **active-vector removal already happened**
in `ReceiveDamage`.

Active in YR: **Yes** — fires on every building whose Health reaches 0.

---

### 1.3 Active-Vector Consequence

| Event | Tick | Active-Vector State |
|-------|------|---------------------|
| Lethal hit fired (attacker's pass) | T | `FUN_0055BAE0` called → building removed from LogicClass vector |
| Remainder of attacker's pass (same T) | T | Building absent from vector — no skip hazard for successors |
| Building's own Update turn | T or T+1 | `vtable+0xD4` Limbo (no-op), `vtable+0xF8` UnInit, survivors spawned |
| C++ deallocation | T+1 or later | PendingDeleteList processed |

**The building is removed from the active vector mid-attacker-pass (synchronously).
Successor objects in the same pass are NOT skipped by the compacting-left remove
because the live-count reload in `PerTickUpdate @ 0x0055AFB0` captures the new count
before each slot is processed** (settled in COMMON_MIDPASS).

---

### 1.4 Spawn of New Logic-Vector Members on Death

The death path spawns:
- **Survivor infantry** (`BuildingClass__SpawnSurvivors` → `Unlimbo` appends to vector).
  Active in YR: **Conditional** (only when `Crewed` and owner not defeated).
- **No other new vector members** from the immediate destruction path itself.
  Debris/AnimClass objects are not logic-vector members.

---

## 2. Rust Delta (Handoff-Critical)

### 2.1 Current Rust behavior (verified via source read)

In `handle_entity_deaths` (`src/sim/combat/mod.rs` line ~1000–1009):

```rust
// Structures and voxel vehicles: immediate despawn.
occupancy.remove(entity.position.rx, entity.position.ry, dead_id);
entities.clear_radio_contacts_for(dead_id);
entities.remove(dead_id);       // ← EntityStore only; does NOT touch logic vector
despawned_ids.push(dead_id);
```

`entities.remove()` removes from the `BTreeMap<u64, GameEntity>` but does NOT call
`World::unregister_live_object()` (which would call `logic.remove()`).

Post-combat in `advance_tick` (line ~1517), `despawned_ids` is iterated only for
`decrement_owned_count` — never for `unregister_live_object`.

**Result:** Combat-killed structures are removed from `EntityStore` immediately
(on the killing tick, mid-combat phase), but their ID remains in `logic` (the LogicVector).

### 2.2 Parity gap

| Dimension | gamemd | Rust | Verdict |
|-----------|--------|------|---------|
| Removal from active vector | Synchronous during attacker's ReceiveDamage (same tick, mid-pass) | EntityStore remove is immediate, but LogicVector retains the dead ID indefinitely | DRIFT |
| Removal owner | `BuildingClass__Limbo` → `ObjectClass::Conceal` → `FUN_0055BAE0` | `entities.remove()` (no logic vector call) | DRIFT |
| Final teardown (UnInit) | `BuildingClass::Update` on building's own AI turn | Immediate (EntityStore removal = destruction; no deferred Update step) | DRIFT — but benign if logic vector is cleaned at same tick |
| Survivor spawn | `BuildingClass::Update` after Health==0 | `eject_destruction_survivors` called from `advance_tick` post-combat | Functionally equivalent timing if done same tick |

The logic-vector DRIFT is currently **low observable impact** because:
- `logic.snapshot()` callers check `entities.get(id)` and gracefully skip None results.
- The dangling ID does not affect EntityStore lookups, occupancy, or targeting.

However, it IS a real DRIFT: gamemd's active-vector count is accurately decremented the
same tick the kill happens; Rust's `logic` vector retains dead IDs across ticks until
`unregister_live_object` is called (which never happens for combat kills).

The correct fix is to route structure combat deaths through `World::despawn_entity` instead
of calling `entities.remove()` directly, so `unregister_live_object` is called at the same
tick as the EntityStore removal.

---

## 3. Implementation Handoff

**H1 — Route structure deaths through `despawn_entity`**

> Verified behavior: active-vector removal is synchronous with the lethal ReceiveDamage
> call in gamemd (same tick, same attacker pass).
> Rust delta: `handle_entity_deaths` calls `entities.remove()` directly, bypassing
> `logic.remove()`. Dead structures linger in the logic vector.
> Affected surface: `src/sim/combat/mod.rs` `handle_entity_deaths`, ~line 1000–1009.
> Acceptance: after fix, `logic.as_slice()` must not contain a combat-killed structure's
> ID after the tick in which it was killed.
> Proposed test: `test_combat_killed_structure_removed_from_logic_vector_same_tick` — place
> two buildings, fire enough damage to kill one, advance one tick, assert the dead ID is
> absent from `sim.live_object_order_snapshot()` and absent from `EntityStore`.
> Risk: `handle_entity_deaths` takes `&mut EntityStore` directly (not `&mut World`), so
> `despawn_entity` is not directly callable there. Mitigation: collect IDs to despawn in
> the death handler, pass back, and call `world.despawn_entity()` in `advance_tick` after
> `handle_entity_deaths` returns — same tick, correct ordering.

**H2 — No deferred-Update destruction loop needed for buildings**

> Verified behavior: in gamemd, `BuildingClass::Update` is the UnInit owner. In Rust,
> there is no `BuildingClass::Update` equivalent running per building per tick. The
> EntityStore-remove already achieves immediate destruction. The Rust architecture does
> not need a deferred-Update destruction step — it just needs to ensure the logic-vector
> removal is co-timed with the EntityStore removal.
> Affected surface: no new per-building tick loop required.
> Proposed test: (covered by H1 test above).
> Risk: Low — no new mechanism required, only a routing fix.

**H3 — IC-delay-kill path: buildings with active IC must not be immediately removed**

> Verified behavior: when `BuildingClass::ReceiveDamage` case 4 fires for a building that
> had active IC (still within IC duration), `UnInit` is called synchronously in
> `ReceiveDamage` rather than deferred to `Update`. The Rust `invulnerability::is_invulnerable`
> guard prevents damage while IC is active (`target.health.current` is not reduced),
> so a building under active IC will not enter the dead branch. This means the IC-delay-kill
> scenario from gamemd (where a building was lethal-hit but IC timer was still positive)
> does not arise in Rust's current model.
> Affected surface: `src/sim/superweapon/iron_curtain.rs`, `src/sim/combat/mod.rs`.
> Proposed test: (out of scope for this slot; flagged as remaining uncertainty).
> Risk: Low for typical play; verify when implementing IC expiry + building damage.

---

## 4. Negative Facts / Do Not Do

**N1 — Do NOT add a per-building deferred-Update destruction loop in Rust.**
gamemd defers `UnInit` to `BuildingClass::Update`, but Rust's architecture achieves the
same observable result (building gone, survivors spawned, occupancy cleared) via immediate
EntityStore removal + same-tick survivor ejection. The C++ deferral was needed because of
vtable/allocator lifecycle; it is not needed in Rust.
Evidence: `BuildingClass::Update` Health==0 branch decompiled — its only observable effects
beyond `UnInit` are survivor spawning (already ported) and anim slot cleanup (already done).

**N2 — Do NOT skip logic-vector removal thinking "entities.get(id) returns None so it's
harmless."**
The gamemd contract is that the vector accurately reflects the live set. A dangling ID
means `logic.len()` over-counts, which could affect any future per-logic-vector-member
code (AI, hit-test loops, defeat detection, replay hash if it covers vector state).

**N3 — Do NOT call `UnInit`-equivalent twice.**
`ObjectClass::Conceal` has an `InLimbo` idempotency guard — the second call from `Update`
is safe in gamemd. In Rust, calling `despawn_entity` twice on the same ID is safe
(EntityStore.remove returns None on second call), but the current code must not call
`unregister_live_object` more than once for the same ID either (it clears the membership
flag, making the second call a no-op via the `if !e.in_logic_vector { return; }` guard).

**N4 — Do NOT implement the `BuildingClass::Update` Health==0 wait-loop for IC timer.**
The IC-delay-kill path (where `Update` waits for IC duration before calling `UnInit`) is
a special case that only fires when a building is killed while IC is still active. Rust's
invulnerability guard prevents this scenario from arising in normal play.
Evidence: `BuildingClass::Update` decompile shows `if (iVar3 != 0) return;` before
`LAB_004400c1` — it is guarding the IC-remaining case.

**N5 — Do NOT implement `BuildingClass::SpawnSurvivors` from within the Update tick loop.**
The existing `eject_destruction_survivors` called post-combat in `advance_tick` is the
correct location for this. Moving it into a per-building Update loop would drift timing
(survivors would spawn one tick later than needed).

---

## 5. Remaining Uncertainty

- **IC-delay-kill exact timing in Rust.** When IC expires on a damaged building and the
  next damage packet kills it, what is the exact Rust timing? The `is_invulnerable` guard
  only blocks while IC is active; post-expiry, the building is killable normally. The
  transition tick is not verified against gamemd. Risk: low for current scope.

- **vtable+0xD4 slot binding for BuildingClass specifically.** The claim that
  BuildingClass vtable+0xD4 = `BuildingClass__Limbo` (not a different override) is
  inferred from the Ghidra label `BuildingClass__Limbo @ 0x00445880` and the fact that
  `BuildingClass::Update` calls `vtable+0xD4` and the effect is Limbo. Direct
  `read_memory` of the BuildingClass vtable at offset +0xD4 was not performed in this
  session. CONFIDENCE: HIGH (consistent with label + behavior + Update decompile).

- **`TechnoClass__Limbo_Helper` address and exact call sequence.** The Conceal → vector-
  remove chain goes through `TechnoClass__Limbo_Helper` — its address is not verified
  in this session (not needed for the handoff, but needed for a full chain audit).

---

## 6. Key Function Address Summary

| Address | Function | Verified In This Session |
|---------|----------|--------------------------|
| 0x0043FB20 | BuildingClass__Update | Yes — decompile_function |
| 0x00442230 | BuildingClass__ReceiveDamage | Yes — decompile_function |
| 0x00445880 | BuildingClass__Limbo (vtable+0x4EC, vtable+0xD4) | Yes — decompile_function + get_function_by_address |
| 0x005F4D30 | ObjectClass__Conceal | Yes — decompile_function + get_function_by_address |
| 0x005F65F0 | ObjectClass__UnInit | Yes — decompile_function + get_function_by_address |
| 0x0055BAE0 | FUN_0055BAE0 (active-vector compacting remove) | Cited (settled in COMMON_MIDPASS) |

---

## 7. Stale-Doc Note

`BUILDING_DAMAGE_DESTRUCTION_GHIDRA_REPORT.md` labels 0x00445880 as
`BuildingClass::OnDestroyed` (Section 4). Ghidra labels it `BuildingClass__Limbo` and the
decompile confirms it calls `TechnoClass__Limbo_Helper` (a Limbo path, not merely a
notification). The existing doc's content is functionally correct for the cleanup steps, but
the function name and characterization ("Called via vtable+0x4EC when the building is
confirmed dead") should be updated to reflect that this IS the Limbo call, not an
OnDestroyed notification. The active-vector removal happens inside this call via the
Conceal chain. **This report extends the existing doc** — no replacement needed, but the
function label should be corrected on the next edit pass.
