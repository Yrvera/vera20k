# CABHUT C4 Timer Expiry → BridgeRepairHut Branch Dispatch — Trace Report

**Slot:** 2 of 5 (C4 timer expiry → bridge-collapse signal)  
**Date:** 2026-05-20  
**Scenario:** CABHUT at cell (9,10), `pending_c4_detonation` set, World advances ticks until C4Delay reaches zero.  
**Scope:** From countdown tick through expiry detection → BridgeRepairHut branch check → hut survival → bridge-collapse signal emission. Stop at dispatch boundary (signal emitted or function called); do NOT trace bridge span enumeration (slot 3) or cell destruction (slot 4).

---

## Stage Results

| # | Stage | Verdict | Evidence |
|---|-------|---------|----------|
| 1 | Timer tick (per-tick countdown decrement) | **PASS** | See §1 |
| 2 | Expiry detection (BridgeRepairHut branch before damage) | **PASS** | See §2 |
| 3 | Hut survival (no C4Warhead damage applied to hut) | **PASS** | See §3 |
| 4 | Bridge-collapse signal emission | **PASS** | See §4 |
| 5 | Signal payload | **PASS** | See §5 |
| 6 | `pending_c4_detonation` cleanup after expiry | **PASS** | See §6 |
| 7 | CABHUT vs non-CABHUT C4 control path | **PASS** | See §7 |

**Verdict tally: PASS: 7 | FAIL: 0 | UNCHECKED: 0 | NOT-IMPLEMENTED: 0**

---

## §1 — Timer Tick: Per-Tick Countdown Decrement

### gamemd (verified by live decompile at `0x43FB20`)

```c
// From BuildingClass::Update decompilation (0x43FB20):
if (this->field_0x6df != '\0') {
    iVar3 = *(int *)&this->field_0x530;   // delay_frames = C4Delay (frames)
    if (*(int *)&this->field_0x528 == -1) {
        // timer not started — fires only if delay == 0
        if (iVar3 != 0) goto LAB_00440378;  // skip this tick
    } else {
        iVar12 = g_CurrentFrameCounter - *(int *)&this->field_0x528;  // elapsed
        if (iVar12 < iVar3) {          // elapsed < delay_frames?
            iVar3 = iVar3 - iVar12;    // remaining = delay - elapsed
            goto LAB_004401fe;         // → if remaining != 0, skip this tick
        }
        // falls through here when elapsed >= delay_frames → FIRE
    }
    // ... detonation body
}
```

Expiry condition: `g_CurrentFrameCounter - field_0x528 >= field_0x530`

`field_0x528` is set to `g_CurrentFrameCounter` at plant time (inside `InfantryClass::PerCellProcess @ 0x51A5A7`).  
`field_0x530` is set to `RulesClass + 0xFD0` (C4Delay in frames) at plant time.  
Default C4Delay = `0.03` minutes × 60 × 15 fps = **27 frames**.

### Our code (`src/sim/world/world_orders.rs:503–513`)

```rust
let delay = rules.c4_delay_ticks as u64;
// ...
if self.tick.saturating_sub(pending.plant_start_tick) < delay {
    continue;  // timer still ticking
}
// falls through when elapsed >= delay → FIRE
```

`pending.plant_start_tick` is set to `self.tick` at plant time.  
`rules.c4_delay_ticks` is parsed from `[CombatDamage] C4Delay` (minutes × 60 × 15, rounded). Default = **27 ticks**.

### Comparison

Both implementations fire when `elapsed >= delay`. The comparison is identical in direction and semantics. Tick units correspond directly (gamemd frames = our ticks at 15 fps). The `field_0x528 == -1` sentinel in gamemd (immediate-fire on zero delay) maps cleanly to `plant_start_tick = 0` falling through when `delay = 0`.

**Result: PASS** — timer math matches gamemd exactly.

---

## §2 — Expiry Detection: BridgeRepairHut Branch BEFORE Damage

### gamemd (decompile `0x43FB20`)

After timer expiry falls through:

```c
iStack_28 = this->Health;
if (this->Type[0x16b6] == '\0') {
    // NOT BridgeRepairHut → apply damage to self
    (*vtable[0x16C])(&iStack_28, 0, RulesClass+0xFA8 /*C4Warhead*/, field_0x540, 1, 0, 0);
} else {
    // IS BridgeRepairHut → destroy bridge (5×5 scan + DestroyBridge dispatch)
    // field_0x6df = 0; field_0x540 = 0;  ← cleared ONLY in this branch
}
```

`Type[0x16b6]` = `BridgeRepairHut` (verified: `BuildingTypeClass::ReadINI @ 0x460E8D`).

The `BridgeRepairHut` check is the **outermost branch** at the detonation site. The damage call (`vtable[0x16C]`) is INSIDE the `Type[0x16b6] == '\0'` branch — it is never reached for CABHUT. There is no intermediate flag check, no Immune gate, no IronCurtain gate between timer expiry and the branch dispatch.

### Our code (`src/sim/world/world_orders.rs:719–753`)

```rust
fn apply_c4_damage_to_building(&mut self, building_id, damage, warhead_id, attacker_id, rules) -> C4DamageOutcome {
    let target_bridge_hut = self.entities.get(building_id)
        .and_then(|b| rules.object(self.interner.resolve(b.type_ref)).map(|t| t.bridge_repair_hut))
        .unwrap_or(false);
    if target_bridge_hut {
        // → bridge collapse path (hut survives)
        return C4DamageOutcome { killed_building: false, bridge_state_changed: ..., consumed_pending_marker: true };
    }
    // → generic damage path (only reached for non-hut buildings)
    // IC check, warhead resolution, HP subtraction, dying flag
}
```

The `bridge_repair_hut` flag check is the **first check** in the function, before the IronCurtain gate and before any HP modification. Matches gamemd's branch order exactly.

Note: In gamemd, `field_0x6df` clear is ONLY in the BridgeRepairHut branch. In our code, `consumed_pending_marker: true` triggers `pending_c4_detonation = None` in the caller (world_orders.rs:551–554), which is also only executed for hut targets. Non-hut buildings fall through to the `killed_building` path, where `pending_c4_detonation` is only cleared by entity despawn. This matches.

**Result: PASS** — BridgeRepairHut branch fires before damage, matches gamemd branch order.

---

## §3 — Hut Survival: No C4Warhead Damage Applied to Hut

### gamemd

In the `BridgeRepairHut` branch, `vtable[0x16C]` (TakeDamage / ReceiveDamage) is **never called**. The `iStack_28 = this->Health` line sets up a local variable for the non-hut path but is not used in the hut branch. The hut's HP field is never written.

### Our code

`apply_c4_damage_to_building` returns immediately with `killed_building: false` when `target_bridge_hut` is true, before any HP mutation:

```rust
if target_bridge_hut {
    // ... dispatch_bridge_collapse_from_hut(...)
    return C4DamageOutcome { killed_building: false, bridge_state_changed: ..., consumed_pending_marker: true };
}
// HP subtraction only reached when target_bridge_hut == false
```

No HP mutation, no `dying = true`, no `last_attacker_id` update for hut targets.

Integration test `c4_on_cabhut_collapses_bridge_and_hut_survives` (world_orders_bridge_repair_tests.rs:535–617) asserts `hut.health.current == cabhut_max_hp` across every tick of the C4Delay window and post-detonation, and `!hut.dying`. **Test passes** (confirmed by `cargo test c4_on_cabhut` run: 6/6 tests pass).

**Result: PASS** — hut HP preserved, entity not marked dying.

---

## §4 — Bridge-Collapse Signal Emission

### gamemd (decompile `0x43FB20`, BridgeRepairHut branch)

```c
// 5×5 scan for low/high bridge overlay:
iVar3 = -2;
do {
    iVar12 = -2;
    do {
        psVar6 = (*vtable[0x1B8])(auStack_24);  // GetCoord
        // scan cells ±2 in x and y
        // check DAT_00abad1c (high-bridge overlay range) + cell[0x44] (low-bridge 0x4A..0x65)
    } while (iVar12 < 3);
    iVar3++;
} while (iVar3 < 3);

if (uStack_3c._3_1_ == '\0') {   // high-bridge found
    uVar8 = (*vtable[0x1B8])(auStack_20);
    MapClass__DestroyBridge_High_OnHutDeath(uVar8);  // = DestroyBridge_High_MapInit (0x574000)
} else {                          // low-bridge found
    uVar8 = (*vtable[0x1B8])(auStack_20);
    MapClass__DestroyBridge_Low_OnHutDeath(uVar8);   // = DestroyBridge_Low_MapInit (0x574C20)
}
this->field_0x6df = 0;
*(undefined4 *)&this->field_0x540 = 0;
```

The signal is a **direct function call** to `MapClass::DestroyBridge_Low/High_MapInit` with the hut's coordinate as argument. No event queue, no deferred dispatch — immediate synchronous call.

### Our code (`src/sim/world/world_orders.rs:739–746` + `bridge_orchestrator.rs:165–226`)

```rust
let bridge_state_changed = match bld_center {
    Some(center) => {
        crate::sim::world::bridge_orchestrator::dispatch_bridge_collapse_from_hut(
            self, rules, center,
        )
    }
    None => false,
};
```

`dispatch_bridge_collapse_from_hut` at `bridge_orchestrator.rs:165`:
1. Performs a 5×5 scan via `cells_in_5x5_scan(hut_center)` (matches gamemd's ±2 loop)
2. Calls `choose_hut_bridge_family` to detect low/high (matches gamemd's low/high decision)
3. Dispatches directly into the direct-overlay collapse sweep

This is a synchronous direct call mirroring gamemd's synchronous `MapClass::DestroyBridge_*` call. No deferred event queue is used (unlike the combat `BridgeDamageEvent` path).

**Result: PASS** — bridge-collapse signal emitted synchronously as a direct function call, matching gamemd's call structure.

---

## §5 — Signal Payload

### gamemd

Payload to `MapClass::DestroyBridge_Low/High_MapInit(coord)`:
- `coord` = result of `vtable[0x1B8](auStack_20)` = CABHUT's map coordinate (same call used for the scan center)
- No additional parameters; function uses the coord to scan/walk the bridge span

### Our code

Payload to `dispatch_bridge_collapse_from_hut(sim, rules, hut_center)`:
- `hut_center` = `(b.position.rx, b.position.ry)` = CABHUT's map cell coordinate
- `rules` for BridgeStrength and warhead lookups (needed for cascade, not the scan itself)

Both pass the hut's coordinate as the scan origin. Our function uses `(rx, ry)` cell coordinates directly, equivalent to gamemd's `vtable[0x1B8]` coord (which returns the same map cell). The scan in both cases is 5×5 centered on the hut.

**Result: PASS** — payload matches: hut center coordinate as scan origin.

---

## §6 — `pending_c4_detonation` Cleanup

### gamemd

`field_0x6df = 0` and `field_0x540 = 0` are written ONLY inside the `BridgeRepairHut` branch (after the DestroyBridge call). For the non-hut branch, `field_0x6df` is NOT cleared — it stays set until the building dies (and disappears with the object). This means the damage path re-fires every tick until `Health == 0` (killing the building).

### Our code

`consumed_pending_marker: true` is returned from the hut branch. In `tick_c4_plants` (world_orders.rs:551–554):
```rust
} else if outcome.consumed_pending_marker {
    if let Some(building) = self.entities.get_mut(building_id) {
        building.pending_c4_detonation = None;
    }
    // also clears attacker's c4_plant
}
```

For non-hut buildings: `killed_building: true` path — `pending_c4_detonation` is cleared by entity despawn (comment: "pending_c4_detonation goes away with the entity via despawn path"). This matches gamemd's "field_0x6df is only cleared explicitly in the hut branch; non-hut buildings are destroyed (removed) before re-fire matters."

Integration test `c4_on_cabhut_collapses_bridge_and_hut_survives` asserts `hut.pending_c4_detonation.is_none()` post-detonation (line 586–588). Passes.

**Result: PASS** — cleanup matches: hut clears marker after dispatch; non-hut clears via entity removal.

---

## §7 — CABHUT vs Non-CABHUT C4 Control Path

### gamemd

- CABHUT (`Type[0x16b6] != 0`): skip vtable[0x16C], call DestroyBridge_*_MapInit, clear flags
- Non-CABHUT (`Type[0x16b6] == 0`): call `vtable[0x16C](&health, C4Warhead, engineer_ptr, ...)` — repeated every tick until HP reaches 0

### Our code

- CABHUT (`bridge_repair_hut == true`): `dispatch_bridge_collapse_from_hut`, return `C4DamageOutcome { killed_building: false, consumed_pending_marker: true }`
- Non-CABHUT (`bridge_repair_hut == false`): IronCurtain check → warhead resolution → HP subtraction → `dying = true` when HP == 0

Integration test `c4_on_standard_building_kills_building` (world_orders_c4_tests.rs) confirms a non-CABHUT building (GAPILE) is killed by C4. Test passes.

The divergence at `bridge_repair_hut` is the correct and only branch point, matching gamemd's `Type[0x16b6]` check.

**Result: PASS** — CABHUT takes hut-branch; non-CABHUT takes damage-branch; both match gamemd.

---

## Timer Math Correction to §3.2 Pseudocode in Bridge Report

The pseudocode in `BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md §3.2` reconstructed the timer logic as:
> "if (elapsed < delay_frames) { delay_frames -= elapsed; // timer still ticking — fall through, no bridge action yet }"
> "if (delay_frames == 0) { ... }"

This implied the `delay_frames == 0` check would only trigger on explicitly-zero delay. The **live decompile** (`0x43FB20`) clarifies the actual control flow:

```c
if (iVar12 < iVar3) {         // elapsed < delay_frames
    iVar3 = iVar3 - iVar12;   // remaining = delay - elapsed
    goto LAB_004401fe;         // → if (remaining != 0) goto LAB_00440378 (skip)
}
// falls through when iVar12 >= iVar3 (elapsed >= delay) → FIRE
```

The detonation fires when `elapsed >= delay_frames`, NOT when `delay_frames == 0`. The `delay_frames == 0` check in the pseudocode was a simplification that was misleading. The **actual condition** is: `g_CurrentFrameCounter - field_0x528 >= field_0x530`. Our code's `self.tick.saturating_sub(pending.plant_start_tick) >= delay` is semantically identical. No bug here; the two implementations agree.

This is an advisory note for the research doc — not a parity failure.

---

## Adjacent Findings

1. **`field_0x6df` clear is CABHUT-branch-only in gamemd** (confirmed by decompile): for non-hut buildings, `field_0x6df` is never cleared explicitly — it remains set until the building is destroyed. Our implementation correctly handles this via entity despawn clearing the `pending_c4_detonation` component. No parity gap.

2. **gamemd `iStack_28 = this->Health`** is computed unconditionally before the BridgeRepairHut branch — but the value is only passed to `vtable[0x16C]` in the non-hut path. Our code reads `b.health.current` only in the non-hut path. Functionally equivalent; the unconditional read in gamemd is a compiler artifact.

3. **IronCurtain bypass for CABHUT**: gamemd's BridgeRepairHut branch skips `vtable[0x16C]` entirely, so there is no IronCurtain check in gamemd for the CABHUT path. Our code also skips the IronCurtain check (`is_invulnerable`) for hut targets — the `bridge_repair_hut` early return fires before the IC check. Test `c4_on_invulnerable_cabhut_still_dispatches_bridge_and_clears_pending` (world_orders_bridge_repair_tests.rs:648) confirms IC-curtained CABHUT still dispatches bridge collapse. Matches gamemd.

4. **Pending-marker-already-claimed guard** (world_orders.rs:433–440): our Phase 1 walk-up checks `pending_c4_detonation.is_some()` before a second SEAL can claim. This maps to gamemd's `field_0x6df == 0` check in `PerCellProcess` Mission_Sabotage (bridge report §3.6). No gap.

5. **The user-reported symptom** ("only 1 small piece of bridge falls") is in slot 3/4, not slot 2. Slot 2 (this report) is fully implemented and passing 6/6 tests. The symptom must originate in the bridge span enumeration (slot 3) or cell destruction/cascade (slot 4).

---

## Sources

- `BuildingClass::Update @ 0x43FB20` — live decompile via Ghidra MCP (this session)
- `BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md §3.2, §14` — primary spec
- `C4_ON_BRIDGE_REPAIR_HUT_GATE_INVESTIGATION.md §7` — confirms BridgeRepairHut branch, hut survival
- `src/sim/world/world_orders.rs:387–570, 707–804` — Phase 1 walk-up + Phase 2 detonation
- `src/sim/world/bridge_orchestrator.rs:155–226` — dispatch_bridge_collapse_from_hut
- `src/sim/world/world_orders_bridge_repair_tests.rs:535–617` — integration test (6/6 pass)
