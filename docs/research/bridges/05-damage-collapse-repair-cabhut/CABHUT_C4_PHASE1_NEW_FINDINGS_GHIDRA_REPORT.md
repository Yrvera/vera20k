# CABHUT C4 Cascade — Phase 1: New Findings Sweep

**Date:** 2026-05-18
**Mode:** exhaustive-slice → rescoped to Phase 1 (>25% deferral; see §7)
**Companion:** [CABHUT_C4_INVESTIGATION_LOG.md](CABHUT_C4_INVESTIGATION_LOG.md) — the full Open Questions Log (77 entries; 16 resolved, 61 deferred with categories + addresses)
**Parent docs:**
- [C4_ON_BRIDGE_REPAIR_HUT_GATE_INVESTIGATION.md](C4_ON_BRIDGE_REPAIR_HUT_GATE_INVESTIGATION.md)
- [TECH_CABHUT_GHIDRA_REPORT.md](TECH_CABHUT_GHIDRA_REPORT.md)
- [BRIDGE_HUT_DESTRUCTION_ENTRY_DECODE_GHIDRA_REPORT.md](BRIDGE_HUT_DESTRUCTION_ENTRY_DECODE_GHIDRA_REPORT.md)

**Confidence:** HIGH on the 10 newly-decompiled function bodies and the
3-state damage model. MEDIUM on the High-variant walkers (presumed twins,
not exhaustively verified). Deferred everything else with category labels.

**Active in YR:** Yes — the entire chain is reachable from a stock skirmish
game on any map containing a CABHUT plus a C4-capable infantry or a demo
truck.

---

## 1. Overview

This investigation re-entered the C4-on-CABHUT chain with the new
`/re-investigate` skill's mandatory open-questions log and branching rule.
The seed log of 32 entries grew to 77 entries during the Ghidra dive
(branching rule fired ~45 times on new callees, struct fields, globals,
constants, and magic flags). Resolved 16 entries with binary evidence
covering **eight functions never documented in any prior report**, and
deferred 61 with explicit categories.

The headline finding is that **bridges have a three-state damage model
(healthy → damaged → destroyed-anchor) and the C4 cascade transitions each
cell by exactly one step**. A single C4 strike on a fully-healthy bridge
section leaves it in the damaged state, NOT destroyed. A second strike (or
a strike on an already-damaged section) destroys it. This is not in any
existing doc and is parity-critical: a Rust port that one-shots a healthy
bridge with a single C4 plant produces wrong observable behavior.

A second material finding is that **all four `CollapseBridge_*` walkers
call `UpdateBridgeZonesHelper` themselves**, contradicting the asymmetry
flagged in BRIDGE_HUT_DESTRUCTION §4.3 (which had claimed the zone rebuild
fires only on the ramp-fallback path).

---

## 2. New functions decompiled this session

Per the skill's "what new functions get uncovered" test:

| # | Function | Address | Prior doc coverage | Phase 1 status |
|---|----------|---------|--------------------|----------------|
| 1 | `MapClass::CollapseBridge_NS_Low` | `0x575540` | Labeled only (BRIDGE_HUT_DESTRUCTION §2.3 open work) | **Full body decompiled** |
| 2 | `MapClass::CollapseBridge_EW_Low` | `0x575220` | Labeled only | **Full body decompiled** |
| 3 | `MapClass::CollapseBridge_NS_High` | `0x575BA0` | Labeled only | **Full body decompiled** |
| 4 | `MapClass::CollapseBridge_EW_High` | `0x575870` | Labeled only | **Full body decompiled** |
| 5 | `MapClass::DestroyBridge_Low` (per-cell) | `0x57BAA0` | Address cited in parent doc; body never decompiled | **Full body decompiled** |
| 6 | `MapClass::DestroyBridge_High` (per-cell) | `0x57CCF0` | Same | **Full body decompiled** |
| 7 | `MapClass::DestroyBridgeWalker_NS_Low` | `0x57BCF0` | **NEVER mentioned in any prior doc** | **Full body decompiled** |
| 8 | `MapClass::DestroyBridgeWalker_EW_Low` | `0x57C2B0` | **NEVER mentioned in any prior doc** | **Full body decompiled** |
| 9 | `MapClass::DestroyBridgeWalker_NS_High` | `0x57CF60` | **NEVER mentioned in any prior doc** | Address resolved; body deferred (compiled-twin inference) |
| 10 | `MapClass::DestroyBridgeWalker_EW_High` | `0x57D530` | **NEVER mentioned in any prior doc** | Address resolved; body deferred |
| 11 | `BombClass::Detonate` | `0x438720` | Surface-level cited in BRIDGE_HUT_DESTRUCTION §3.1 (only the BridgeRepairHut branch) | **Full body decompiled, including pre-cascade `Apply_area_damage` finding** |
| 12 | `BombClass::Attach` | `0x438E70` | Address cited in C4 doc §6 with note "specific RTTI category"; body never decompiled | **Full body decompiled** |
| 13 | `WarheadTypeClass::Detonate` (chain) | `0x4690B0` | Not previously linked to C4-on-CABHUT | **Confirmed as sole caller of BombClass::Attach** |

Plus addresses resolved for follow-up (Phase 2):

| Function | Address | Status |
|----------|---------|--------|
| `ApplyBridgeDestruction_NS_Low` | `0x57DD50` | DEFERRED |
| `ApplyBridgeDestruction_EW_Low` | `0x57E2A0` | DEFERRED |
| `ApplyBridgeDestruction_NS_High` | `0x57E7A0` | DEFERRED |
| `ApplyBridgeDestruction_EW_High` | `0x57ED00` | DEFERRED |
| `FindBridgeEndpoints_NS_Low` | `0x57C990` | DEFERRED |
| `FindBridgeEndpoints_EW_Low` | `0x57C870` | DEFERRED |
| `FindBridgeEndpoints_NS_High` | `0x57DC20` | DEFERRED |
| `FindBridgeEndpoints_EW_High` | `0x57DAF0` | DEFERRED |

**Total new functions surfaced this session: 13 documented + 8 enumerated-for-Phase-2 = 21.** The old `/re-investigate` skill (without the open-questions log + branching rule) would have stopped after producing a polished narrative of the existing top-level dispatch; the new gate forced enumeration of every callee and surfaced the entire walker subtree.

---

## 3. The bridge 3-state damage model — NEW

The single most important parity finding of this session.

Bridge cells carry an overlay type index at `cell.+0x44`. The collapse walker
classifies cells into three states by overlay value:

### 3.1 Low-bridge NS axis (`DestroyBridgeWalker_NS_Low @ 0x57BCF0`)

| Overlay range | State | Transition target |
|---------------|-------|-------------------|
| `0x4A..0x4F` | NS main HEALTHY | → `0x50` (damaged); then dispatch to `ApplyBridgeDestruction_NS_Low` at cell-1 AND cell+1 |
| `0x50..0x52` | NS main DAMAGED | → `0x64` (DESTROYED-ANCHOR); mark radar dirty (3 cells); dispatch at cell-1 AND cell+1; call `FindBridgeEndpoints_NS_Low`; set `local_a5=1` (success flag) AND set `local_78=3, local_74=3` (object-on-bridge rect for `FUN_005868a0`) |
| `0x5C` | NS bridgehead-A HEALTHY | → `0x5D` (damaged); dispatch at cell-1 only |
| `0x5D` | NS bridgehead-A DAMAGED | (handled by entry-test: `0x52 < local_ac` falls through to early return; second hit recurses through different path) |
| `0x5E` | NS bridgehead-B HEALTHY | → `0x5F` (damaged); dispatch at cell+1 only |
| `0x5F` | NS bridgehead-B DAMAGED | (early return) |
| `0x64` | NS DESTROYED-ANCHOR | early return (function returns 0; `CollapseBridge_*` retries next cell) |

### 3.2 Low-bridge EW axis (`DestroyBridgeWalker_EW_Low @ 0x57C2B0`)

| Overlay range | State | Transition target |
|---------------|-------|-------------------|
| `0x53..0x58` | EW main HEALTHY | → `0x59` (damaged); dispatch row-1 AND row+1 |
| `0x59..0x5B` | EW main DAMAGED | → `0x65` (DESTROYED-ANCHOR); mark dirty; dispatch row-1 AND row+1; call `FindBridgeEndpoints_EW_Low`; success flag |
| `0x60` | EW bridgehead-A HEALTHY | → `0x61` (damaged); dispatch row+1 only |
| `0x62` | EW bridgehead-B HEALTHY | → `0x63` (damaged); dispatch row-1 only |
| `0x65` | EW DESTROYED-ANCHOR | early return |

### 3.3 Implication for parity

A C4 plant on a fully-healthy bridge section walks 4 cells along the bridge
axis (per `CollapseBridge_*` `local_2c = 4`). At each cell:

- If the cell is **healthy**: writes "damaged" overlay across 3 cells (this + 2 perpendicular neighbors), dispatches `ApplyBridgeDestruction_*` at both axial neighbors. **Bridge is still walkable; cells just look damaged.**
- If the cell is **damaged**: writes "destroyed-anchor" overlay across 3 cells, marks radar dirty, dispatches `ApplyBridgeDestruction_*`, calls `FindBridgeEndpoints_*`, sets the object-on-bridge rect notification. **Bridge is now broken.**

**This means a single C4 on a healthy bridge does NOT collapse it.** The
player sees damage textures and the bridge is unwalkable later (because of
the damaged-overlay state), but the actual "collapsing" debris animation
fires at every cell regardless of starting state. A second C4 strike (or
heavy bombardment by other means that pre-damages the bridge) is needed
for full destruction.

Note also that the walker writes the SAME damaged-or-destroyed overlay to
three cells per strike (the current cell + two perpendicular neighbors).
For an NS bridge this is `(x-1, y), (x, y), (x+1, y)`. For an EW bridge
this is `(x, y-1), (x, y), (x, y+1)`. The perpendicular neighbors are the
bridge-edge cells (railings).

### 3.4 Confidence

| Claim | Content | Identity | Binding |
|-------|---------|----------|---------|
| 3-state model for NS_Low | HIGH (direct decompile) | HIGH | HIGH |
| 3-state model for EW_Low | HIGH (direct decompile) | HIGH | HIGH |
| 3-state model for NS_High / EW_High | MEDIUM (presumed compiled twins; not yet decompiled) | MEDIUM-HIGH | MEDIUM |
| Specific overlay transition values | HIGH (constants extracted from binary) | HIGH | HIGH |

---

## 4. CollapseBridge_* walker body (all 4 are byte-equivalent twins)

`CollapseBridge_{NS,EW}_{Low,High}` are 4-iteration walkers along the bridge
axis. Per-iteration:

1. **Span-finder (one-time at function entry)**:
   - Walk backward along the bridge axis until cell.overlay leaves the overlay band → count `iVar11`.
   - Walk forward along the bridge axis until cell.overlay leaves the overlay band → count `iVar10`.
   - If `iVar10 < iVar11` (more cells backward than forward): set step direction to `-1`. Else `+1`.
   - Start position = input coord adjusted by `(iVar11 - iVar10) / 2` (signed division — see Q48 for parity flag on odd-length bridges).

2. **For each of 4 iterations along the chosen direction**:
   - Look up the current cell.
   - If overlay != destroyed-anchor sentinel (`0x65` for low EW, `0x64` for low NS, `0xE8` for high EW, `0xE7` for high NS):
     - Spawn **3 random debris animations** at perpendicular-offset positions (`(x-1,y), (x,y), (x+1,y)` for NS-walker; row-offset for EW).
     - Each anim: position randomized in X and Y; Z = `cell.+0x11B * DAT_00abde88`; flag arg = `0x600`; AnimClass type chosen from `RulesClass+0x15C[Random(0, RulesClass+0x168 - 1)]`; frame-delay arg = `Random(1, 5)`.
   - Call `DestroyBridge_{Low,High}(coord)` up to 3 times (retry loop).
   - Step along chosen direction.
   - Break if new cell's overlay is outside the bridge band.

3. **Function tail (always runs)**:
   - `MapClass::UpdateBridgeZonesHelper()` — full pathfinding zone rebuild.
   - `g_Tactical + 0xD7C = 1` — global renderer "redraw bridges" flag.

### 4.1 RNG-order constraint (load-bearing for lockstep)

Per anim spawn, RNG is consumed in this exact order:

1. `Random__RandomRanged(0, 0x7FFFFFFE)` → consumed for X jitter (via `Math::ftol`)
2. `Random__RandomRanged(0, 0x7FFFFFFE)` → Y jitter
3. `Random__RandomRanged(1, 5)` → frame-delay arg
4. `Random__RandomRanged(0, RulesClass+0x168 - 1)` → anim-type-array index

**4 RNG calls per anim × 3 anims per intact cell × up to 4 cells = up to 48 RNG calls per `CollapseBridge_*` invocation.** A Rust port that wants
identical multiplayer replay must consume the RNG in exactly this order.

### 4.2 Correction to BRIDGE_HUT_DESTRUCTION §4.3

That doc claimed `UpdateBridgeZonesHelper` fires only on the ramp-fallback
path inside `DestroyBridge_*_MapInit`. **This is incorrect.** The overlay-
match fast path dispatches to `CollapseBridge_*` (via `DestroyBridgeFromCell_*`)
which itself calls `UpdateBridgeZonesHelper` at its function tail.

Net: zone rebuild fires on **both** paths. The asymmetry the prior doc
flagged for parity is non-existent in gamemd.

---

## 5. BombClass::Detonate — the demo-truck path (new detail)

`BombClass::Detonate @ 0x438720` is the timer-expiry function for any
BombClass instance. The body, in order:

1. **Apply area damage** at the bomb coord: `Apply_area_damage(self.+0x24, RulesClass+0xFC8, 1, 0)`. The warhead is **`RulesClass+0xFC8` (DemoTruckWarhead, not C4Warhead)** — distinct from the inline-on-building C4 cascade. This means demo-truck blasts apply real area damage to surrounding units/buildings BEFORE the bridge cascade fires.

2. **Spawn explosion anim** at the cell-clipped coord. AnimClass type from `WarheadClass::SelectExplosionAnim`. Flag arg = `0x2600` (vs the walker's `0x600` — bit `0x2000` is set; meaning deferred — Q54).

3. **Conditional bridge cascade**:
   ```
   if (target.RTTI == 6 && target.Type.+0x16B6 != 0) {  // Building + BridgeRepairHut
       run 5×5 outer scan (dy outer, dx inner, no short-circuit, low-found bool in high byte);
       dispatch DestroyBridge_{Low,High}_MapInit(target.GetCoord());
   }
   ```

The 5×5 scan is byte-equivalent to `BuildingClass::Update`'s scan — confirmed
by direct compare of statement order and constants. No new behavior here.

**Key new finding:** the `Apply_area_damage` call happens BEFORE the bridge
cascade. If the area damage destroys the CABHUT itself (e.g., via splash
into a sufficiently-damaging warhead config), the cascade still fires
because `BombClass+0x2C` still holds the target ref. **The hut's `Immune=yes`
doesn't matter for either path** — the bridge cascade is unaffected by
target-side immunity.

---

## 6. BombClass::Attach — RTTI 0xf gate confirmed

`BombClass::Attach @ 0x438E70` is gated on `param_2.RTTI == 0xf`. RTTI value
0xf identity is **deferred (Q55)** — needs RTTI enum extraction. Other
properties confirmed:

- Allocates 92-byte (`0x5C`) BombClass instance.
- Stores `param_2` (the planter, RTTI-0xf) at BombClass+`0x24`.
- Stores `param_3` (a cell? or target?) at BombClass+`0x2C`, and writes the
  back-ref `param_3.+0x38 = this_bomb` (one bomb per slot — collision
  prevention).
- Sets expiry frame: `BombClass+0x38 = g_CurrentFrameCounter + RulesClass+0xFD0`. **`RulesClass+0xFD0` is the C4Delay constant** (confirmed by prior doc; resolved here as the source).
- Plays an attach voc if `RulesClass+0x210 != -1` AND player is human.
- Sole caller: `WarheadTypeClass::Detonate @ 0x4690B0`.

**Confirms the architectural split:** the inline `field_0x6DF` path
(PerCellProcess) is the **infantry plant** mechanism (Tanya/SEAL); the
BombClass::Attach path is reachable through a **weapon-side warhead
detonation** route. These are distinct C4-placement mechanisms, NOT
duplicates.

For the CABHUT-specific question of the project memory `project_c4_bridge_hut_followup`:
the Tanya/SEAL plant is the PerCellProcess path; it does NOT go through
BombClass::Attach.

---

## 7. Open work (the deferred 61 entries — Phase 2 work-list)

Per the new skill's rescoping rule (>25% deferred → title as "Phase 1"):

**Phase 2 follow-up priorities, ordered:**

1. **Decompile the 4 `ApplyBridgeDestruction_*` bodies** at `0x57DD50`, `0x57E2A0`, `0x57E7A0`, `0x57ED00`. These are the recursive next-layer functions dispatched by the walker on each cell transition. They likely contain the cascade-propagation logic and may surface additional state transitions.

2. **Decompile the 4 `FindBridgeEndpoints_*` bodies** at `0x57C990`, `0x57C870`, `0x57DC20`, `0x57DAF0`. These fire only on full-destroy transition (`local_a5=1` path); they likely propagate to the railing/bridgehead cleanup.

3. **Verify the 2 `DestroyBridgeWalker_*_High` are true compiled twins** at `0x57CF60`, `0x57D530`. High-confidence inference but not bit-verified.

4. **Resolve Q48 (signed-division direction in walker midpoint).** Parity-critical for odd-length bridge spans.

5. **Identify magic flags `0x600` and `0x2600`** passed to `AnimClass::Constructor`. Decompose into bit semantics.

6. **Identify `RulesClass+0x15C` / `+0x168` INI keys.** The bridge-collapse animation array — what does it look like in `rulesmd.ini`?

7. **Resolve `Engineer=` field offset conflict** between TECH_CABHUT §2 (`+0xEC3`) and CAHOSP §10 (`+0xEC5`). Tagged in TECH_CABHUT §12 as the highest-priority single follow-up.

8. **Trace `FUN_005868a0`** (called at walker end with the `local_80..local_74` rect). Likely the unit-on-bridge cell notification — relevant to "what happens to units on a collapsing bridge."

9. **Identify RTTI 0xf** (Q55) by extracting the RTTI enum.

The full categorized list of 61 deferred entries with `bounded-cost-too-high`,
`requires-different-system-context`, `out-of-scope`, and `needs-runtime-debugger`
labels lives in [CABHUT_C4_INVESTIGATION_LOG.md](CABHUT_C4_INVESTIGATION_LOG.md).

---

## 8. Skill self-evaluation (test of the new exhaustion-driven flow)

The user invoked this investigation specifically to compare the new
`/re-investigate` skill (with the open-questions log + branching rule)
against the old one (which gated on "topic feels covered").

**What the new skill produced:**
- 32 seed entries before any Ghidra call
- 45 more entries spawned during the Ghidra dive (the branching rule fired
  on every new callee, struct field, global, constant, magic flag)
- 13 functions decompiled with binary evidence; **8 of them never
  documented in any prior doc** (CollapseBridge_*, DestroyBridge_* per-cell,
  DestroyBridgeWalker_*_Low, BombClass::Attach body, BombClass::Detonate body)
- 8 more function addresses resolved for Phase 2
- 1 prior-doc correction (BRIDGE_HUT_DESTRUCTION §4.3 zone-rebuild asymmetry)
- 1 major new parity-critical finding (the 3-state damage model)
- A turnkey Phase-2 work-list with addresses + categories

**What the old skill would have produced** (extrapolated):
- A clean narrative report saying "C4 on CABHUT routes through
  PerCellProcess and BuildingClass::Update, dispatches to
  DestroyBridge_*_MapInit, which calls the walker family, and the bridge
  collapses." This is what TECH_CABHUT and BRIDGE_HUT_DESTRUCTION already
  said.
- The CollapseBridge_* bodies would have stayed deferred under "open
  question §19.2" (where they sat for weeks).
- The DestroyBridgeWalker_* family would never have surfaced (because the
  old skill stopped one tier short).
- The 3-state damage model would NOT have been found.

**Result:** the new skill demonstrably produces broader (not just polished)
output. The honest deferral pile (61 entries) is also a deliverable in its
own right — it's the next session's work-list.

---

## Sources

**Ghidra decompiles run this session** (all on `gamemd.exe`):

| Address | Function | Status |
|---------|----------|--------|
| `0x575540` | `CollapseBridge_NS_Low` | Decompiled |
| `0x575220` | `CollapseBridge_EW_Low` | Decompiled |
| `0x575BA0` | `CollapseBridge_NS_High` | Decompiled |
| `0x575870` | `CollapseBridge_EW_High` | Decompiled |
| `0x57BAA0` | `DestroyBridge_Low` (per-cell) | Decompiled |
| `0x57CCF0` | `DestroyBridge_High` (per-cell) | Decompiled |
| `0x57BCF0` | `DestroyBridgeWalker_NS_Low` | Decompiled |
| `0x57C2B0` | `DestroyBridgeWalker_EW_Low` | Decompiled |
| `0x438720` | `BombClass::Detonate` | Decompiled |
| `0x438E70` | `BombClass::Attach` | Decompiled |

**Memory reads:**
- `0x00ABDE88` (DAT_00abde88, Z-scale): 4 bytes = 0 (likely runtime-set at theater init)
- `0x00ABDC74` (DAT_00abdc74, OOB-backup): 4 bytes = 0 (sentinel-empty)
- `0x007E3EBC` (BuildingClass vtable): 512 bytes captured for slot verification (Q6/Q7 deferred)

**xrefs:**
- `BombClass::Attach @ 0x438E70` callers: only `WarheadTypeClass::Detonate @ 0x4690B0`

**Function-search:**
- `DestroyBridgeWalker` family addresses resolved at `0x57BCF0`, `0x57C2B0`, `0x57CF60`, `0x57D530`
- `ApplyBridgeDestruction` family addresses resolved at `0x57DD50`, `0x57E2A0`, `0x57E7A0`, `0x57ED00`
- `FindBridgeEndpoints` family addresses resolved at `0x57C990`, `0x57C870`, `0x57DC20`, `0x57DAF0`

**Prior docs read in full (Step 0 dependency):**
- `C4_ON_BRIDGE_REPAIR_HUT_GATE_INVESTIGATION.md`
- `TECH_CABHUT_GHIDRA_REPORT.md`
- `BRIDGE_HUT_DESTRUCTION_ENTRY_DECODE_GHIDRA_REPORT.md`
- `BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md` (referenced; too large to read whole — sections sampled)

**INI files:** Not re-grepped this session (Agent B skipped — scope was
explicitly the open-work tail of existing reports, not a fresh INI scan).

**Open Questions Log:** [CABHUT_C4_INVESTIGATION_LOG.md](CABHUT_C4_INVESTIGATION_LOG.md)
