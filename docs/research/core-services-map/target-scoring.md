# Core Service Profile — Target-scoring helpers (threat score + target acquisition)

**Slug:** `target-scoring`
**Primary doc:** `docs/research/TARGET_SCORING_ENGINE_SUBSTRATE_SERVICE_STUDY.md` (STUDY+DESIGN, Ghidra-verified 2026-06-04, Pass 2 closed).
**Edges re-confirmed live this session** via `get_function_callers` / `get_function_callees` on the family addresses (see Depends-on / Used-by evidence inline).

---

## Purpose

The per-unit "what do I shoot at" decision. Given an attacker (unit / infantry / aircraft / garrisoned building) it scans a region of the map, gates each candidate for legality, computes an **integer threat score** per candidate, and returns the single best target pointer (or null) by **strictly-greater** selection with scan-order tie-break. This is the target-acquisition substrate that TechnoClass combat sits on: idle/guard/area-guard/attack-move/opportunity-fire acquisition, garrison-building fire, the retaliation target-switch decision, and (deferred) AI house base-target planning.

Three-stage contract:
1. **Calculate_Threat_Score** (`0x0070CD10`) — pure score terms (effectiveness, special-threat, enemy-house bonus, strength, beyond-range distance penalty), base const 100000.0, returns float10.
2. **Evaluate_Candidate** (`0x006F7CA0`) — per-candidate legality gate pipeline + calls the score + post-score integer modifiers (PreferWounded, force-to-1, ThreatAvoidance) + final accept clamp.
3. **Greatest_Threat** (`0x006F8DF0`) — scanner: derive radius, dual topology (flat array vs ring-expanding squares with quarter/half early return), strictly-greater selection, scan-order tie-break.

## Owns

State, globals, and tables this service is the authority for:

- **Selection invariants:** best-score init = −1 (`local_50=0xffffffff`), best-ptr init = null, replace only on `new > best`, equal keeps earlier scan-order candidate. Scan-order tie source = ring-perimeter walk × intra-cell `cell+0xE8 → obj+0x30` linked list (tail-most passer) × array-index — NOT stable id.
- **Score constants/coefficients (read from RulesClass / TechnoTypeClass, but the score math is owned here):** base const `DAT_007F4E90 = 100000.0`; FIVE coeffs per branch A/B/C/D/E at Rules `+0x1068/+0x1070/+0x1078/+0x1080/+0x1088` (non-human "Dumb" set) or Type `+0x2C8/+0x2D0/+0x2D8/+0x2E0/+0x2E8` (per-type); `EnemyHouseThreatBonus` Rules+0x1090; early-return const `DAT_007E2800 = 0.0`.
- **ThreatAvoidance multiplier:** `ThreatAvoidance_Modifier @ 0x006F79A0`, per-allied-building factor `DAT_007E1738 = 0.5`, radius `Rules+0x1430 ThreatAvoidanceRadius`, gated by attacker building `+0x146`.
- **Per-unit cadence/scan state (sim/hash-relevant):** `+0x4FC` last passive-scan frame stamp, `+0x50C` TarCom-changed flag, `+0x688` (FootClass) ConvoyDisbanded one-shot scan flag.
- **Threat flag bit semantics** folded into the scan flags (bit0 weapon-range, bit1 guard-range, bit2 neutral, bit3 air-priority, bit4 allies, bit8 house-only(AI), bit14 enemy-house-only).
- **Profiling artifact (does NOT own as gameplay):** `DAT_00A8EC34` Greatest_Threat call counter — incremented at entry, read nowhere else; non-hash, do not model.

## Key functions & globals (addresses)

Functions:
- `TechnoClass::Greatest_Threat` `0x006F8DF0` — vtable+0x3C4; the scanner. Body 0x006F8DF0–0x006F9DAE.
- `TechnoClass::Evaluate_Candidate` `0x006F7CA0` — gate pipeline + score call + post-score modifiers. Body 0x006F7CA0–0x006F895A.
- `TechnoClass::Calculate_Threat_Score` `0x0070CD10` — pure score terms. Body 0x0070CD10–0x0070D0CF.
- `TechnoClass::Scan_Cell_For_Target` `0x006F8960` — one cell's occupant list walk, best-of-cell via Evaluate_Candidate, zone filter.
- `TechnoClass::ThreatAvoidance_Modifier` `0x006F79A0` — allied-building avoidance score multiplier (sole caller = Evaluate_Candidate).
- `TechnoClass::ShouldRetaliate` `0x007087C0` — retaliation target-switch gate; second direct score consumer; called by ReceiveDamage.
- `TechnoClass::Cell_Threat_Fallback` `0x006F8C10` — cell-level fallback when no object candidate found (AI/guard steering).
- `FootClass::Greatest_Threat` (wrapper) `0x004D9920` — vtable+0x3C4 override; ConvoyDisbanded one-shot forces weapon-range mode.
- `UnitClass scanner wrapper` `0x00743190` — UnitClass vtable+0x3C4; OR selected-weapon target-mask bits, then call 0x004D9920.
- `BuildingClass scanner/mask wrapper` `0x00445F00` — BuildingClass vtable+0x3C4; OR both weapons' mask bits, force weapon-range, call Greatest_Threat directly (verified via get_xrefs_to → DATA 0x007E4280 = BuildingClass vtable+0x3C4).
- `UnitClass passive driver` `0x00709820` — UnitClass vtable+0x39C; schedule next passive-scan timer, clear stale TarCom, call +0x3C4, set TarCom via +0x3C8.
- Helpers reached: `Sqrt_Approx 0x004CAC40`, `Math__ftol 0x007C5F00`, `ObjectClass__GetHealthRatio 0x005F5C60`.

Globals/data:
- `DAT_007F4E90` 0x007F4E90 = 100000.0 (score base).
- `DAT_007E1738` 0x007E1738 = 0.5 (avoidance per-building).
- `DAT_007E2800` 0x007E2800 = 0.0 (no-weapon early return).
- `g_RulesClass_Instance` ptr 0x008871E0 (coeff/bonus/threshold source).
- `g_TechnoClass_Array` / `g_AircraftClass_Array` (flat-scan sources).
- `g_NullCoord_Chrono_X/Y/Z` (the "use my-coord → distance in cells" sentinel).
- `DAT_00A8EC34` 0x00A8EC34 (profiling call counter; non-hash).

## Tick / render position

Not a fixed tick-spine phase of its own — it is a **callee invoked from the combat/turret and movement phases** of `LogicClass::PerTickUpdate` (project tick order: "turrets + combat" and "retaliation + passengers"). Two entry rhythms:
- **Passive acquisition:** UnitClass AI update (`0x00709820`, vtable+0x39C) runs on a per-unit cadence timer (`+0x4FC`) during object AI; when a unit has no TarCom it calls the +0x3C4 scanner chain. This is the "ground/air movement → object AI" region of the tick, before firing.
- **Retaliation:** `ShouldRetaliate 0x007087C0` is called from `TechnoClass::ReceiveDamage 0x00701900` (damage application), i.e. in the combat / damage-resolution part of the tick — it compares current-target vs attacker scores to decide whether to switch.
- **Garrison/order-driven:** invoked from attack-move/guard mission handling and garrisoned-building fire.

No render-pass involvement — pure sim. Layering invariant holds (no render/ui/audio/net dependency).

## Depends-on (outgoing edges)

Each edge = target service slug + the specific symbol that creates it + evidence (live `get_function_callees` this session unless noted).

- **rules-class** — via `g_RulesClass_Instance` coefficient/bonus/threshold reads (Rules `+0x1068..+0x1090` coeffs+EnemyHouseThreatBonus, `+0xF48` OccupyWeaponRange, `+0x1430` ThreatAvoidanceRadius, `+0x16F8` ConditionYellow, `+0x1708` ConditionRed) inside Calculate_Threat_Score / Greatest_Threat / Scan_Cell_For_Target / ThreatAvoidance_Modifier. Evidence: doc §2b/§2c/§10.3 (disassemble 0x0070CD10; decompile 0x0066BBB0 for the OccupyWeaponRange key).
- **cell-map** — via `MapClass__Get_CellClass_At_Coord 0x00565730`, `MapClass__Get_CellClass 0x005657A0`, `MapClass__GetZoneID 0x0056D230`, `Cell_in_bounds_check 0x00568300`, `CellClass__SensorCountForHouse 0x004870D0` (cloak-needs-sensor gate), `Look_up_building_in_cell 0x0047C520`, and the `cell+0xE8/+0xE4 → obj+0x30` occupant-list walk in Scan_Cell_For_Target. Evidence: live callees of 0x006F8DF0, 0x006F8960, 0x006F7CA0.
- **factory-house (HouseClass)** — via `HouseClass__Is_Ally_ByObject 0x004F9A90`, `HouseClass__IsAlliedWith 0x004F9A50`, `HouseClass__IsPlayerControl 0x0050B730`, `HouseClass__IsHumanPlayer 0x0050B6F0` (alliance prefilter, "Dumb"-vs-type coeff switch on `Owner+0x1FB`, enemy_only `Owner+0x5600`, force-to-1 `Owner+0x1580`). Evidence: live callees of 0x006F8DF0 and 0x006F7CA0.
- **damage-helpers** — via Warhead `Verses[armor]` table reads (Warhead+0xA0 + armor*8) in Calculate_Threat_Score for both effectiveness terms (A·myWarhead.Verses[candArmor], ±B·candWarhead.Verses[myArmor]); same Verses kernel as ReceiveDamage. Evidence: doc §2b/§2e (disassemble 0x0070CD10 FMULs); the armor-vs-warhead lookup is the damage kernel's table.
- **techno-foot (TechnoClass/FootClass)** — via `TechnoClass__GetWeaponRange 0x006F3970` (scan-radius derivation), weapon selection `SelectWeaponAgainst`/`Get_Weapon` (vtable+0x2E4/+0x3F8), `Get_Coord` (vtable+0x48), `Can_Fire_At` (vtable+0x3A8), `GetFireError` (vtable+0x3BC), and the FootClass/UnitClass wrapper overrides that fold weapon target-mask bits before scanning. Evidence: live callees of 0x006F8DF0 and 0x006F8960; doc §2f vtable table.
- **abstract-object (ObjectClass)** — via `ObjectClass__GetHealthRatio 0x005F5C60` (strength term D), RTTI discriminator (vtable+0x2C), `Class_Of` (vtable+0x84), candidate weapon-presence flag (vtable+0x88), InLimbo `+0x81`, health `+0x6C`, discovery bytes `+0x41A/+0x41B`, target `+0x2B4`. Evidence: live callees of 0x0070CD10 / 0x006F8960; doc §2f.
- **mission-radio (MissionClass)** — via `MissionClass__GetMissionTimerEntry 0x005B3A00` (read in Evaluate_Candidate; the `+0x3D5` / mission-timer reject gate) and the GUARD(5) degenerate-radius branch keyed on mission. Evidence: live callees of 0x006F7CA0; doc §10.5.
- **random-scenario (RandomClass/ScenarioClass)** — via `Random__RandomRanged 0x0065C7E0` in Evaluate_Candidate, and the `g_ScenarioClass+0x800` "no-target list" gate (rejects matching types when scenario flag 0x800 set). Evidence: live callees of 0x006F7CA0; doc §10.5. (RNG-instance routing per-callsite ECX — not re-resolved here.)
- **lookup-tables / helpers** — via `Math__ftol 0x007C5F00` (the single float→int score truncate, truncate-toward-zero) and `Sqrt_Approx 0x004CAC40` (distance). Evidence: live callees of 0x0070CD10 and 0x006F8DF0.
- **bridge-helpers** — via the bridge-layer reject gate in Evaluate_Candidate: reject only when BOTH attacker and target cells have the structural-bridge bit (`cell+0x140 & 0x100`) and the two `OnBridge` (`+0x8C`) flags differ. Evidence: doc C16 / GRIZZLY_CLOAK_BRIDGE §5 (DOC-SOURCED; predicate uses cell bridge bit + OnBridge flag).
- **ini-parsing** — INDIRECT only: the coefficients/SpecialThreatValue/EnemyHouseThreatBonus/OccupyWeaponRange/GuardRange/PreferWounded are parsed by CCINIClass at load and stored on RulesClass/TechnoTypeClass; this service reads the parsed fields, it does not call INI accessors at runtime. Listed for completeness; not a runtime edge.

## Used-by (incoming edges)

- **techno-foot (TechnoClass/FootClass/UnitClass)** — the scanner is invoked through the vtable+0x3C4 chain: UnitClass passive driver `0x00709820` (vtable+0x39C) → UnitClass mask wrapper `0x00743190` (vtable+0x3C4) → FootClass wrapper `0x004D9920` (vtable+0x3C4 override) → TechnoClass::Greatest_Threat `0x006F8DF0`. Evidence: live `get_function_callers 0x006F8DF0` = {0x00445F00, 0x004D9920}; `get_function_callers 0x004D9920` = {0x00743190}. This is the primary consumer (per-unit acquisition / OpportunityFire / guard / attack-move).
- **factory-house / building combat (BuildingClass)** — `0x00445F00` (BuildingClass vtable+0x3C4) calls Greatest_Threat directly for garrisoned-building fire. Evidence: live `get_function_callers 0x006F8DF0`; doc §10.4 (vtable class resolved to BuildingClass).
- **damage-helpers** — `TechnoClass::ReceiveDamage 0x00701900` calls `ShouldRetaliate 0x007087C0`, which calls Calculate_Threat_Score twice (current target vs attacker) to decide retaliation target-switching. Evidence: live `get_function_callers 0x007087C0` = {TechnoClass__ReceiveDamage @ 0x00701900}; `get_function_callers 0x0070CD10` = {Evaluate_Candidate, ShouldRetaliate}.
- **frontier-ai (AI house planning)** — DEFERRED-AI: the house-only quick scan (flag bit 8 / `param_2 & 0x100`, returns a house) and Cell_Threat_Fallback as steering output are consumed by AI base-target planning. Out of current scope; clean seam left. Evidence: doc §3 DEFERRED-AI, §10.5.

## Open / unverified edges

- **Equal-score replay parity (cell-occupant insertion order):** UNCHECKED — the Rust cell-occupant ordering must equal gamemd's `cell+0xE8` reveal/unlimbo insertion chain for ties to replay-match. Depends on master-TODO #1 (native live-object vector) landing first. Next query: trace cell-occupant insertion in ObjectClass::Mark/unlimbo and diff vs the Rust EntityStore cell index. (Non-blocking for shadow work.)
- **`+0x3D5` reject-gate semantics:** gate existence VERIFIED (read in Evaluate_Candidate), but its "underground probabilistic detection" meaning is UNPROVEN and disputed across sibling docs — treat as a generic map-state reject, NOT TS underground RNG.
- **bridge-helpers edge:** the bridge-layer predicate (cell+0x140 bit + OnBridge) is DOC-SOURCED (GRIZZLY_CLOAK_BRIDGE), not re-decompiled live this session; the exact in-function offsets in Evaluate_Candidate were not re-walked here.
- **mission-radio edge precision:** `GetMissionTimerEntry` is a confirmed callee, but whether the `+0x3D5` gate is truly a MissionClass timer read vs an ObjectClass field read is the same unproven-semantics item above.
- **RNG instance binding for `Random__RandomRanged 0x0065C7E0`:** which RNG stream (Scen->Random vs g_MainRng) is per-callsite ECX and not resolved here; relevant only if this RNG draw is hash-relevant (low — appears to be a tie/scatter draw in candidate eval).
