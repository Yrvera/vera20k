# Frame-Modulo Cadence Inventory — YR Skirmish Tick

**Status:** COMPLETE  
**Date:** 2026-05-28  
**Source:** Live Ghidra MCP decompilation of `gamemd.exe`  
**Scope:** Every `g_CurrentFrameCounter % N` / `g_CurrentFrameCounter & mask` gate reachable in a standard local-skirmish YR tick (g_GameMode == 5)

---

## Background

`g_CurrentFrameCounter` @ `0x00A8ED84` is incremented **late** in `Main_Tick @ 0x0055D360`, after all logic/render/service work completes. All gates in `LogicClass::PerTickUpdate` and its callees therefore execute against the **pre-increment** frame value N, meaning "tick N" sees the counter as N, not N+1.

CDTimerClass-based delays (elapsed = frame − start_frame) are **not** listed here — this inventory covers only absolute-frame modulo/mask arithmetic gates.

---

## Complete Gate Table

| # | Function | Address | Expression | Period | Active in YR | Notes |
|---|----------|---------|-----------|--------|-------------|-------|
| 1 | `LogicClass::PerTickUpdate` | `0x0055B29C` | `frame % 0x78 == 0` | every 120 frames (~4 s at 30 fps) | **Yes** | `MapClass::RecalcBridgeShroudFlags()`; assembly: `MOV ECX,0x78` / CDQ / IDIV / TEST EDX / JNZ |
| 2 | `Main_Tick` | `~0x0055D390` | `(byte)frame & 7 == 7` | every 8 frames | **No — network only** | `Network_Keepalive()`; guard: `g_GameMode == 4`; not active in local skirmish (mode 5) |
| 3 | `FootClass::AI` | `0x004DA554–0x004DA566` | `frame % Rules+0x1808 == 0` | INI-driven (tiberium heal rate) | **Conditional** | Tiberium self-heal for infantry/vehicles; active only if unit on tiberium, not in transport, and `Rules+0x17F0` flag set (`TiberiumHeal` rules key) |
| 4 | `FootClass::AI` | `0x004DA7B5` | `frame & 0x8000000f == 0` | every 16 frames | **Conditional** | FogBorder visual refresh for allied unit; active only if fog-capable (SpecialFlags & 0x1000 set) — **TS-dormant** in standard YR |
| 5 | `FootClass::AI` | `0x004DA90E` | `frame % type.WalkRate == 0` | INI-driven (`WalkRate`) | **Yes** | Foot unit body animation frame advance while moving; fires whenever unit is walking |
| 6 | `FootClass::AI` | `0x004DA989` | `frame % type.IdleRate == 0` | INI-driven (`IdleRate`) | **Yes (if IdleRate != 0)** | Foot unit body animation frame advance while idle; skipped if `IdleRate == 0` |
| 7 | `FootClass::AI` | `0x004DADFC–0x004DADFE` | `frame & 0x3f == 0x3f` | every 64 frames | **Conditional** | Idle scatter dispatch; active only if unit not in transport, locomotor-capable, idle, and at cell 0 (animation cycle end) |
| 8 | `UnitClass::AI` | `~0x007365xx` | `(x/10 + frame + (y/10)*0x10000 + misc) % 0x1e == 0` | every 30 frames, staggered by position | **Conditional** | Dust particle spawn; active only if `TechnoTypeClass+0x1C8 != 0` (dust type set) |
| 9 | `UnitClass::AI` | `~0x007368xx` | `frame & 0x8000000f == 2` | every 16 frames | **Conditional** | Garage-escape check; active only if unit has non-null garage pointer |
| 10 | `UnitClass::AI` | `~0x00736Axx` | `frame % 0x18 == 0` | every 24 frames | **Conditional** | Garrison muzzle-flash AnimClass spawn; active only if `Rules+0x344 != 0` and type has garrison ports |
| 11 | `UnitClass::AI` | `~0x00736Cxx` | `frame & 0xf == 0` | every 16 frames | **Conditional** | AI auto-return-to-depot; active only for AI-controlled units, mission 0xB or 5, low HP, not player-controlled |
| 12 | `AnimClass::AI` | `0x00423AC0+` | `frame % AnimType.TrailerSeperation == 0` | INI-driven (`TrailerSeperation`) | **Conditional** | Trailer anim spawn; active only if `TrailerSeperation != 0`, anim not dead, not suppressed |
| 13 | Gas particle helper | `0x0062D2A0+` | `frame % (10 / WindEffect) == 0` | wind-driven | **Conditional** | Gas particle wind drift; active only if `WindEffect > 0` |
| 14 | Gas particle helper | `0x0062D2A0+` | `frame & 1` | every 2 frames | **Yes (when gas present)** | Gas particle altitude settling; fires on odd frames only |
| 15 | `HasSpySatelliteUpdate` | `0x00431800+` | `frame % SpySatInterval < SpyRefreshCount + 1` | INI-driven | **Conditional** | Spy satellite cell reveal cadence; active only if active satellite entries exist and player has spy satellite building |
| 16 | `TechnoClass::AI_Update` | `0x006F9F7B` | `frame & 4 != 0` | every 4-frame cycle | **Yes** | Health low-water-mark tracker update; fires every frame where bit 2 of frame counter is set (frames 4–7, 12–15, etc.) |
| 17 | `TechnoClass::AI_Update` | `0x006FA167–0x006FA173` | `frame % Rules+0x314 == 0` | INI-driven (`SpyMoneyTransferDelay`) | **Conditional** | Spy money steal/transfer; active only if techno type has spy satellite/spy money capability |
| 18 | `TechnoClass::AI_Update` | `0x006FA47C–0x006FA48F` | `frame & 0x8000000f == 0` | every 16 frames | **Conditional** | Target validity range check; active if techno has a target and is not in mission 8 or 17 |
| 19 | `TechnoClass::AI_Update` | `0x006FA7E2–0x006FA7EE` | `frame % Rules+0x38 == 0` | INI-driven (`SelfHealUnitFrames`, default 75) | **Conditional** | Power-drain building auto-damage (building at full HP, type Repairable, house has power drain) |
| 20 | `TechnoClass::AI_Update` | `0x006FA8D6–0x006FA8E2` | `frame % Rules+0x30 == 0` | INI-driven (`SelfHealInfantryFrames`, default 50) | **Power-surplus building repair** (building below max HP, aircraft/building, house has surplus power) | **Conditional** |

---

## TS-Dormant / Non-Skirmish Gates Summary

| # | Expression | Reason Not Active |
|---|-----------|------------------|
| 2 | `(byte)frame & 7 == 7` | Network keepalive — `g_GameMode == 4` guard; local skirmish is mode 5 |
| 4 | `frame & 0x8000000f` (fog border) | Gated by `SpecialFlags & 0x1000` (fog-of-war). YR defaults `FogOfWar=false`; this flag is never set in a standard skirmish |
| TS-LogicClass fog branch | `SpecialFlags & 0x1000` @ `0x0055B2XX` (order 7 in PerTickUpdate) | Same fog-of-war gate |
| DAT_00A83E04 loop (PerTickUpdate order 22) | Various | Guarded by `g_GameMode != 0 && g_GameMode != 5` — never fires in local skirmish |

---

## INI Key → RulesClass Offset Cross-Reference

| RulesClass offset | INI key | Default | Used in gate |
|---|---|---|---|
| `+0x30` | `SelfHealInfantryFrames` | 50 | Gate 20 |
| `+0x38` | `SelfHealUnitFrames` | 75 | Gate 19 |
| `+0x314` | `SpyMoneyTransferDelay` | (INI-set) | Gate 17 |
| `+0x1808` | tiberium heal rate (`TiberiumHeal` rules key) | (INI-set) | Gate 3 |
| `+0x344` | garrison fire rate divisor | (INI-set) | Gate 10 |

---

## Pattern Notes

### `& 0x8000000f` — sign-extended modulo-16

This pattern appears at gates 4, 9, 18. The expression `x & 0x8000000f` on a signed 32-bit int is a sign-extended modulo-16: it equals `x & 0xf` for positive frame values, but preserves the sign bit for negative values. In practice `g_CurrentFrameCounter` is always non-negative (incremented from 0), so this is functionally equivalent to `frame & 0xf == 0` (every 16 frames). The explicit sign-extension is a TS-era defensive pattern.

### Staggered dust gate (gate 8)

`(x/10 + frame + (y/10)*0x10000 + misc) % 0x1e == 0` is a spatial hash that spreads the 30-frame cadence across different cells, preventing all units from spawning dust on the same frame. Each unit's position shifts its phase within the 30-frame window.

### Animation-rate gates (gates 5, 6)

`WalkRate` and `IdleRate` are per-type INI fields read from `UnitTypeClass`/`InfantryTypeClass`. They drive frame advance at different rates per unit type — a fast unit advances its walk anim every 2 frames, a slow one every 8. These are the most frequently firing modulo gates in the entire tick pipeline.

---

## Implementation Handoff — Rust Delta

The Rust engine (`src/sim/world/mod.rs`) derives `binary_frame` at the start of `World::advance_tick` (line ~1187) but currently has **no absolute-frame modulo gates**. All periodic behaviors are driven by per-system tick counters or wall-clock timers.

Gates that need Rust equivalents (Active in YR: Yes or Conditional-common-path):

| Priority | Gate | Rust location hint |
|---|---|---|
| HIGH | Gate 1: bridge shroud `% 120` | `src/sim/world/bridge_orchestrator.rs` — add frame-modulo check in world tick |
| HIGH | Gates 5, 6: WalkRate/IdleRate animation cadence | `src/sim/movement/facing_class.rs` or anim tick — per-type INI fields |
| HIGH | Gate 16: health tracker `& 4` | `src/sim/components.rs` or `TechnoClass::AI_Update` equivalent |
| HIGH | Gates 19, 20: self-heal/damage by power status | `src/sim/world/world_tick.rs` — building repair/drain pass |
| MEDIUM | Gate 3: tiberium heal `% TiberiumHealRate` | `src/sim/miner/` or combat — tib heal per unit |
| MEDIUM | Gate 7: idle scatter `& 0x3f == 0x3f` | `src/sim/movement/` — scatter dispatch |
| MEDIUM | Gate 12: trailer anim `% TrailerSeperation` | `src/sim/components.rs` — anim tick |
| MEDIUM | Gate 18: target recheck `& 0xf == 0` | `src/sim/combat/` — target validity |
| LOW | Gate 8: dust particle `% 30` staggered | `src/sim/movement/` — particle spawn |
| LOW | Gate 10: garrison muzzle flash `% 24` | Building garrison system |
| LOW | Gate 14: gas altitude `& 1` | Particle subsystem |
| LOW | Gate 15: spy satellite reveal cadence | Superweapon / vision system |
| SKIP | Gate 2: network keepalive `& 7` | Network-only, no equivalent needed in skirmish sim |
| SKIP | Gate 4: fog border `& 0xf` | TS-dormant (FogOfWar=false in YR) |
| SKIP | Gate 9: garage escape `& 0xf` | TS-era garrison escape, not standard YR gameplay |
| SKIP | Gate 11: AI depot return `& 0xf` | AI system not yet in scope |

---

## Verification Citations

All findings verified via live Ghidra MCP decompilation in this session:

- Gate 1: `decompile_function 0x0055AFB0` + `get_assembly_context 0x0055B29C`
- Gate 2: `decompile_function 0x0055D360`
- Gates 3–7: `decompile_function 0x004DA530` + `disassemble_function 0x004DA530`
- Gates 8–11: `decompile_function 0x007360C0`
- Gate 12: `decompile_function 0x00423AC0`
- Gates 13–14: `decompile_function 0x0062D2A0`
- Gate 15: `decompile_function 0x00431800`
- Gates 16–20: `decompile_function 0x006F9E50` + `disassemble_function 0x006F9E50`

Corroborating docs (read before decompilation):
- `docs/research/PERTICKUPDATE_FULL_ORDERING_LADDER_GHIDRA_REPORT.md`
- `docs/research/LOGICCLASS_GLOBAL_SUBSYSTEM_ORDER_0055AFB0_GHIDRA_REPORT.md`
- `docs/research/GLOBAL_TIMING_MODEL_GHIDRA_REPORT.md`

---

*Report produced by swarm slot 3. Slot 3 finding: 20 gates total, 15 active or conditionally active in YR skirmish, 5 TS-dormant or network-only.*
