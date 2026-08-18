# AnimClass Building/Object Damage Runtime Spawns - Ghidra Research Report

**Address(es):** `0x00421EA0`, `0x0043C0D0`, `0x0043B5E0`, `0x004415F0`, `0x00442230`, `0x0043FB20`, `0x005F5390`, `0x0071C5B0`, `0x0071B920`  
**Investigation Mode:** coverage-map  
**Claimed Scope:** Active runtime `AnimClass::Constructor` spawn paths for building damage-fire, building/object damage/destruction side effects, and terrain fire/death only.  
**Non-Scope:** Garrison `OccupantAnim`, generic 21-slot building anims, global AnimClass lifetime except call-order implications, weapon impact/muzzle flash, chrono swarm details, and full `AnimClass::AI` damage ownership.  
**Confidence:** High for verified caller mechanisms; Medium for vtable-only liveness of `BuildingClass__DestructionEffects`; Partial for ObjectClass RTTI-15 special death flag naming.  
**Active in YR:** Yes / Conditional, itemized below.

Working notes required by swarm prompt:

- Target question: Which building/object/terrain damage or fire paths allocate `AnimClass` at runtime, with active-YR conditions and constructor arguments?
- Non-goals: Do not re-open garrison `OccupantAnim`, 21-slot building state, or global constructor lifetime unless a spawn path depends on it.
- Evidence needed to mark COMPLETE: Live Ghidra decompile plus caller/callee evidence for each candidate, INI/default evidence for gates, Rust touchpoints, and at least one Rust-facing handoff.
- Stop conditions: Stop after classifying candidate functions and writing a narrow report; defer full `AnimClass::AI` fire propagation and exact ObjectClass RTTI-15 flag naming if not needed for the spawn map.

## 1. Overview

The scoped candidates resolve into five active or conditional spawn families: persistent building damage-fire slots, building damage/death debris and destruction anims, terrain fire and TIBTRE death anims, a conditional ObjectClass special-death anim, and `BuildingClass::CreateFireAnim` used by `SuperClass::Launch` rather than normal damage. Current Rust has app-layer building fire overlays but no generic `AnimClass` entity for these runtime spawns.

## 2. Class Layout / Key Offsets

| Offset | Owner | Meaning | Evidence | Active in YR |
|---:|---|---|---|---|
| `+0x5C8..+0x5E4` | `BuildingClass` | 8 persistent damage-fire `AnimClass*` slots | `0x0043C0D0`, `0x0043FB20`, `0x004415F0` | Yes, threshold-gated |
| `+0x5E8` | `BuildingClass` | cached damage-fire active byte | `0x0043FB20` | Yes |
| `+0x15D8..+0x1614` | `BuildingTypeClass` | `DamageFireOffset0..7`, stride 8 | `0x0043C0D0`, art INI parser | Yes |
| `+0x157B` | `BuildingTypeClass` | damage-fire threshold selector | `0x0043FB20` | Yes, label uncertain |
| `+0x2A4/+0x2B0` | `RulesClass` | `DamageFireTypes` vector/count | `0x0043C0D0`, `rulesmd.ini:519` | Yes |
| `+0x344` | `RulesClass` | `ChronoSparkle1` AnimType | `0x0043FB20`, `rulesmd.ini:554` | Conditional, chrono/warp |
| `+0xB78` | `RulesClass` | building damage debris AnimType triplet | `0x00442230` | Conditional, warhead Sparky |
| `+0xB94` | `RulesClass` | terrain tree-fire AnimType vector | `0x0071C5B0` | Conditional, caller unresolved |
| `+0xCC/+0xCD` | `TerrainClass` | on-fire / one-shot death flags | `0x0071C5B0`, `0x0071B920` | Conditional |

## 3. Core Logic

### 3.1 Building Persistent Damage Fire

Active in YR: Yes, conditional. `BuildingClass::Update @ 0x0043FB20` computes a damaged flag. If `Type+0x157B == 0`, it uses `Rules+0x1700` (`ConditionYellow`, `50%` in `rulesmd.ini:753`); otherwise it uses `Rules+0x1708` (`ConditionRed`, `25%` in `rulesmd.ini:752`). When `BuildingClass+0x5E8` changes false-to-true, it calls `BuildingClass::CreateDamageFireAnims @ 0x0043C0D0`; when true-to-false, it loops the 8 fire slots, calls vtable `+0xF8` on non-null anims, and clears pointers.

`CreateDamageFireAnims` is all-at-once, not progressive. It reads `Rules+0x2B0`; if zero, it returns without RNG. Otherwise it consumes one `RandomRanged(0,count-1)` for the starting fire type, then scans slots 0..7. It returns on the first sentinel offset pair or occupied slot. Each successful slot converts `DamageFireOffsetN` through `IsometricPixelToWorld @ 0x006D2070`, adds building render coords from vtable `+0xAC`, allocates `0x1C8`, and calls `AnimClass::Constructor(type, coord, delay=0, loop=1, flags=0x600, facing=0, z=0)`.

After construction it stores the pointer at `+0x5C8+slot*4`, computes `zAdjust = (((offsetY + (foundationHeight + foundationWidth) * -15) * 3) >> 1) - 10`, clamps positive values to 0, reads `AnimType+0x2C0` frame count, optionally consumes `RandomRanged(0, frame_count-1)` into `AnimClass+0xAC`, then increments the fire type index with wrap. Evidence: live decompile `0x0043C0D0`; callee list includes `AnimClass__Constructor`, `IsometricPixelToWorld`, `Random__RandomRanged`; disassembled range `0x0043C0D0..0x0043C24F`.

### 3.2 Building Damage/Death Debris and Destruction Anims

Active in YR: Yes / Conditional. `BuildingClass::ReceiveDamage @ 0x00442230` delegates to `TechnoClass::ReceiveDamage`. For result 2/3, it enters the building damage side-effect path. For each foundation cell, if `WarheadType+0x14A` is set, it rolls `RandomRanged(0, foundationHeight + foundationWidth + 5)`. Results 1..5 spawn `Rules+0xB78[0]` with loop `RandomRanged(1,3)`, 6..8 spawn `Rules+0xB78[1]` with loop `RandomRanged(1,3)`, and 9 spawns `Rules+0xB78[2]` with loop 1. Constructor flags are `0x600`; non-null constructed anims call `AnimClass::SetOwnerObject(building)`.

`BuildingClass__DestructionEffects @ 0x004415F0` first destroys/clears the 8 damage-fire slots. It then conditionally spawns per-foundation-cell anims from `BuildingType+0x730/+0x73C`, adjacent overlay/tree-related explosion anims when `BuildingType+0xD15 != 0`, and one random destruction anim from `BuildingType+0x74C/+0x758`. Constructor flags are `0x600`; optional palette/name copy occurs for the last branch. Evidence: live decompile `0x00442230`, `0x004415F0`; callee graphs include `AnimClass__Constructor`, `Random__Next`, `Random__RandomRanged`, `ParticleSystemClass__Constructor`, `SpawnDebris`, `Debris_Smoke`.

### 3.3 `BuildingClass::CreateFireAnim`

Active in YR: Conditional, but not ordinary damaged-building fire. The only direct caller found was `SuperClass__Launch @ 0x006CC390`. The helper looks up an AnimType, gets building center through vtable `+0x48`, allocates `0x1C8`, constructs `AnimClass(type, coord, 0, 1, 0x600, 0, 0)`, calls `FUN_00424C90` and `FUN_00424CA0` for draw/z offsets, and sets `AnimClass+0x19D = 1`. Evidence: live decompile `0x0043B5E0`; caller graph.

### 3.4 `ObjectClass::ReceiveDamage` Conditional Special Death Anim

Active in YR: Conditional. `ObjectClass::ReceiveDamage @ 0x005F5390` has one scoped allocation when damage reduces health below 1, RTTI is `0x0F`, force/ignore flag is false, and a type/instance flag pair is satisfied. It allocates `0x1C8` and constructs an anim from `Rules+0x9C` at object location with delay 0, loop 1, flags `0x600`, facing 0, z 0. It then clamps health nonzero, sets an instance flag, calls vtable `+0x558`, and returns state 3 instead of immediate death. Existing RTTI evidence says `0x0F` is InfantryClass; exact flag/key naming was not decoded in this slot. Evidence: live decompile `0x005F5390`; callee graph includes `AnimClass__Constructor`.

### 3.5 Terrain Fire and Terrain Death

Active in YR: Conditional, with unresolved caller for `Catch_Fire`. `TerrainClass::Catch_Fire @ 0x0071C5B0` gates on `on_fire == 0`, `one_shot == 0`, `TerrainType.Armor == 6`, and `TerrainType.SpawnsTiberium == false`. On success it picks `Rules+0xB94[(RandomNext & 1)]`, constructs an anim at terrain coords with delay 0, loop count `0xFF`, flags `0x600`, facing 0, z 0, calls `AnimClass::SetOwnerObject(terrain)`, subtracts `0x14` from `AnimClass+0x100`, sets terrain on-fire, and returns true.

`TerrainClass::Take_Damage @ 0x0071B920` enters only when the warhead pointer is non-null, warhead Wood flag `+0x147` is set, and the terrain type is not Immune. If `ObjectClass::ReceiveDamage` returns 4, TIBTRE/SpawnsTiberium terrain spawns an explosion from `Warhead__SelectExplosionAnim` with constructor flags `0x2600`, then applies area damage with `Rules+0xFA8`. Non-TIBTRE terrain does not allocate `AnimClass` on death here; it sets one-shot TerrainClass death state. Evidence: live decompile `0x0071C5B0`, `0x0071B920`; `TERRAIN_CLASS_GHIDRA_REPORT.md`.

## 4. INI Keys

| Key | Source checked | Binary use | Effect | Active in YR |
|---|---|---|---|---|
| `[General] DamageFireTypes` | `rulesmd.ini:519 = FIRE01,FIRE02,FIRE03` | `Rules+0x2A4/+0x2B0`, `0x0043C0D0` | building damage fire types | Yes |
| `[General] ConditionYellow` | `rulesmd.ini:753 = 50%` | `Rules+0x1700`, `0x0043FB20` | ordinary damage-fire threshold | Yes |
| `[General] ConditionRed` | `rulesmd.ini:752 = 25%` | `Rules+0x1708`, `0x0043FB20` | alternate threshold | Conditional |
| `[General] ChronoSparkle1` | `rulesmd.ini:554 = CHRONOSK` | `Rules+0x344`, `0x0043FB20` | building chrono/warp sparkle | Conditional |
| `[AudioVisual] TreeFire` | commented in retail rules; prior terrain doc | `Rules+0xB94`, `0x0071C5B0` | tree-fire choices | Conditional; caller unresolved |
| `[FIRE01..03]` | `artmd.ini:16018..16035`, Rate 450, LoopCount -1 | AnimType parser/constructor | fire runtime/sounds | Yes |
| `DamageFireOffset0..7` | many `artmd.ini` sections | `BuildingType+0x15D8..0x1614` | fire positions | Yes |

## 5. Integration Points

| Entry | Verified calls | Active in YR | Notes |
|---|---|---|---|
| `BuildingClass::Update @ 0x0043FB20` | calls `CreateDamageFireAnims`; directly constructs chrono sparkles | Yes / Conditional | damage fire runs before chrono early-return |
| `BuildingClass::CreateDamageFireAnims @ 0x0043C0D0` | constructor once per valid slot | Yes | args `0,1,0x600,0,0` |
| `BuildingClass::ReceiveDamage @ 0x00442230` | constructor for Sparky debris | Conditional | result 2/3 and warhead flag |
| `BuildingClass::DestructionEffects @ 0x004415F0` | constructor for destruction/debris lists | Conditional | exact vtable binding not re-audited |
| `BuildingClass::CreateFireAnim @ 0x0043B5E0` | direct caller `SuperClass::Launch` | Conditional | not ordinary damage fire |
| `ObjectClass::ReceiveDamage @ 0x005F5390` | RTTI `0x0F` special anim | Conditional | exact flag/key deferred |
| `TerrainClass::Catch_Fire @ 0x0071C5B0` | owner-attached fire anim | Conditional | caller unresolved |
| `TerrainClass::Take_Damage @ 0x0071B920` | TIBTRE death explosion | Conditional | warhead Wood and not Immune |

## 6. Current Rust Implementation Status

`src/app_building_anim.rs` implements `tick_damage_fire_overlays`, but it is an app-layer overlay vector, not native `AnimClass`. Drift points: it spawns for all structures at `<= ConditionYellow`, chooses fire types by slot modulo rather than initial RNG plus wrap, starts frames by stable-id hashing rather than native RNG, advances by app `dt_ms`, and omits native world conversion, z-adjust, constructor/global-object ordering, and threshold selector at `Type+0x157B`.

`src/rules/art_data.rs` parses `DamageFireOffset0..7` until the first missing key. That matches contiguous retail data, but exact native sentinel/malformed behavior is not mirrored. No generic Rust `AnimClass` entity exists for building destruction debris, terrain tree-fire, TIBTRE death explosions, or ObjectClass conditional special death anims.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| constructor caller presence | verified | caller graph for `0x00421EA0` | full constructor semantics covered elsewhere |
| `CreateDamageFireAnims` | verified | decompile/callees/disassembly range | none for spawn args/order |
| `BuildingClass::Update` damage-fire gate | verified | live decompile, update AI doc | exact label for `Type+0x157B` |
| `BuildingClass::ReceiveDamage` Sparky debris | verified | live decompile/callees | exact key name for `Warhead+0x14A` |
| `BuildingClass::DestructionEffects` | touched-not-exhausted | live decompile/callees | exact parser labels and vtable binding |
| `CreateFireAnim` | verified | live decompile/caller graph | superweapon case details out of scope |
| `ObjectClass::ReceiveDamage` special anim | touched-not-exhausted | live decompile/callees | exact RTTI-15 flag/key |
| `TerrainClass::Catch_Fire` | touched-not-exhausted | live decompile, terrain doc | caller path |
| `TerrainClass::Take_Damage` spawn branch | verified | live decompile/callees | exact `Rules+0xFA8` key |
| Rust damage fire overlay | verified-current | `src/app_building_anim.rs`, `src/rules/art_data.rs` | native RNG/order/z/world/full AnimClass |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - Does damaged-building fire allocate real AnimClass objects? -> Yes, up to 8 standalone constructor calls stored at `BuildingClass+0x5C8..0x5E4`.` (evidence: `0x0043C0D0`, `0x0043FB20`)
- `[RESOLVED] OQ-02 - Are damage fires spawned in draw/render path? -> Not in the live pass; `BuildingClass::Update` directly gates and calls creation.` (evidence: `0x0043FB20`)
- `[RESOLVED] OQ-03 - Does `CreateDamageFireAnims` create fires over time? -> No, it scans all valid contiguous slots in one call and returns early on sentinel/occupied slot.` (evidence: `0x0043C0D0`)
- `[RESOLVED] OQ-04 - Is fire type selection deterministic by slot? -> No, first index is `RandomRanged(0,count-1)`, then wraps incrementally.` (evidence: `0x0043C0D0`)
- `[RESOLVED] OQ-05 - Are start frames deterministic? -> No, each positive-frame fire consumes `RandomRanged(0,frame_count-1)` into `AnimClass+0xAC`.` (evidence: `0x0043C0D0`)
- `[RESOLVED] OQ-06 - What removes building damage fires? -> `Update` removes on threshold false; `DestructionEffects` also clears slots by vtable `+0xF8`.` (evidence: `0x0043FB20`, `0x004415F0`)
- `[RESOLVED] OQ-07 - Does `CreateFireAnim` belong to normal damage fire? -> No direct damage caller found; only `SuperClass::Launch`.` (evidence: caller graph for `0x0043B5E0`)
- `[RESOLVED] OQ-08 - Does `ReceiveDamage` allocate building debris anims? -> Yes, for result 2/3 when warhead `+0x14A` is set.` (evidence: `0x00442230`)
- `[RESOLVED] OQ-09 - Does terrain death always allocate AnimClass? -> No; TIBTRE death allocates explosion anim, ordinary tree death uses TerrainClass one-shot state.` (evidence: `0x0071B920`)
- `[RESOLVED] OQ-10 - Does terrain fire create owner-attached AnimClass? -> Yes in `Catch_Fire`, including owner set and `+0x100 -= 0x14`.` (evidence: `0x0071C5B0`)
- `[DEFERRED] OQ-11 - Who calls `TerrainClass::Catch_Fire` in ordinary YR?` (category: requires-different-system-context; reason: no direct callers; next-step-if-pursued: trace `AnimClass::AI` damage-owner path)
- `[DEFERRED] OQ-12 - Exact key names for `BuildingType+0x730/+0x73C/+0x74C/+0x758` destruction lists.` (category: bounded-cost-too-high; reason: runtime constructor use verified but parser not re-scanned)
- `[DEFERRED] OQ-13 - Exact flag/key for `ObjectClass::ReceiveDamage` RTTI-15 special anim.` (category: bounded-cost-too-high; reason: poor decompiler typing around the flag)
- `[DEFERRED] OQ-14 - Exact `Rules+0xFA8` key for TIBTRE death splash.` (category: out-of-scope; reason: inherited terrain-doc uncertainty)

## 9. Visual/UI Composition Ledger

| Order | Function / address | Condition | Asset / frame | Anchor | Active? | Role |
|---|---|---|---|---|---|---|
| 1 | `BuildingClass::Update @ 0x0043FB20` | threshold transition | none | none | yes/conditional | spawn decision |
| 2 | `CreateDamageFireAnims @ 0x0043C0D0` | valid offset and empty slot | `DamageFireTypes`, random frame | building render coord + pixel offset | yes | world anim |
| 3 | `ReceiveDamage @ 0x00442230` | result 2/3 + Sparky | `Rules+0xB78` debris | foundation-cell randomized coord | conditional | debris |
| 4 | `Catch_Fire @ 0x0071C5B0` | wood armor, not TIBTRE | `Rules+0xB94[RandomNext&1]` | terrain coord | conditional | tree fire |
| 5 | `Take_Damage @ 0x0071B920` | SpawnsTiberium death | `SelectExplosionAnim` | terrain coord | conditional | death explosion |

Asset role matrix:

| Asset | Loaded | Drawn | Visible target | Overlay | Transition-only | Evidence |
|---|---|---|---|---|---|---|
| `FIRE01/FIRE02/FIRE03` | yes | yes | damaged buildings | yes | no | `rulesmd.ini:519`, `artmd.ini:16018..16035`, `0x0043C0D0` |
| `CHRONOSK` | yes | yes | building chrono/warp | yes | yes | `rulesmd.ini:554`, `0x0043FB20` |
| `Rules+0xB78` debris | yes | yes | Sparky building damage | yes | no | `0x00442230` |
| `Rules+0xB94` tree fire | conditional | yes if called | burning terrain | yes | no | `0x0071C5B0` |

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Building damage fire is native AnimClass creation gated by `Update`; first type and each start frame consume native RNG. | `0x0043FB20`, `0x0043C0D0`, `rulesmd.ini:519/752/753` | mismatch: deterministic ad-hoc overlay | `src/app_building_anim.rs`, `src/rules/art_data.rs`, future AnimClass pool | Preserve slot scan, early returns, RNG order, constructor args, frame randomization, z clamp, removal | Damage a stock building below threshold with three offsets: all fires spawn in one update, first type starts at native RNG index and wraps. Proposed test: `damage_fire_spawns_native_rng_slot_order_and_z_adjust` | Do not choose `FIRE01/02/03` by `slot % count`; do not advance by render wall time only |
| Building damage/death debris are real AnimClass objects from rules/type lists with owner attachment where applicable. | `0x00442230`, `0x004415F0` | missing generic debris/destruction anims | sim damage events plus app AnimClass runtime/render | Emit constructor-equivalent anims for Sparky threshold damage and destruction lists | Sparky warhead crossing a building to red can spawn Rules debris by native roll bands; non-Sparky does not. Proposed test: `building_sparky_damage_spawns_rules_debris_animclass_only_on_roll_cases` | Do not route through `DamageFireOverlays` |
| Terrain TIBTRE death spawns selected explosion AnimClass and chained area damage; ordinary tree death does not allocate AnimClass in `Take_Damage`. | `0x0071B920`, terrain doc | terrain damage/fire runtime absent | `src/sim/terrain_object.rs`, `src/sim/world`, app anim runtime/render | Preserve Wood/Immune/SpawnsTiberium gates and separate TIBTRE explosion from one-shot tree death | Killing TIBTRE with Wood warhead emits one `0x2600` explosion anim and chained splash; killing TREE01 starts one-shot terrain death. Proposed test: `terrain_tibtre_death_spawns_explosion_but_tree_death_uses_one_shot_state` | Do not make all terrain deaths spawn explosion anims |

### Negative Facts / Do Not Do

- Do not describe `CreateDamageFireAnims` as render/draw-path lazy spawning; live `BuildingClass::Update @ 0x0043FB20` gates and calls it.
- Do not use the 21-slot building anim array for damage fire; damage fire uses `+0x5C8..+0x5E4`.
- Do not implement `CreateFireAnim` as ordinary damaged-building fire; only `SuperClass::Launch @ 0x006CC390` directly calls it.
- Do not use `IsFlammable` as the terrain fire gate; `Catch_Fire` gates on armor `6`.
- Do not remove ordinary tree terrain immediately on death if reproducing native visuals; non-TIBTRE uses one-shot TerrainClass state.

### Remaining Uncertainty

- Exact caller for `TerrainClass::Catch_Fire` remains unresolved.
- Exact names for building destruction type-list fields (`+0x730/+0x73C/+0x74C/+0x758`) were not decoded from the parser.
- Exact flag/key for the `ObjectClass::ReceiveDamage` RTTI-15 special death anim was not decoded.
- Exact `Rules+0xFA8` key for TIBTRE death splash remains unresolved.

### Stale Docs / Follow-up Docs

- `docs/research/DAMAGE_FIRE_ANIMS_GHIDRA.md`: replace "The function `0x43c0d0` is called through the rendering pipeline - specifically from the building draw path" with "In the current live Ghidra pass, `BuildingClass::Update @ 0x0043FB20` calls `BuildingClass::CreateDamageFireAnims @ 0x0043C0D0` when the cached damage-fire state at `BuildingClass+0x5E8` changes false->true; the false transition removes the 8 fire anim slots with vtable `+0xF8`."
- `docs/research/BUILDING_DAMAGE_DESTRUCTION_GHIDRA_REPORT.md`: replace "Called when a building transitions to ConditionYellow/Red" with "Called from `BuildingClass::Update @ 0x0043FB20` when the cached damage-fire state flips true; the threshold is `ConditionYellow` when `BuildingType+0x157B == 0` and `ConditionRed` when `BuildingType+0x157B != 0`."
- `docs/research/BUILDINGCLASS_UPDATE_AI_TICK_GHIDRA_REPORT.md`: replace "`Type+0x157B` (bool) - `CanBeOccupied`" with "`Type+0x157B` (bool) - threshold selector for damage-fire state; exact INI label conflicts with destruction docs and should not be called `CanBeOccupied` without a parser audit."

## Sources

- Ghidra read-only decompiled: `0x0043C0D0`, `0x0043B5E0`, `0x004415F0`, `0x00442230`, `0x0043FB20`, `0x005F5390`, `0x0071C5B0`, `0x0071B920`, `0x006CC390`.
- Ghidra caller/callee graphs: `AnimClass::Constructor @ 0x00421EA0`, `CreateDamageFireAnims`, `CreateFireAnim`, `BuildingClass::Update`, `TerrainClass::Take_Damage`, `ObjectClass::ReceiveDamage`.
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini`, `ini/artmd.ini`, `ini/art.ini`.
- Docs referenced: `DAMAGE_FIRE_ANIMS_GHIDRA.md`, `BUILDING_DAMAGE_DESTRUCTION_GHIDRA_REPORT.md`, `BUILDINGCLASS_UPDATE_AI_TICK_GHIDRA_REPORT.md`, `TERRAIN_CLASS_GHIDRA_REPORT.md`, `CONTINUOUS_GARRISON_MUZZLE_FLASH_CADENCE_GHIDRA_REPORT.md`.
- Rust checked: `src/app_building_anim.rs`, `src/app_instances/overlays.rs`, `src/rules/art_data.rs`, `src/sim/components.rs`, `src/sim/game_entity.rs`.
