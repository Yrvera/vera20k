# Core Service Profile — damage-helpers

**Slug:** `damage-helpers`
**Service:** Damage helpers (warhead/armor kernel + ReceiveDamage pipeline)
**Primary doc:** `docs/research/DAMAGE_HELPERS_ENGINE_SUBSTRATE_SERVICE_STUDY.md` (STUDY+DESIGN, Pass 2 bit-VERIFIED 2026-06-04)
**This profile:** edge/graph extract for the core-services map. Long content lives in the primary doc.
**Evidence base:** Ghidra-verified study (addresses cited inline). Two spot-checks this session: `get_function_callers 0x00489180` (exactly 3 callers) and `get_function_by_address 0x005f5390` (ObjectClass__ReceiveDamage).

> **2026-07-13 correction:** `disassemble_function(address="0x006fdd50",
> program="gamemd.exe")` proves ordinary `Damage <= 0` skips the country/per-unit
> and veterancy stages; positive damage groups `(house * unit) * integer Damage`
> before one conversion; Wave/special stores zero and rejoins the civilian-
> garrison → tank-bunker → open-topped containment chain. See
> `0x006fe328..0x006fe455` and the corrected primary damage-helper/gate reports.
> Warhead Verses also has a binding 0x80-byte ReadString/default/token/fault
> contract documented in the corrected D1/INI reports; it is not merely an
> unbounded eleven-value list.

---

## Purpose

The **damage-application math service** — the pure-ish function family that converts
`(raw weapon damage + warhead + target armor index + impact distance + attacker/defender
vet/country/per-unit state + immunity gates)` into a **final integer HP delta**, plus the
AoE target-collection distributor that fans one detonation out to many targets at their
individual distances. It does NOT own projectile flight, target acquisition, retaliation
scheduling, the HP-state machine itself, or warhead *side effects* (Temporal/EMP/MindControl
state machines) — it touches those only where the damage number depends on them.

The conceptual core that is purely this service: **`ApplyWarheadDamage` (the kernel, R1)**
+ **`HouseClass::GetArmorMultForType`** + **`Apply_area_damage`** (the AoE distributor) +
the vet-threshold predicates. The receiver methods (`TechnoClass::ReceiveDamage`,
`ObjectClass::ReceiveDamage`) are owned by techno-foot / abstract-object but are the primary
**callers** that drive this kernel and apply its output to HP.

---

## Owns

- **The armor-vs-warhead-vs-distance kernel** `ApplyWarheadDamage @ 0x00489180`. Sig
  `int __fastcall(int damage, WarheadType* wh, int armorIndex, int distance)`. Owns: the
  `ScenarioFlags & 0x20` no-damage early-out, the healing gate (`(7<armor)-1 & damage`,
  armor index ≥ 8 cannot be healed), the distance-falloff lerp, the Verses[armor] double
  multiply, the **three** `ftol` truncations (contract `ftol(ftol(lerp)*Verses)`), and the
  MaxDamage cap. (LIVE `decompile`/`disassemble 0x00489180`.)
- **The AoE distributor** `Apply_area_damage @ 0x00489280`. Owns: per-ring cell walk
  (ring-offset tables `DAT_00abd490`/`DAT_00abd492`, count table `CellSpreadTable @
  0x007ed3d0` = `[1,9,21,37,61,89,121,161,205,253,309,369]`), same-base-damage / per-target-
  distance fan-out, in-air aircraft distance-halving, eligibility test, self-skip, and the
  bridge/wall/overlay/tiberium destruction + rocking dispatch.
- **The defender country armor multiplier** `HouseClass::GetArmorMultForType @ 0x0050bd30` —
  type-switched float from the *defender's* HouseTypeClass; default `1.0` (`_DAT_007e2ac8`).
- **Vet-threshold predicates** `IsVeteran @ 0x0074ff90` (`1.0 ≤ v < 2.0`), `IsElite @
  0x00750010` (`v ≥ 2.0`).
- **Weapon-select / retaliation Verses gates** (logic at `SelectWeaponAgainst @ 0x006f3330`,
  retaliation `FUN_007087c0`): Verses[armor]==0 → weapon unusable; ≤0.01 → suppress auto-
  acquire.
- **Constants** it reads/owns the semantics of: `0x007e2224 = 256.0f` leptons/cell,
  `0x007e2ac8 = 1.0f` (vet threshold / default armor mult), `0x007e37b4 = 2.0f` elite
  threshold, `0x007e3808 = 0.01` percent-parse, `Math__ftol @ 0x007c5f00` truncate-toward-
  zero.
- **Dead data it explicitly does NOT own/read:** WarheadType `+0xF8` ProneDamage (parsed at
  `0x0075de31`, never read during damage in YR — exhaustive whole-image byte sweep, Pass 2).

It does **not** own HP storage, the Yellow/Red/Dead state machine, or the EstimatedHealth
field — those live in ObjectClass/TechnoClass (abstract-object / techno-foot); the kernel
only computes the delta they subtract.

---

## Key functions & globals (addresses)

| Symbol | Address | Role |
|---|---|---|
| `ApplyWarheadDamage` (WarheadTypeClass__GetDamage) | 0x00489180 | The kernel: falloff → Verses → MaxDamage cap, 3× ftol |
| `Apply_area_damage` | 0x00489280 | AoE target collection + per-target ReceiveDamage(vtable+0x16c) dispatch |
| `HouseClass::GetArmorMultForType` | 0x0050bd30 | Defender country armor multiplier (type-switched) |
| `IsVeteran` | 0x0074ff90 | `1.0 ≤ v < 2.0` |
| `IsElite` | 0x00750010 | `v ≥ 2.0` |
| `Math__ftol` | 0x007c5f00 | Truncate-toward-zero float→int (the rounding boundary) |
| `WarheadTypeClass::Detonate` | 0x004690b0 | Top of the damage chain; dispatches AoE + non-damage effects |
| `WarheadTypeClass__ReadINI` | 0x0075de31 (parse site) | Parses Verses[11] double, CellSpread, ProneDamage, immunity bools |
| **Callers (not owned, drive the kernel):** | | |
| `ObjectClass::ReceiveDamage` | 0x005f5390 | Core HP-deduct + overkill clamp + Yellow/Red/Dead classify (abstract-object) |
| `TechnoClass::ReceiveDamage` | 0x00701900 | Receiver pre-pipeline: country/vet armor divisor + ordered immunity gates (techno-foot) |
| `Fire_At` (TechnoClassFireAtSpawnsBullet) | 0x006fdd50 | Positive-only grouped country/per-unit FirePower, vet, then civilian-garrison/tank-bunker/open-topped containment; special zero rejoins containment |
| `FUN_006fdb80` | 0x006fdb80 | Pre-fire EstimatedHealth overkill estimator |

**Globals / fields the kernel + helpers read:**
- `g_ScenarioClass_Instance & 0x20` — no-damage scenario gate (kernel + AoE).
- `g_RulesClass_Instance + 0x16C8` MaxDamage (constructor fallback 1000; stock 10000); `+0x688` VeteranArmor (1.5, FDIV);
  `+0x670` VeteranCombat (1.1, FMUL); `+0xf40` Occupy/civilian-garrison multiplier; `+0x1708` ConditionRed (0.25);
  `+0x1700` ConditionYellow (smoke-particle gate only); `+0xff0/+0xfa8/+0xfac/+0x1740/
  +0x1734` bridge/chain/AreaFire warheads + spread (AoE).
- WarheadType `+0xA0` Verses[11] double, `+0x12C` PercentAtMax, `+0x124` CellSpread, `+0x144/
  +0x146` Wall, `+0x156` Poison, `+0x16D` Psychedelic, `+0x177` Radiation, `+0x178`
  PsychicDamage, `+0x179` AffectsAllies.
- HouseTypeClass (via `HouseType+0x34`) `+0x100/0x104/0x108/0x10c/0x110` Armor{Aircraft,
  Building,Infantry,Units,Flying}Mult.
- TechnoClass `+0x160` per-unit FirepowerMultiplier (attacker), `+0x158` per-unit
  ArmorMultiplier (defender); HouseClass `+0x188` country FirePower.

---

## Tick / render position

**Not a tick-spine owner — a callee invoked from the combat phase of `World::advance_tick`.**
In gamemd terms the kernel runs whenever a warhead detonates, which in the per-tick order is
the **turrets + combat** stage (and the **retaliation** stage that follows). Concretely:

- During combat, an attacker's `Fire_At @ 0x006fdd50` builds the bullet damage (attacker-side
  mults); the projectile flies; on impact `WarheadTypeClass::Detonate @ 0x004690b0` calls
  `Apply_area_damage @ 0x00489280`, which dispatches `ReceiveDamage` per target, which calls
  the kernel `0x00489180` and subtracts HP. This is synchronous within the combat/retaliation
  stage, not a separate scheduled phase.
- It is **also driven outside projectile combat** by 18 distinct AoE callers (superweapons:
  Nuke `0x004251f0`, LightningStorm `0x0053a300`, PsychicDominator `0x0053b080`,
  SuperClass::Launch `0x006cc390`; animations: AnimClass AI/Middle `0x00423ac0`/`0x00424ce0`,
  VoxelAnimClass `0x00749f30`; per-cell processing: InfantryClass `0x00519630`; terrain:
  TerrainClass `0x0071b920`; bombs, disk laser, fly-locomotor splash, etc.). These run in
  their respective tick phases (special-weapon processing, building/anim AI), all funnelling
  the same per-target dispatch.
- The pre-fire estimator `FUN_006fdb80` runs during target acquisition/firing decisions
  (before the shot), feeding `EstimatedHealth (+0x70)` so multiple shooters don't overkill.

No render-pass role. The service is pure sim math; its only render-adjacent neighbour
(rocking/shake in `Apply_area_damage`) is flagged as a distinct rocking subsystem, not HP.

---

## Depends-on (outgoing edges)

Each edge: target slug + via-symbol + evidence.

1. **random-scenario** (RandomClass + ScenarioClass)
   - via: `g_ScenarioClass_Instance & 0x20` (the global no-damage scenario flag).
   - evidence: LIVE `0x00489180` `MOV EAX,[0x00a8b230]; TEST [EAX],0x20` → return 0; same
     gate mirrored in `Apply_area_damage 0x00489280`. The kernel reads ScenarioClass state
     to decide whether any damage happens at all. (No RNG consumption inside the kernel
     itself; AoE bridge-destroy chance `Rules+0x1740` consumes RNG but that is the rocking/
     destruction path.)

2. **rules-class** (RulesClass)
   - via: `Rules+0x16C8` MaxDamage cap; `Rules+0x688` VeteranArmor (FDIV); `Rules+0x670`
     VeteranCombat (FMUL); `Rules+0xf40` Occupy/civilian-garrison multiplier; `Rules+0x1708` ConditionRed;
     `Rules+0xff0/+0xfa8/+0xfac/+0x1740/+0x1734` bridge/chain/AreaFire warheads + spread.
   - evidence: LIVE `disassemble 0x00489180` (`MOV ECX,[0x008871e0]; MOV ECX,[ECX+0x16c8]`
     cap); `disassemble 0x007019cb` (`FDIV double [g_Rules+0x688]`); `disassemble 0x006fe3d2`
     (`FMUL double [g_Rules+0x670]`); `decompile 0x005f5390` (ConditionRed). The damage
     numbers are tuned entirely by RulesClass globals.

3. **factory-house** (FactoryClass + HouseClass)
   - via: `HouseClass::GetArmorMultForType @ 0x0050bd30` (defender HouseTypeClass armor mult);
     attacker `HouseClass+0x188` country FirePower (folded in Fire_At); `IsAlliedWith @
     0x004f9a50` (AffectsAllies + Psychedelic ally gates); `HouseClass::RegisterDestruction`
     (death credit, ObjectClass D19); `HouseClass__Update_Threat_Score` (post-damage AI
     feedback `0x00701e0c`).
   - evidence: LIVE `decompile 0x0050bd30` (reads defender `HouseType+0x34`+armor offsets);
     `disassemble 0x006fe33d` (`FLD double [HouseClass+0x188]`); D11 gate 7 + D12 call
     `0x004f9a50`. The country firepower/armor multipliers and ally relationship come from
     HouseClass/HouseTypeClass.

4. **techno-foot** (TechnoClass + FootClass)
   - via: per-unit `TechnoClass+0x160` FirepowerMultiplier (attacker) / `+0x158`
     ArmorMultiplier (defender) folded into the FirePower/armor stages; veterancy abilities
     read off TechnoTypeClass (`type+0x29d/0x2af` ARMOR, `type+0x29e/0x2b0` FIREPOWER) gating
     the vet/elite divisor/mult; `IsWarpingOut @ 0x0070c5b0` (`+0x270`). The receiver methods
     `TechnoClass::ReceiveDamage @ 0x00701900` and `Fire_At @ 0x006fdd50` are themselves
     techno-foot vtable methods that wrap the kernel.
   - evidence: LIVE `disassemble 0x006fe343` (`FMUL double [ESI+0x160]`), `0x00701957`
     (`FMUL double [ESI+0x158]`); vet ability byte tests in `0x00701984`/`0x006fe3c8`.

5. **abstract-object** (AbstractClass / ObjectClass)
   - via: the kernel's primary normal-path caller `ObjectClass::ReceiveDamage @ 0x005f5390`
     owns HP storage, the overkill clamp, building min-1, healing+max clamp, and the Yellow/
     Red/Dead state classify + death-vtable dispatch (`vtable+0xE0/0xE4/0xDC`); the kernel
     reads the target's armor index via `vtable+0x88 → +0x9c` and Strength via `+0xa0`.
   - evidence: LIVE `decompile 0x005f5390` calls `FUN_00489180(*(armor+0x9c), warhead)`;
     `get_function_by_address 0x005f5390` → ObjectClass__ReceiveDamage (this session). This is
     a bidirectional relationship — ObjectClass is both a top caller and the HP-state owner the
     kernel's output flows into.

6. **cell-map** (CellClass / MapClass)
   - via: `Apply_area_damage` walks the cell grid by ring (offset tables
     `DAT_00abd490`/`DAT_00abd492`, count table `CellSpreadTable @ 0x007ed3d0`) to collect
     targets in radius; the receiver bunker/cell-match immunity gate (D11.3) tests the
     occupying cell/building; bridge/overlay/tiberium destruction edits cell contents.
   - evidence: study §P2.3 (ring tables + cell walk in `decompile 0x00489280`); D11.3 bunker
     gate `0x00701b67–0x00701bf6`. AoE target selection is a spatial query over the cell grid.

7. **ini-parsing** (CCINIClass / INIClass accessors)
   - via: `WarheadTypeClass__ReadINI` (`0x0075de31` and surrounding) populates Verses[11]
     (`atoi(str)*0.01` per `%`, `strtod` otherwise → double), CellSpread, PercentAtMax,
     ProneDamage (parsed-but-dead), and the immunity bool flags the kernel/gates read.
   - evidence: LIVE `decompile` ReadINI `*(double*)(ESI+0xf8)=ReadDouble()`, Verses double
     loop at `+0xA0`. All warhead damage parameters originate from INI parse.

8. **lookup-tables** (static read-only tables)
   - via: `CellSpreadTable @ 0x007ed3d0` (cells-per-radius), ring-offset tables
     `DAT_00abd490`/`DAT_00abd492`, the float constants `0x007e2224` (256), `0x007e2ac8`
     (1.0), `0x007e37b4` (2.0), `0x007e3808` (0.01).
   - evidence: study §2b, §P2.3. These are static substrate tables the kernel/AoE index into.

(Optional/weak) **drawing-helpers / frontier-objects** — `Apply_area_damage` also triggers
rocking/shake and destruction-anim spawns; flagged in the study as a *distinct rocking
subsystem*, not part of the HP damage number. Not counted as a damage-helpers dependency.

---

## Used-by (incoming edges)

Other services that call into / depend on this one:

1. **techno-foot** (TechnoClass + FootClass)
   - via: `TechnoClass::ReceiveDamage @ 0x00701900` calls the kernel `0x00489180` (Psychedelic
     branch second call site `0x00701d64`) and routes damage; `Fire_At @ 0x006fdd50` builds
     attacker-side damage that becomes the kernel input. Every unit/infantry/aircraft taking
     or dealing damage routes through this service.
   - evidence: `get_function_callers 0x00489180` includes `TechnoClass__ReceiveDamage`
     (this session).

2. **abstract-object** (AbstractClass / ObjectClass)
   - via: `ObjectClass::ReceiveDamage @ 0x005f5390` is the normal-path kernel caller; HP
     deduction depends on the kernel's returned delta.
   - evidence: `get_function_callers 0x00489180` includes `ObjectClass__ReceiveDamage`.

3. **target-scoring** (Target-scoring helpers)
   - via: `SelectWeaponAgainst @ 0x006f3330` and retaliation `FUN_007087c0` use the
     Verses[targetArmor] gate (0 → weapon unusable; ≤0.01 → suppress auto-acquire); the
     pre-fire estimator `FUN_006fdb80` calls the kernel at distance 0 to predict overkill and
     drive retargeting; `Update_Threat_Score` consumes the final clamped damage.
   - evidence: study R11/D23/D24; `get_function_callers 0x00489180` includes `FUN_006fdb80`.

4. **frontier-objects** (superweapons / animations / terrain — un-studied core services)
   - via: 18 distinct callers of `Apply_area_damage @ 0x00489280` — Nuke `0x004251f0`,
     LightningStorm `0x0053a300`, PsychicDominator `0x0053b080`, SuperClass::Launch
     `0x006cc390`, AnimClass AI/Middle `0x00423ac0`/`0x00424ce0`, VoxelAnimClass `0x00749f30`,
     InfantryClass PerCellProcess `0x00519630`, TerrainClass `0x0071b920`, BombClass
     `0x00438720`, etc.
   - evidence: study §P2.2 (`get_function_callers 0x00489280` → 18 systems). The AoE entry is
     shared substrate across superweapons, animations, terrain, and per-cell processing.

5. **logicclass** (LogicClass — indirect)
   - via: the combat/retaliation stage of the per-tick spine drives the attack→detonate→
     ReceiveDamage chain; the kernel is reached transitively each tick a weapon fires.
   - evidence: tick-order spine (combat phase) in `World::advance_tick`; the study frames this
     as master-TODO #5 (combat/projectile/warhead pipeline). Edge is structural, not a direct
     call from LogicClass into the kernel.

---

## Open / unverified edges

- **MaxDamage cap reachability (non-blocking):** the `Rules+0x16C8` cap (constructor fallback 1000; stock merged 10000) is
  per-target output; not proven that any stock-YR `Damage × Verses × falloff` single hit
  reaches it. Cap is kept regardless (cheap). Next query: enumerate per-weapon `Damage` ×
  max `Verses` in `rulesmd.ini` vs `[General] MaxDamage`. (Study §9 Remaining UNCHECKED.)
- **x87 intermediate parity:** an f64/fixed shortcut is not proven equivalent to
  the captured PC/RC-aware operations and explicit store boundaries. Sampled
  equal integers do not certify it; until exhaustive proof or the software-x87
  implementation lands, the shortcut is DRIFT. Boundary-spanning Oracle cases
  remain required.
- **Pre-fire estimate predicate parity:** do not reuse the removed
  gattling/deploy labels. Reconcile estimate predicates with the audited
  civilian-garrison/tank-bunker/open-topped identities before implementation.
- **D8 case-7 discriminator `param_2[0x382]==5` (byte +0xE08):** FlyLocomotor flying-vs-ground
  selector; switch + offsets verified, exact INI semantics of +0xE08 not pinned (not load-
  bearing for mult selection).
- **frontier-net edge:** damage outputs are state-hash-relevant (SNAPSHOT_VERSION bump
  required on the Rust side); the lockstep/replay consumer is structural and not a direct
  call edge — listed here as a reminder, not a verified function edge.
