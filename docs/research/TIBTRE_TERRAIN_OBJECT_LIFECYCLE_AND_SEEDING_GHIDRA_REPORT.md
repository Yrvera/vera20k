# TIBTRE Terrain Object Lifecycle And Seeding -- Ghidra Research Report

**Address(es):** `0x0071CA70` (TerrainClass map `[Terrain]` read), `0x0071BB90` (TerrainClass typed constructor), `0x0071D000` (Unlimbo), `0x0071C930` (Limbo), `0x0071C110` / `0x0071C070` (occupation mark/unmark), `0x0071C730` (AI tick), `0x0071CB90` (map `[Terrain]` write), `0x0071B920` (Take_Damage), `0x0071DEA0` (TerrainTypeClass ReadINI)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** TIBTRE terrain object lifecycle relevant to Rust spawner seeding and mutability: map `[Terrain]` read/write, Unlimbo placement side effects, source-cell overlay clearing, occupation/blocking bits, damage/limbo/destruction despite `Immune=yes`, object-liveness tie for ticking, and scenario map save/load state.
**Non-Scope:** Terrain `Light*` keys, detailed ore spawn RNG/type/gates/place effects, renderer lighting, and full savegame serializer reverse engineering.
**Confidence:** High for map read/write, constructor/Unlimbo/Limbo, occupation bits, INI flags, damage immunity gate, and current Rust surface; Medium for the precise outer global AI iteration because this slice verified TerrainClass::AI and object-array registration but did not drain the whole object scheduler.
**Active in YR:** Yes. TIBTRE01-03 are registered in stock `rulesmd.ini`, are present in standard YR skirmish map `[Terrain]` sections per prior TIBTRE reports, and the verified TerrainClass paths are not gated by TS-only flags.

## 0. Working Notes Gate

- Target question: Verify TIBTRE terrain object lifecycle relevant to seeding/keeping Rust spawners: map `[Terrain]` read, Unlimbo placement, overlay clearing, occupation/foundation/blocking bits, destruction/limbo despite `Immune=yes`, global liveness ticking, and save/load runtime state.
- Non-goals: Do not reinvestigate terrain Light* keys, ore spawn probability/type/gates/place effects beyond lifecycle implications, or Rust implementation changes.
- Evidence needed to mark COMPLETE: Binary/decompile plus disassembly context for map read/write, Unlimbo/Limbo, Mark/Unmark occupation, AI tick liveness; INI/default proof for TIBTRE Immune/lifecycle flags; current Rust surface scan; each material claim tagged Active in YR.
- Stop conditions: Stop after lifecycle and seeding handoff are resolved or explicitly deferred; do not follow unrelated CellClass Spread/Place details, lighting, renderer, or general damage systems unless they directly control TIBTRE existence.

## 1. Overview

TIBTRE trees are normal map-placed `TerrainClass` objects, not special resource emitters outside the object system. Scenario map load constructs a TerrainClass instance for each `[Terrain]` entry, Unlimbo places it into the map/cell systems, and `TerrainClass::AI` only runs while the TerrainClass object remains live enough to be processed by the normal object update path.

Rust currently seeds an independent `ProductionState::terrain_spawners` map once at load. That matches static stock TIBTRE maps in the common case, but it is a shortcut: the binary's spawn source is the live terrain object, with placement/removal side effects and source-cell overlay clearing.

## 2. Class Layout / Key Offsets

| Offset | Field | Verified behavior | Active in YR |
|---|---|---|---|
| TerrainClass `+0xAC` | current animation frame / image index | Constructor zeros it; AI increments it while the terrain animation timer is active. | Yes, for `IsAnimated=yes` TIBTRE. Evidence: `0x0071BB90`, `0x0071C730`. |
| TerrainClass `+0xB0` | frame-change/dirty byte | Constructor zeros it; AI writes `1` on frame advance and `0` when timer not advancing. | Yes. Evidence: `0x0071BB90`, `0x0071C730`. |
| TerrainClass `+0xB4/+0xB8/+0xBC` | timer snapshot fields | Constructor and AI write current frame counter and timer snapshot state. | Yes. Evidence: `0x0071BB90`, `0x0071C730`. |
| TerrainClass `+0xC0` | active animation timer/remaining total | Constructor zeros it; AI starts it from type `AnimationRate`; midpoint spawn resets it to zero. | Yes. Evidence: `0x0071BB90`, `0x0071C730`. |
| TerrainClass `+0xC4` | frame increment per timer expiration | Constructor initializes to `1`; AI adds it to current frame. | Yes. Evidence: `0x0071BB90`, `0x0071C730`. |
| TerrainClass `+0xC8` | TerrainTypeClass pointer | Constructor stores the type pointer; load fix-up resolves it; map write uses it to write the type name. | Yes. Evidence: `0x0071BB90`, `0x0071CDA0`, `0x0071CB90`. |
| TerrainClass `+0xCD` | one-shot destroy flag | Constructor zeros it; AI checks it before the TIBTRE spawn block and invokes vtable `+0xF8` at last frame if set. | Conditional; not set by stock TIBTRE constructor/INI in this slice. Evidence: `0x0071BB90`, `0x0071C730`. |
| TerrainTypeClass `+0x233` | `Immune` | Read by `TerrainClass::Take_Damage`; when true, the normal Wood-warhead damage branch is skipped. | Yes for stock TIBTRE01-03. Evidence: `rulesmd.ini [TIBTRE01-03] Immune=yes`, `0x0071B920`, inherited ObjectType read via `0x0071DEA0`. |
| TerrainTypeClass `+0x298/+0x2B8` | Foundation index/pointer | Read from art `Foundation`; TIBTRE art uses `1x1`; pointer is computed from the foundation table. | Yes. Evidence: `artmd.ini [TIBTRE01-03] Foundation=1x1`, `0x0071DEA0`. |
| TerrainTypeClass `+0x2A8/+0x2AC` | temperate/snow occupation masks | Mark/Unmark choose snow mask when scenario theater field equals `1`, otherwise temperate mask. | Yes. Evidence: `0x0071C110`, `0x0071C070`, `rulesmd.ini [TIBTRE03] TemperateOccupationBits=4 SnowOccupationBits=7`; defaults verified by prior `TERRAIN_CLASS_GHIDRA_REPORT.md`. |
| TerrainTypeClass `+0x2B1/+0x2B3` | `SpawnsTiberium` / `IsAnimated` | Both must be true for the TerrainClass::AI ore-spawn midpoint block. | Yes for stock TIBTRE01-03. Evidence: `rulesmd.ini`, `0x0071DEA0`, `0x0071C730`. |

## 3. Core Logic

### Map `[Terrain]` Read And Constructor

Active in YR: Yes. Evidence: `0x0071CA70` decompile plus assembly context at `0x0071CA70`; prior TIBTRE map coverage report confirms standard YR skirmish maps contain TIBTRE entries.

`TerrainClass__Read_Map_Section` clears the INI section cache, counts entries in section `"Terrain"`, reads each key/value, resolves the value string through `TerrainTypeClass__Find_Or_Allocate`, parses the key as coordinates, allocates exactly `0xE0` bytes, and calls the typed constructor. For map format version >= 4, the key is decoded as `rx = key % 1000`, `ry = key / 1000`; older map format uses a 7-bit packed coordinate path.

The typed constructor initializes animation state to idle: current frame `0`, dirty byte `0`, active timer `0`, frame increment `1`, one-shot flag `0`, and then Unlimbos at cell center `(rx*256+128, ry*256+128, z=0)` unless the coordinate equals the global sentinel. It inserts the object into the global TerrainClass array and a global ID/object side table.

### Unlimbo Placement Side Effects

Active in YR: Yes. Evidence: `0x0071D000` decompile plus assembly context at `0x0071D000`; constructor `0x0071BB90` calls Unlimbo for map objects.

On successful `ObjectClass::Reveal`, TerrainClass::Unlimbo:

- visits all 8 neighboring cells using `g_DirectionOffsets` and increments each neighbor cell's byte `+0x122` by one;
- computes render/client extent data into TerrainClass fields `+0xD8/+0xDC`;
- checks the source cell overlay slot `CellClass+0x44`;
- if an overlay exists and its `OverlayTypeClass+0x2A9` flag is true, clears `CellClass+0x44` to `-1` and `CellClass+0x11E` overlay data to `0`.

Lifecycle implication: a TIBTRE placed on top of an incompatible overlay removes that overlay at placement. Rust seeding a spawner from map terrain without applying the Unlimbo overlay clear can leave an impossible source-cell overlay/resource state.

### Occupation / Blocking Bits

Active in YR: Yes. Evidence: `0x0071C110` and `0x0071C070` decompile plus assembly context at both entry points.

TerrainClass Mark/Unmark occupation choose the occupation mask from `TerrainTypeClass+0x2AC` on snow theater, otherwise `+0x2A8`. Source mask bits map to `CellClass+0x124` bits:

| Source mask bit | Mark sets | Unmark clears |
|---|---|---|
| `0x01` | `Cell+0x124 |= 0x04` | `Cell+0x124 &= 0xFB` |
| `0x02` | `Cell+0x124 |= 0x08` | `Cell+0x124 &= 0xF7` |
| `0x04` | `Cell+0x124 |= 0x10` | `Cell+0x124 &= 0xEF` |

TIBTRE01 and TIBTRE02 use default occupation masks from TerrainTypeClass defaults unless overridden elsewhere. TIBTRE03 in stock rules sets `TemperateOccupationBits=4` and `SnowOccupationBits=7`, so on temperate maps it only sets/clears `0x10`, while on snow it sets/clears `0x04|0x08|0x10`.

### Limbo / Removal

Active in YR: Conditional. It is live object code and is reached by destructor/removal paths, but stock TIBTRE `Immune=yes` prevents the ordinary Wood-warhead damage path from killing it. Evidence: `0x0071C930`, `0x0071B7B0`, `0x0071B920`.

TerrainClass::Limbo checks object byte `+0x81`; if the object is not already limboed, it decrements the same 8-neighbor `Cell+0x122` counters, clears source-cell `Cell+0x124` bit `0x40`, then calls ObjectClass concealment and cell recalculation. In non-map-editor mode it also assigns orphaned cell zones, triggers zone refresh, and marks terrain dirty on radar.

The TerrainClass destructor path (`0x0071B7B0`) removes the object from the global terrain object array and, if the game is active and the object has a type pointer, sets a liveness-ish byte at `+0x90` (`param_1[0x24]`) and calls Limbo.

### Damage / Destruction Despite Immune

Active in YR: Conditional. Damage code is live, but ordinary stock TIBTRE has `Immune=yes`, so the normal tree-damage branch is skipped. Evidence: `rulesmd.ini [TIBTRE01-03] Immune=yes`, `0x0071B920`.

`TerrainClass::Take_Damage` first requires a non-null warhead pointer. It then requires the warhead's Wood-capable byte (`Warhead+0x147`) and `TerrainTypeClass+0x233 Immune == false` before calling `ObjectClass::ReceiveDamage`. Therefore stock TIBTRE01-03 are not destroyed through this ordinary terrain damage path. If a mod changes `Immune=no`, or if another non-damage removal/destructor path removes the object, the binary does have Limbo/destruction side effects and the spawner should not keep ticking as if the source object still exists.

Negative fact: `Immune=yes` is not a proof that TerrainClass objects have no lifecycle. It is only a gate in the normal damage path verified here.

### AI Tick / Spawner Liveness

Active in YR: Yes for live TIBTRE TerrainClass instances. Evidence: `0x0071C730` decompile and assembly context at entry; `0x0071BB90` global TerrainClass array insertion; prior TIBTRE reports for virtual-dispatch reachability.

`TerrainClass::AI` is the owner of TIBTRE ore spawning. It calls `ObjectClass::AI` first, then if `IsAnimated` and no active animation timer, rolls the probability. When the animation timer expires, it advances the frame. If the type has both `SpawnsTiberium` and `IsAnimated`, and the current frame equals half the image frame count, it resets current frame and timer to zero, resolves the source cell from the terrain object's own coordinates, and calls `CellClass::SpreadTiberium(1)`.

Lifecycle implication: the binary does not tick an abstract spawner list detached from terrain object liveness. The spawn attempt is a method on each live TerrainClass instance. Rust can use a separate map for scale, but it must stay synchronized with terrain object creation/removal if those become mutable.

### Scenario Map `[Terrain]` Write

Active in YR: Yes for map/scenario writing tools or editor/save-to-map flows; not a mid-match savegame state proof. Evidence: `0x0071CB90` decompile plus assembly context at entry.

`TerrainClass__Write_Map_Section` clears/recreates the `"Terrain"` section, iterates the global TerrainClass array, and writes entries only when the pointer is non-null, object byte `+0x81 == 0`, and `(char)TerrainClass+0x90 != 0`. The key is `rx + ry*1000`; the value is the TerrainType name. It does not write current animation frame, animation timer, pending midpoint state, or a separate spawner state.

Scenario map round-trip implication: a map write/read preserves object type and cell, not runtime animation/spawn progress. A TIBTRE reloaded from a written map reconstructs idle animation state from the constructor.

## 4. INI Keys

| Key / section | Stock value | Binary reader / effect | Active in YR |
|---|---|---|---|
| `[TerrainTypes] 46-48` | `TIBTRE01`, `TIBTRE02`, `TIBTRE03` | Map values resolve through TerrainType registry / `TerrainTypeClass__Find_Or_Allocate`. | Yes. Evidence: `rulesmd.ini`, `0x0071CA70`. |
| `[TIBTRE01-03] SpawnsTiberium` | `yes` | Read at `TerrainTypeClass+0x2B1`; AI requires it for midpoint spawn. | Yes. Evidence: `rulesmd.ini`, `0x0071DEA0`, `0x0071C730`. |
| `[TIBTRE01-03] IsAnimated` | `yes` | Read at `+0x2B3`; AI requires it for probability/animation and midpoint spawn. | Yes. Evidence: `rulesmd.ini`, `0x0071DEA0`, `0x0071C730`. |
| `[TIBTRE01-03] AnimationRate` | `3` | Read at `+0x2A0`; copied into active timer when an animation starts. | Yes. Evidence: `rulesmd.ini`, `0x0071DEA0`, `0x0071C730`. |
| `[TIBTRE01-03] AnimationProbability` | `.003` | Read at `+0x2A4`; compared against normalized modulo-million roll. | Yes. Evidence: `rulesmd.ini`, `0x0071DEA0`, `0x0071C730`. |
| `[TIBTRE01-03] Immune` | `yes` | Inherited ObjectType read; `Take_Damage` checks `+0x233 == false` before damage. | Yes. Evidence: `rulesmd.ini`, `0x0071B920`. |
| `[TIBTRE03] Armor/IsVeinhole/Strength` | `Armor=None`, `IsVeinhole=true`, `Strength=1000` | Type read applies inherited fields and IsVeinhole side effect sets LegalTarget and clears another flag. | Yes for TIBTRE03 only. Evidence: `rulesmd.ini`, `0x0071DEA0`. |
| `[TIBTRE03] TemperateOccupationBits/SnowOccupationBits` | `4` / `7` | Mark/Unmark choose by theater and set/clear `Cell+0x124` bits. | Yes. Evidence: `rulesmd.ini`, `0x0071C110`, `0x0071C070`. |
| `artmd.ini [TIBTRE01-03] Foundation` | `1x1` | Read by `FUN_00474DA0`; stores index at `+0x298`, pointer at `+0x2B8`. | Yes. Evidence: `artmd.ini`, `0x0071DEA0`. |

## 5. Integration Points

| Integration point | Verified behavior | Active in YR |
|---|---|---|
| Map load | `[Terrain]` entries construct TerrainClass instances at cell centers and Unlimbo them immediately. | Yes. Evidence: `0x0071CA70`, `0x0071BB90`, stock map reports. |
| Placement | Unlimbo increments 8-neighbor terrain adjacency bytes and clears source overlay if overlay type `+0x2A9` is true. | Yes. Evidence: `0x0071D000`. |
| Occupancy | Mark/Unmark map type occupation masks to `Cell+0x124` bits `0x04/0x08/0x10`. | Yes. Evidence: `0x0071C110`, `0x0071C070`. |
| Removal | Limbo decrements neighbor counters, clears source `0x40`, recalculates cell/zone/radar state in normal mode. | Conditional for stock TIBTRE because `Immune=yes` prevents normal damage removal; live for object removal paths. Evidence: `0x0071C930`, `0x0071B920`. |
| Tick | TerrainClass::AI owns TIBTRE spawn timing and calls ObjectClass::AI first. | Yes for live objects. Evidence: `0x0071C730`; prior virtual dispatch reports. |
| Scenario map write | Writes only live, non-limbo terrain objects as `rx+ry*1000=type name`; no runtime animation state. | Yes for scenario/map write path. Evidence: `0x0071CB90`. |

## 6. Current Rust Implementation Status

Current Rust surface:

- `src/sim/terrain_spawn.rs`: `TerrainSpawnerState` is serialized, keyed by cell, and explicitly documents that the spawner "doesn't move and isn't destroyable (`Immune=yes` on TIBTRE), so the only lifecycle is exists from map load to game end."
- `src/sim/terrain_spawn.rs::seed_terrain_spawners`: seeds from parsed map `TerrainObject` entries whose RuleSet terrain type has `spawns_tiberium && is_animated`.
- `src/sim/world/mod.rs`: ticks terrain spawners after ore growth each simulation tick.
- `src/sim/production/production_types.rs`: stores `terrain_spawners` and `default_ore_overlay_id` in serializable `ProductionState`.
- `src/sim/snapshot.rs`: serializes the full Simulation through bincode, so Rust's current spawner map is preserved across Rust save/load.

Rust deltas from this lifecycle slice:

- Rust has no live TerrainClass object lifecycle surface in sim, so a seeded spawner cannot be removed if a terrain object is destroyed/limboed later.
- Rust does not apply TerrainClass::Unlimbo source-cell overlay clearing when it seeds a spawner from map terrain.
- Rust does not model TerrainType occupation/foundation side effects for TIBTRE as part of spawner seeding; passability rejection currently uses `PathGrid::is_walkable` plus `spawners.contains_key`.
- Rust scenario/map save is not equivalent to gamemd map `[Terrain]` write; Rust snapshot preserves `terrain_spawners`, while gamemd scenario `[Terrain]` write reconstructs idle TerrainClass state.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Working-note gate | verified | Required four lines recorded in §0 | none |
| TerrainClass map `[Terrain]` read | verified | `0x0071CA70` decompile + assembly context | none |
| Typed constructor and initial animation state | verified | `0x0071BB90` decompile + assembly context | none |
| TerrainClass Unlimbo overlay clearing | verified | `0x0071D000` decompile + assembly context | Exact identity of `OverlayType+0x2A9` belongs to overlay-type research, not this lifecycle slice. |
| TerrainClass Limbo side effects | verified | `0x0071C930` decompile + assembly context | none for lifecycle handoff |
| Occupation mark/unmark bits | verified | `0x0071C110`, `0x0071C070` decompile + assembly context | none |
| Stock TIBTRE Immune gate | verified | `rulesmd.ini`, `0x0071B920` | Non-damage scripted deletion routes not exhaustively enumerated. |
| TerrainClass::AI spawn ownership | verified | `0x0071C730` decompile + assembly context | Outer global scheduler remains medium-confidence via prior docs, not redrained here. |
| TerrainClass scenario `[Terrain]` write | verified | `0x0071CB90` decompile + assembly context | none for scenario map state |
| Binary savegame serialization | touched-not-exhausted | `0x0071CF50`, `0x0071CDA0`, `0x005F6250`, `0x005F5E80` | The generic serializer naming is ambiguous; not needed for scenario `[Terrain]` map write conclusion. |
| Rust terrain spawner seeding/tick/snapshot | verified | Codegraph + `src/sim/terrain_spawn.rs`, `src/sim/world/mod.rs`, `src/sim/snapshot.rs` | No Rust changes made. |
| Terrain Light* keys | not-touched | Prior report `TERRAIN_OBJECT_LIGHT_KEYS_AND_LIGHTSOURCE_OWNERSHIP_GHIDRA_REPORT.md` | Already settled; intentionally not repeated. |

## 8. Open Questions -- Final State Of The Investigation Log

- `[RESOLVED] Q1 -- Is TIBTRE a live TerrainClass object created from map [Terrain]? -> Yes, map [Terrain] values are TerrainType names, allocated as 0xE0 TerrainClass objects and constructed.` (evidence: `0x0071CA70`, `0x0071BB90`, `rulesmd.ini [TerrainTypes]`)
- `[RESOLVED] Q2 -- Does constructor seed idle animation state or inherit runtime state from map data? -> Constructor seeds idle state: frame/timer/snapshots zeroed, frame increment set to 1.` (evidence: `0x0071BB90`)
- `[RESOLVED] Q3 -- Does Unlimbo clear ore/overlay on the TIBTRE source cell? -> It clears source cell overlay id to -1 and overlay data to 0 when the overlay type has byte +0x2A9 set.` (evidence: `0x0071D000`)
- `[RESOLVED] Q4 -- Does Unlimbo affect neighboring cells? -> It increments byte +0x122 on all eight neighboring cells.` (evidence: `0x0071D000`)
- `[RESOLVED] Q5 -- Does terrain occupation use foundation/occupation bits? -> Occupation Mark/Unmark use type masks +0x2A8/+0x2AC to set/clear Cell+0x124 bits 0x04/0x08/0x10.` (evidence: `0x0071C110`, `0x0071C070`)
- `[RESOLVED] Q6 -- Are TIBTRE stock objects immune to normal terrain damage? -> Yes, stock TIBTRE01-03 have Immune=yes and Take_Damage requires Immune=false before ReceiveDamage.` (evidence: `rulesmd.ini`, `0x0071B920`)
- `[RESOLVED] Q7 -- Can TerrainClass still be limboed/removed in the binary? -> Yes, TerrainClass::Limbo and destructor/removal paths exist and perform real side effects; stock TIBTRE ordinary damage does not reach them because Immune=yes.` (evidence: `0x0071C930`, `0x0071B7B0`, `0x0071B920`)
- `[RESOLVED] Q8 -- Is the spawner tied to object liveness rather than an independent list? -> Yes at owner level: spawn call is inside TerrainClass::AI using the object's own type and coordinates; constructor/destructor maintain global object arrays.` (evidence: `0x0071C730`, `0x0071BB90`, `0x0071B7B0`)
- `[RESOLVED] Q9 -- Does scenario map write preserve runtime animation/spawn state? -> No; it writes only live terrain cell key and type name, not animation frame/timer/spawn progress.` (evidence: `0x0071CB90`)
- `[RESOLVED] Q10 -- Do TIBTRE INI keys come from YR md files first? -> Yes, stock `rulesmd.ini` and `artmd.ini` contain the same TIBTRE lifecycle-relevant values as base fallback and are the priority source for YR.` (evidence: `rulesmd.ini`, `artmd.ini`)
- `[RESOLVED] Q11 -- Does Rust currently keep mutable terrain object lifecycle with spawners? -> No; it seeds a serializable BTreeMap from map terrain objects and ticks it independently.` (evidence: `src/sim/terrain_spawn.rs`, `src/sim/production/production_types.rs`)
- `[RESOLVED] Q12 -- Does Rust snapshot preserve its current spawner list? -> Yes; `ProductionState` derives Serialize/Deserialize and `GameSnapshot` serializes the full Simulation.` (evidence: `src/sim/production/production_types.rs`, `src/sim/snapshot.rs`)
- `[DEFERRED] Q13 -- Which exact overlay-type semantic name corresponds to `OverlayTypeClass+0x2A9`?` (category: `requires-different-system-context`; reason: overlay-type layout is outside TIBTRE lifecycle and not needed to prove Unlimbo clears flagged overlays; next-step-if-pursued: run a narrow OverlayTypeClass flag report.)
- `[DEFERRED] Q14 -- Can standard YR triggers/scripts delete TIBTRE despite Immune=yes?` (category: `bounded-cost-too-high`; reason: trigger/action deletion matrix is outside this lifecycle slice; next-step-if-pursued: trace map trigger object-removal actions against TerrainClass targets.)
- `[DEFERRED] Q15 -- Does binary savegame serialization preserve TerrainClass animation counters?` (category: `requires-different-system-context`; reason: scenario map `[Terrain]` save/load is resolved, but generic savegame serializer helpers require a dedicated class serialization pass; next-step-if-pursued: investigate TerrainClass vtable save/load slots and the generic serializer format.)

## 9. Negative Facts / Do Not Do

- Do not treat TIBTRE as an AnimClass ore-spawn source. Active in YR: No for TIBTRE; prior `TIBTRE_ANIMCLASS_ORE_SPAWN_TICK_GHIDRA_REPORT.md` verified TIBTRE uses TerrainClass::AI directly.
- Do not treat `Immune=yes` as proof that terrain spawners can never be removed. Active in YR: Conditional; it blocks the ordinary terrain damage path, but Limbo/destructor code exists and has real cell/zone/radar side effects.
- Do not seed a TIBTRE spawner without accounting for Unlimbo's source-cell overlay clear if the source cell has an incompatible overlay. Active in YR: Yes; Unlimbo performs the clear.
- Do not use one generic blocking bit for all TIBTRE variants. Active in YR: Yes; TIBTRE03 has different temperate vs snow occupation masks in stock INI.
- Do not assume scenario map write/read preserves a half-played TIBTRE animation or pending spawn. Active in YR: Yes for scenario write path; only cell and type are written.

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| TIBTRE spawner source is a live TerrainClass object constructed from `[Terrain]`, not an independent rules-only emitter. | `0x0071CA70`, `0x0071BB90`, `0x0071C730` | Partial: Rust seeds a separate BTreeMap once at load. | `src/sim/terrain_spawn.rs`, future terrain-object sim lifecycle surface | Keep separate map only if it is synchronized with live terrain object create/remove state once terrain objects become mutable. | `tibtree_spawner_removed_when_source_terrain_limboed` -- spawn ticks after a simulated TIBTRE removal must not add ore. | Do not rely permanently on "Immune means exists forever" once terrain damage/scripts are implemented. |
| Unlimbo clears flagged overlay and overlay data on the source cell. | `0x0071D000` | Missing/unchecked: Rust seeds spawner but does not perform TerrainClass::Unlimbo overlay clearing. | Map load overlay/resource seeding order; `src/sim/terrain_spawn.rs`; overlay grid/resource node initialization | Source cell under a TIBTRE must not keep an incompatible ore/vein overlay/resource after terrain object placement. | `tibtree_unlimbo_clears_source_cell_flagged_overlay` -- map with ore overlay and TIBTRE at same cell loads with no resource/overlay on the source cell, while adjacent cells can still receive later spawn. | Do not infer spawn ore type from a source-cell overlay that binary would have cleared during Unlimbo. |
| Occupation masks are type/theater-specific and set/clear `Cell+0x124` bits `0x04/0x08/0x10`. | `0x0071C110`, `0x0071C070`, `rulesmd.ini [TIBTRE03]` | Missing/approximated: Rust uses terrain/path grids and spawner-cell rejection, not exact TerrainClass occupation bits. | Path/passability grid construction and future terrain object occupancy state | Apply terrain occupation according to `TemperateOccupationBits` / `SnowOccupationBits`; TIBTRE03 differs by theater. | `tibtree03_occupation_bits_use_theater_specific_masks` -- temperate TIBTRE03 blocks only the bit represented by mask 4, snow uses all default bits. | Do not hardcode all TIBTRE to a single 1x1 blocker bit pattern. |
| Stock TIBTRE ordinary damage path is blocked by `Immune=yes`, but Limbo/destructor side effects exist. | `rulesmd.ini`, `0x0071B920`, `0x0071C930`, `0x0071B7B0` | Current Rust comment overstates permanence. | `src/sim/terrain_spawn.rs` docs and any future terrain damage/removal implementation | Preserve stock immunity for ordinary damage, but design spawner storage so later scripted/modded removal can delete or disable the source. | `tibtree_immune_damage_does_not_remove_spawner` and `modded_nonimmune_tibtree_damage_removes_spawner` -- stock stays, non-immune/modded removal stops spawning. | Do not delete stock TIBTRE on ordinary weapon damage; do not make the spawner impossible to delete. |
| Scenario map `[Terrain]` write/read stores only live non-limbo terrain cell/type, not runtime animation state. | `0x0071CB90` | Rust snapshot intentionally preserves current `terrain_spawners`; scenario map export behavior not equivalent. | `src/sim/snapshot.rs`; future map editor/save-as-map surface | Distinguish Rust mid-match snapshot from scenario map write. If adding map export, write terrain objects as cell/type and reset runtime animation on reload. | `scenario_terrain_export_resets_tibtree_animation_state` -- exported/reloaded map has same TIBTRE positions/types but no pending midpoint spawn state. | Do not use snapshot serialization behavior as evidence for gamemd scenario `[Terrain]` write parity. |

## 11. Remaining Uncertainty

- Exact semantic name of `OverlayTypeClass+0x2A9` remains outside this slice; the material lifecycle fact is that Unlimbo clears overlays whose type has that byte set.
- Full binary savegame serialization of TerrainClass animation counters was touched but not exhausted. Scenario map `[Terrain]` write/read was verified and is the relevant source for map seeding.
- Standard YR trigger/script paths that can delete a TIBTRE despite `Immune=yes` were not enumerated. The handoff should still keep the Rust spawner collection mutable because the binary object lifecycle has real removal paths.
- Outer global object scheduler was not redrained; this report relies on TerrainClass::AI ownership plus prior virtual-dispatch reachability for the statement that live objects tick.

## Sources

- Ghidra decompile/assembly context: `0x0071CA70`, `0x0071BB90`, `0x0071B7B0`, `0x0071D000`, `0x0071C930`, `0x0071C110`, `0x0071C070`, `0x0071C730`, `0x0071CB90`, `0x0071B920`, `0x0071DEA0`, `0x0071CF50`, `0x0071CDA0`.
- Prior reports: `TERRAIN_CLASS_GHIDRA_REPORT.md`, `TIBTRE_ORE_SPAWNING_GHIDRA_REPORT.md`, `TIBTRE_ANIMCLASS_ORE_SPAWN_TICK_GHIDRA_REPORT.md`, `TERRAIN_OBJECT_LIGHT_KEYS_AND_LIGHTSOURCE_OWNERSHIP_GHIDRA_REPORT.md`.
- INI files checked: `ini/rulesmd.ini`, `ini/rules.ini`, `ini/artmd.ini`, `ini/art.ini`.
- Rust files scanned: `src/sim/terrain_spawn.rs`, `src/sim/production/production_types.rs`, `src/sim/world/mod.rs`, `src/sim/snapshot.rs`, `src/app_init.rs`.
