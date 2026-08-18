# Phase F Bridge-Damage Dispatch — Verification

**Status:** verified live in Ghidra (gamemd.exe) on 2026-05-07.
**Scope:** narrow — corrects and extends `HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md` §4 (call chain) and §3.1 (state-machine entry) for the purposes of scoping Tier 2 Phase F.
**Why this exists:** the existing HIGH report's call-chain summary is approximate.
The dispatch architecture in `Apply_area_damage` and `ApplyDamageToCell` is materially
different from what the summary suggests, and that difference changes Phase F scope.

---

## 1. Verified dispatch in `Apply_area_damage @ 0x00489280`

After the AoE entity-damage loop, gated by:

```
(*g_ScenarioClass_Instance & 0x8000) != 0      // SpecialFlags::DestroyableBridges
  && warhead+0x144 != 0                        // warhead.Wall
```

the binary runs **four independent paths sequentially** on the impact cell. Each
path has its own `RandomRanged(1, BridgeStrength) < damage` gate, with
`warhead == Rules+0xFF0` (IonCannonWarhead) bypassing the gate.

| # | Source label | Path | Match condition | Retry on fail |
|---|---|---|---|---|
| 1 | LAB_00489f27 → LAB_00489f77 | **HIGH state-machine** via `ApplyDamageToCell` | `flags & 0x100` set AND anchor.+0x44 ∈ {0x18, 0x19} OR tile-class index `(IsoTileType - DAT_00aa0e28 + 1) ∈ DAT_00abad30..+3 / DAT_00aa1028..+3` | **3 retries** if IonCannon |
| 2 | LAB_0048a0a5 | **LOW state-machine** via `ApplyDamageToCell` | similar shape: anchor.+0x44 ∈ {0xED, 0xEE} OR tile-class `(IsoTileType - DAT_00abad1c + 1)` matches DAT_00abad30..+3 / DAT_00aa1028..+3 | **3 retries** if IonCannon |
| 3 | LAB_0048a214 (first half) | **LOW direct-overlay** via `DestroyBridge_Low` | `0x49 < cell.OverlayIndex < 100` (i.e., 0x4A..0x63) | **single shot** |
| 4 | LAB_0048a214 (second half) | **HIGH direct-overlay** via `DestroyBridge_High @ 0x0057CCF0` | `0xCC < cell.OverlayIndex < 0xE7` (i.e., 0xCD..0xE6) | **single shot** |

`Apply_area_damage` returns after path 4 (and the post-damage tail that handles
TiberiumWeed-style overlay destruction + particle FX).

---

## 2. Four surprising findings vs the HIGH report's call-chain summary

### Finding 1 — Direct-overlay paths have NO IonCannon retry loop

The HIGH report's §4 lists the IonCannon retry under `Apply_area_damage` outer
gate. Verified live: only paths 1 and 2 (state-machine via `ApplyDamageToCell`)
loop on `ApplyDamageToCell` returning false. Paths 3 and 4 are single-attempt:

```c
// HIGH state-machine retry (path 1) — bVar21 = (warhead != IonCannon)
cVar7 = ApplyDamageToCell();
iVar19 = 3;
while (cVar7 == 0) {
  if (bVar21 || iVar19 < 1) goto LAB_0048a049;   // exit on non-IonCannon
  cVar7 = ApplyDamageToCell();
  iVar19 = iVar19 - 1;
}

// HIGH direct-overlay (path 4) — single attempt
if (... cell.OverlayIndex in [0xCD..0xE6] ... && DestroyBridge_High() != 0) {
  TechnoClass__StopAllTargeting();
}
```

### Finding 2 — Z-height range gate is state-machine-only

Both state-machine blocks contain a Z-range check that fires only when
`flags & 0x100` is set:

```c
if (this->Flags & 0x100U) {
  if ((this->Level + 1) * DAT_0089e870 + DAT_0089e864 < impact.z
      || impact.z <= (this->Level - 2) * DAT_0089e870 + DAT_0089e864)
    goto next_block;       // skip state-machine, fall through
}
```

So the explosion's Z must be within `[level − 2, level + 1]` (in tile-step
units, around the deck) to engage the state machine on a structural body cell.
Direct-overlay paths (3 and 4) skip this entirely — a ground-level explosion
on a body-overlay cell still triggers the walker.

### Finding 3 — `ApplyDamageToCell` dispatches overlay-direct *first*

`ApplyDamageToCell @ 0x00587180` does NOT route to the state machine first
when the cell still carries a raw body overlay. Verified order inside
`ApplyDamageToCell`:

```c
iVar1 = cell.OverlayIndex;                     // CellClass+0x44
if (0x49 < iVar1 && iVar1 < 100) {             // 0x4A..0x63 — low body
  return DestroyBridge_Low(coord);             // overlay-direct LOW
}
if (0xCC < iVar1 && iVar1 < 0xE7) {            // 0xCD..0xE6 — high body
  return DestroyBridge_High(coord);            // overlay-direct HIGH
}
// only now: tile-class checks → ProcessBridgeDamageStateMachine_*
```

So when path 1 of `Apply_area_damage` reaches `ApplyDamageToCell` for a body
cell whose overlay is still in `[0xCD..0xE6]`, the call resolves to
`DestroyBridge_High` (the walker), NOT `ProcessBridgeDamageStateMachine_High`.
The state machine inside `ApplyDamageToCell` only executes when the overlay
byte is OUT of those ranges — i.e., already transitioned via prior
`UpdateRamp_*` writes.

### Finding 4 — Direct-overlay path is reachable in normal play

The decorative perpendicular cells of a high bridge (shadow rows, deck-only
siblings outside the anchor span's walked group) carry overlay in
`[0xCD..0xE6]` but have NO `flags & 0x100`. They fail every state-machine
match condition. They DO match path 4 (HIGH direct-overlay) → instant
full-bridge collapse via `DestroyBridge_High` walker.

This means a single shell on a decorative bridge cell triggers full collapse
in gamemd.exe. No retry needed — just the BridgeStrength RNG gate.

---

## 3. Implications for the shipped Phase C body / bridgehead drivers

`BridgeRuntimeState::body_cell_advance_state` and `bridgehead_advance_state`
mirror `ProcessBridgeDamageStateMachine_High` (`0x00576BA0`). Per Finding 3,
that function is the **late-stage progression** for cells whose `+0x44`
overlay byte has already been transitioned out of the body-overlay range.
It is NOT the primary damage path on a freshly-placed bridge.

The primary damage path on a freshly-placed bridge (overlay still raw in
`[0xCD..0xE6]`) is `DestroyBridge_High @ 0x0057CCF0` (overlay-direct walker)
— a separate driver currently unbuilt in the Rust runtime.

Phase F therefore must wire BOTH:
- the overlay-direct walker (`DestroyBridge_*` → `DestroyBridgeWalker_*_High`)
  for hits on raw-overlay cells (the common case on fresh bridges and on
  decorative siblings)
- the state-machine drivers (already shipped) for hits on cells whose overlay
  has been transitioned by prior damage / `UpdateRamp_*` perpendicular writes

The state-machine drivers' overlay-write branch (Tasks 13.5 / 15.5, deferred)
is the bridge between these two paths: it transitions the overlay byte out of
the body range, which then enables subsequent hits to land on the
`ProcessBridgeDamageStateMachine_*` branch via the post-overlay `flags & 0x100`
+ anchor lookup.

---

## 4. Addresses

| Address | Symbol | Role |
|---|---|---|
| `0x00489280` | `Apply_area_damage` | 4-path outer dispatcher + RNG gates + retry loop |
| `0x00587180` | `ApplyDamageToCell` | inner dispatcher (overlay-direct first, state-machine second) |
| `0x0057CCF0` | `DestroyBridge_High` | overlay-direct HIGH walker entry; routes to NS/EW walker |
| `0x0057CF60` | `DestroyBridgeWalker_NS_High` | NS axis-walker (per HIGH §4 / report) |
| `0x0057D530` | `DestroyBridgeWalker_EW_High` | EW axis-walker |
| `0x0057E7A0` | `ApplyBridgeDestruction_NS_High` | per-cell scatter / unit damage during walker |
| `0x0057ED00` | `ApplyBridgeDestruction_EW_High` | EW counterpart |
| `0x00576BA0` | `ProcessBridgeDamageStateMachine_High` | late-stage progression (mirrored by shipped Rust drivers) |
| `0x0047DD70` | `CellClass::BlowUpBridge` | per-cell ground kill + DropIn + debris (HIGH §11.4) |

Globals referenced:
- `g_ScenarioClass_Instance.SpecialFlags & 0x8000` — `DestroyableBridges`
- `Rules+0x144` (warhead `Wall=`) — outer gate
- `Rules+0x1740` — `BridgeStrength`
- `Rules+0xFF0` — `IonCannonWarhead`
- `Rules+0xFA8` — `C4Warhead`
- `DAT_0089e864`, `DAT_0089e870` — Z-step constants for level → world-z conversion
- `DAT_00aa0e28`, `DAT_00abad30`, `DAT_00aa1028`, `DAT_00abad1c` — runtime-init
  bridgehead tile-class base / offset tables (deferred per Task 13.5 / 15.5)

---

## 5. Sources

- `Apply_area_damage @ 0x00489280` — full decompile, 2026-05-07
- `ApplyDamageToCell @ 0x00587180` — full decompile, 2026-05-07
- `DestroyBridge_High @ 0x0057CCF0` — full decompile, 2026-05-07
- `HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md` §3.1, §4, §11.2, §11.4 (cross-referenced)
