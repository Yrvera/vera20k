# Damage Area and Producer Timing Reconciliation — 2026-07-13

**Plan unit:** damage authoritative cutover Task 3S  
**Investigation mode:** coverage-map reconciliation of bounded Tasks 3A, 3B, and 3C  
**Binary target:** active retail Yuri's Revenge gamemd.exe 1.001, x86 32-bit  
**Retail SHA-256:** 1cdd1180e49024fbda8ad568caac2e86e856063ff67ab38f62b7d2c7bb84298c  
**Primary anchors:** Apply_area_damage at 0x00489280, TechnoClass::Fire_At at 0x006FDD50, LogicClass::PerTickUpdate at 0x0055AFB0  
**Active in YR:** yes for the named ordinary and conditional routes; dormant and unreferenced entries are identified separately  
**Synthesis status:** COMPLETE  
**Authority status:** PARTIAL; cutover BLOCKED  
**Overall confidence:** HIGH for the bounded dispatcher, ordinary Bullet, death-weapon, periodic-radiation, and lightning contracts; explicitly PARTIAL for adjacent special-effect producer internals and runtime certification

## 1. Concise Verdict

Task 3 now has one coherent static model:

1. Apply_area_damage is a synchronous two-phase transaction. It captures ordered 8-byte identity-plus-distance records, then calls each still-eligible concrete receiver in that exact record order.
2. Ordinary Bullet damage is deferred out of Fire_At but is not uniformly next-frame. Reveal tail-appends the Bullet to the live Logic vector, whose forward loop reloads count after each object. First AI and impact can therefore happen later in the firing frame, or on a later visit.
3. DiskLaser and Wave are not ordinary Bullets. DiskLaser's separate rung has already run when a main-rung Techno creates it, so its earliest damage is the next PerTick call. Wave's separate rung follows the main object rung, so a newly created Wave can damage later in the same frame.
4. Death weapons recurse synchronously inside the lethal receiver/crash call through an ephemeral Bullet that is detonated and destroyed before the helper returns.
5. Radiation has two different paths. Initial impact uses the ordinary Bullet/area route; periodic HP is a direct Foot receiver call after the reverse RadSite pass, with null source object, null source house, current Rules.RadSiteWarhead, distance zero, ignore_defenses=false, and arg6=true.
6. Lightning is global animation-delayed damage. GroundStrike runs only after a tracked animation frame is strictly greater than half its type frame count, then reads the current global owner and current Rules damage/warhead.

The correct status is:

| Boundary | Verdict | Consequence |
|---|---|---|
| Task 3 dispatcher and named-producer G1 rows | RESOLVED | No in-scope area filter, distance conversion, producer call field, or named producer timing fact remains UNKNOWN in this reconciliation. |
| Project-wide G1 | FAILED | Task 1S and Task 2S still contain authority-critical receiver, wrapper, lifecycle, and persisted-state gaps. Task 3 cannot promote global G1 by itself. |
| G2 projectile timing | FAILED / BLOCKED | The verified native scheduler exists as evidence, but no approved and implemented Rust projectile lifecycle owns creation, live-vector AI, exact impact payload, detonation, and removal. |
| G3 retail Oracle | FAILED / NOT RUN | Static evidence is not an executable retail trace. No ordered retail observations, raw numeric bits, RNG receipts, frame cursor, or membership receipts were acquired here. |

This report does not authorize a live damage cutover.

## 2. Scope, Inputs, and Evidence Discipline

### 2.1 Reports read in full

| Source | Lines at reconciliation | SHA-256 at reconciliation | Child verdict retained |
|---|---:|---|---|
| docs/research/DAMAGE_AREA_DISPATCH_REINVESTIGATION_2026-07-13.md | 620 | 75258DAB48B97A7390396111E7970C77FD7341A9A112D87A1C6EE9948A74A405 | COMPLETE for Task 3A |
| docs/research/DAMAGE_PROJECTILE_IMPACT_TIMING_REINVESTIGATION_2026-07-13.md | 552 | C878E1D90B82469F95489C1D453CE946822E29E9C03711F391FEF21C7CE51F6E | PARTIAL overall; ordinary Bullet route VERIFIED; G2 FAILED/BLOCKED |
| docs/research/DAMAGE_SPECIAL_PRODUCER_TIMING_REINVESTIGATION_2026-07-13.md | 548 | DECA42422B60B46AA9B80E9336CC37436EB197EBB4EFAD94284173B7C1A0F402 | COMPLETE for bounded Task 3C |

Directly relevant plan and gate sources were also read:

- docs/plans/2026-07-13-damage-authoritative-cutover-plan.md, Authority Gates, Interface Changes, and Task 3;
- docs/research/DAMAGE_RECEIVER_CORE_REINVESTIGATION_2026-07-13.md for the inherited Task 1 G1 failure;
- docs/research/DAMAGE_CONCRETE_RECEIVER_REINVESTIGATION_2026-07-13.md for the inherited Task 2 G1 failure.

### 2.2 Focused current-Rust reads

The reconciliation directly checked:

- src/sim/combat/combat_aoe.rs;
- src/sim/combat/cell_spread.rs;
- src/sim/combat/mod.rs;
- src/sim/world/logic_vector.rs;
- src/sim/world/mod.rs;
- src/sim/movement/homing_movement.rs;
- src/sim/movement/rocket_movement.rs;
- src/sim/radiation.rs;
- src/sim/superweapon/lightning_storm.rs;
- src/sim/world/world_hash.rs.

### 2.3 Fresh read-only binary reconciliation checks

No broad new reverse engineering was performed. Four narrow checks guarded against transcription or cross-report drift:

- get_xrefs_to(0x00489280) returned the same 33 direct callsites listed in Section 6;
- disassembly 0x0055B5BE..0x0055B623 reconfirmed LightningStorm::Process, then reverse RadSite iteration, then the forward Logic loop with its post-call count reload at 0x0055B613;
- disassembly 0x004DA60F..0x004DA634 reconfirmed the periodic-radiation receiver packet and arg6=true;
- disassembly 0x00489A78..0x00489AC3 reconfirmed the area receiver's final live gates and false/false packet.

No game process, debugger, screen or input operation, Oracle tool, Cargo command, Rust edit, Ghidra mutation, or edit to another research document occurred.

## 3. Unified Tick and Call-Stack Model

### 3.1 Global order relevant to the named producers

| Relative position | Native owner | Iteration behavior | Damage significance | Evidence |
|---:|---|---|---|---|
| before the main object rung | DiskLaser global rung | separate reverse/global array | A DiskLaser created later by a main-rung Techno has missed this rung and cannot damage until the next PerTick call | Task 3B Section 9 |
| global call at 0x0055B5C8 | LightningStorm::Process | tracked animations processed by the storm service | Eligible GroundStrike and its complete area transaction finish here | fresh 0x0055B5BE..0x0055B5C8; Task 3C |
| 0x0055B5CD..0x0055B5E8 | global RadSite vector | starting count, reverse index | Existing sites decay/update before any Foot periodic read | fresh scheduler disassembly; Task 3C |
| 0x0055B608..0x0055B619 | singleton Logic vector | forward index; live count reloaded after every vtable+0x5C call | Techno fire, newly appended Bullet first AI, and Foot periodic radiation occur at each object's exact live-order position | fresh scheduler disassembly; Task 3B/3C |
| after the main object rung | Wave global rung | separate current-count Wave processing | A Wave created in the main rung can damage later in the same frame | Task 3B Section 9 |

The full PerTick function contains other systems between these rows. This table claims only the relative positions proved for the damage routes in scope.

### 3.2 Synchronous nesting rule

Once any producer calls Apply_area_damage:

1. all target records are collected;
2. all eligible receivers are called sequentially;
3. any nested death weapon, nested area transaction, trigger effect, or receiver lifecycle work completes before the outer dispatcher resumes;
4. the dispatcher's late non-HP tail finishes before the producer resumes.

There is no native end-of-tick HP batch between these stages.

## 4. Apply_area_damage Exact Transaction

### 4.1 Entry and radius details

The effective entry carries an exact signed Cartesian world-lepton CoordStruct, signed i32 base damage, nullable source object, nullable warhead, an affect-resource boolean, and nullable source house.

It returns early without collection when:

- base damage is zero;
- the scenario flag byte has bit 0x20 set; or
- warhead is null.

The fine receiver radius is Math__ftol(CellSpread × 256.0) leptons. The airborne spatial-query radius is separately Math__ftol(CellSpread) and is subject to the spatial helper's minimum-bucket behavior. These two values are not interchangeable.

### 4.2 Ordered collection

| Stage | Exact order and mechanism | Tiny parity details |
|---|---|---|
| Airborne | Only when terrain ground height at exact impact XY is strictly less than impact Z; spatial buckets enumerate center then perimeters; scratch entries pop by left shift | Filters alive byte +0x90, byte +0x74, signed health greater than zero, and exact 3D distance at or below the fine radius. Distance is recomputed for the stored record rather than reusing the filter result. |
| Layer selection | One impact-derived ground/deck selector is computed once and reused for every spread cell | Deck requires Cell+0x140 bit 0x100 and impact Z strictly greater than ground height plus 2×H. The initializer computes the intermediate as 4×H, then the selector halves it. Neighboring cells do not choose their own layer. |
| CellSpread table | Band is Math__ftol(CellSpread + 0.99); offsets are read in exact initializer order, with center first | CellSpread zero yields band 0 and one center-cell scan. There is no native clamp beyond the valid 0..11 table domain. |
| Per-cell non-HP work | Overlay/resource work runs before reading that cell's object-list head | A duplicated table cell can observe earlier changes made by the first visit. |
| Per-cell object lists | Chosen CellClass list is followed head-to-tail through Object+0x30 | Non-buildings are normally prepended; buildings are appended. There is no sort and no explicit target deduplication. |

Band 11 contains duplicate offset (-3,11) at indices 319 and 322 and omits (3,-11). Stock content stops at CellSpread 10, but compatible mod input reaching band 11 repeats the cell's pre-list effects and receiver opportunities. Exact parity preserves this defect.

### 4.3 Collection and dispatch filters stay at different times

The cell collector applies source/self handling, alive membership, and the HarvesterImmune/HarvesterUnit rule. It does not yet require positive health, +0x74, non-limbo state, or in-radius distance. Those are checked against live state during dispatch.

The dispatch stage, in order, checks:

1. target+0x90 is nonzero;
2. invisible-in-game Buildings are skipped;
3. transaction-wide near-center invulnerability isolation, if armed;
4. high-flying Aircraft distance is halved with signed truncation toward zero;
5. signed health is greater than zero;
6. target+0x74 is nonzero;
7. target+0x81 is zero, meaning not limbo;
8. signed stored/adjusted distance is at or below the inclusive fine radius.

An earlier receiver can therefore change whether a later record passes, but it cannot change that record's captured identity or captured distance.

### 4.4 Coordinate and distance contract

CoordStruct components are signed i32 Cartesian world leptons. They are not map cells, terrain levels, screen pixels, isometric axes, facing values, or track indices. One X/Y cell is 256 leptons.

Signed lepton-to-cell conversion truncates toward zero using the native signed correction before shift. For example, -255 and -1 map to cell 0, while -256 and -257 map to cell -1.

Distance is:

Math__ftol(Sqrt_Approx(dx² + dy² + dz²))

The helper uses the native float32 lookup/reconstruction path, not host square root and not exact integer square root. Distance stays a signed i32 lepton count through the receiver boundary.

Reference points differ by target path:

| Target path | Native coordinate | Adjustment |
|---|---|---|
| airborne | exact target+0x9C CoordStruct | qualifying Aircraft distance halves at dispatch |
| non-building cell-list target | exact virtual +0xA4 CoordStruct | none |
| Building at noncenter spread offset | center of the currently enumerated cell | none |
| Building at center spread offset | impact-cell center | vertical delta at or below 2×H forces zero; otherwise approximate 3D distance minus 2×H |

### 4.5 Fixed record, lifetime, and re-entry

Each accepted candidate gets one separately allocated 8-byte record:

| Offset | Width | Meaning |
|---|---:|---|
| +0 | 4 | raw target pointer |
| +4 | 4 | captured signed i32 distance in leptons |

The record list stores record pointers. All collection finishes before the first receiver call. Records are later freed.

The record freezes only identity and distance. It does not freeze health, alive/limbo flags, +0x74, owner, armor, coordinates, invulnerability, type-visible state, or receiver result.

Standard ObjectClass::UnInit clears Object+0x90 and queues the allocation in the pending-delete vector. It does not reclaim the allocation inside the area receiver chain. If an earlier record removes a later target, the later pointer remains valid and is skipped by the first live-state check. A Rust implementation should use a safe stable or generational handle while reproducing that observation; it should not emulate unsafe raw-pointer ownership.

### 4.6 Exact area receiver packet

For every accepted record, the dispatcher resets local signed i32 damage from the original base damage, then calls target vtable +0x16C with:

| Argument | Value |
|---|---|
| incoming_damage | pointer to a fresh local signed i32 copy |
| distance_leptons | captured signed i32, possibly Aircraft-halved |
| warhead | unchanged dispatcher argument |
| source object | unchanged nullable dispatcher argument |
| ignore_defenses | false |
| arg6 | false |
| source house | unchanged nullable dispatcher argument |

Fresh assembly 0x00489AA7..0x00489AB6 pushes source house, zero, zero, source, warhead, distance, and the local damage address right-to-left. The receiver result is ignored. A receiver's mutation of its local damage does not carry to the next record.

### 4.7 Non-HP ordering

Before each cell list, the dispatcher may reduce resource by native signed base_damage/10, destroy eligible overlays/walls, and run targeting cleanup.

After all entity receivers, ordinary completion preserves:

1. optional Rocker scan;
2. bridge and wood-bridge work with its Scenario RNG and Z windows;
3. explodable-overlay removal, refresh, animation, and possible recursive area call;
4. debris/particle-side effects for the overlay chain;
5. warhead Particle creation.

The special Rules+0xFAC warhead returns before this late tail. Moving any of these stages into a later Rust world phase changes same-call visibility, RNG order, and recursion.

## 5. Producer Contracts

### 5.1 Ordinary BulletClass

Fire_At allocates and initializes a Bullet, then calls its fire slot. Fire begins with Reveal. Reveal tail-appends a logic-enabled Bullet to singleton vector 0x0087F778 with no sort or next-frame queue.

The main Logic loop reads vector.data[index], calls vtable+0x5C, reloads vector.count at 0x0055B613, increments the index, and continues while index is below that new count. Therefore:

- a Bullet appended by an attacker already being visited can receive first AI later in the same pass;
- pre-existing later entries remain before that appended Bullet;
- if first AI reaches an impact branch, receiver damage can occur in the firing frame;
- if first AI does not impact, the Bullet remains registered for later visits;
- compacting removal shifts successors left while the cursor still increments, so the shifted successor is skipped for the rest of that pass.

Terminal Bullet AI calls BulletDetonation, then vtable +0xF8 UnInit. Detonation and all nested receiver work complete before conceal/removal.

### 5.2 Ordinary Bullet impact payload

| Planned field | Exact native provenance | Read/capture time |
|---|---|---|
| receiver target_id | each Apply_area_damage fixed-record target, converted without dedup | after area collection; never assumed to be Bullet+0x10C |
| source_object_id | Bullet+0xB0, initialized from firing Techno | retained to impact |
| source_house | source_object+0x21C if source remains non-null | read at impact, not frozen at launch |
| warhead_id | Bullet+0x128 from WeaponType+0xAC | retained to impact |
| incoming_damage | low signed 32-bit product of Bullet+0x150 and Bullet+0x6C, arithmetic shift right 8 | computed once at impact; no saturation |
| impact_coord | final stack-local signed Cartesian world-lepton CoordStruct after BulletDetonation adjustments | impact |
| ignore_defenses | false | each fixed-record receiver |
| area affect_resource | true | producer call constant; outside the current DTO but still required |

The native area producer has no target argument. A pre-collection adapter that copies the projectile tracking target into target_id is wrong.

### 5.3 Same-frame and delayed mechanism fixtures

With live order [attacker A, B, C], A fires and Reveal appends projectile P, producing [A, B, C, P]. B and C run before P. P then receives first AI in that same live pass. If its verified close-impact branch accepts the post-step distance, detonation and damage occur then. If not, P remains live and can impact on a later visit.

For the checked ROT close branch, the inclusive threshold is current speed × 0.5. With speed magnitude 4, returned distance 2 selects this impact branch while distance 3 does not, assuming no other branch overrides the decision. This is a branch fixture, not a universal stock latency table.

### 5.4 DiskLaser and Wave remain separate

| Effect | Storage and scheduler | Earliest damage after creation by a main-rung Techno | Evidence boundary | Status |
|---|---|---|---|---|
| DiskLaser | separate global DiskLaser array; AI rung before main Logic objects | next PerTick call, because its rung already ran | constructor, PerTick position, AI area call 0x004A76AF | timing VERIFIED; full field/RNG/damage adapter DEFERRED |
| Wave | separate Wave array; processing rung after main Logic objects | later in the same frame if construction succeeds and its splash condition is active | constructor, post-main rung, area calls 0x0053CDB5/0x0053CDD4 | timing VERIFIED; full field/RNG/damage adapter DEFERRED |

Laser, electric, rad-beam, particle, RocketLocomotion, and other special effect internals are not promoted into the ordinary Bullet contract. Their direct callsite identity may be classified in Section 6 while their complete scheduler/payload mechanism remains deferred.

## 6. All 33 Direct Apply_area_damage Xrefs

Fresh get_xrefs_to(0x00489280) returned exactly 33 direct calls. The table preserves Task 3A's reachability classifications. Excluded means excluded from this bounded producer contract, not unimportant to eventual whole-game parity.

| Direct callsite(s) | Count | Native owner / route | Active-YR class | Reconciliation status |
|---|---:|---|---|---|
| 0x00424ED1 | 1 | AnimClass::Middle, TiberiumChainReaction | conditional active | classified; special animation producer outside bounded 3B/3C |
| 0x0048A371 | 1 | Apply_area_damage explodable-overlay recursive tail | conditional active | dispatcher recursion verified |
| 0x0053A5D0 | 1 | LightningStorm::GroundStrike | conditional active standard YR | in-scope producer verified |
| 0x0053CDB5, 0x0053CDD4 | 2 | Wave_splash_forces base/deck paths | conditional active | scheduler position verified; effect-specific adapter deferred |
| 0x0053B16B | 1 | PsychicDominator::MindControlArea | conditional active standard YR | classified; excluded superweapon mechanism |
| 0x004387A3 | 1 | BombClass::Detonate | conditional active | classified; attached-bomb mechanism outside bounded 3C death helper |
| 0x006E04DD, 0x006E0545, 0x006E05AD, 0x006E062F, 0x006E0697 | 5 | TriggerAction case 0x3F helper, center plus four ±0x55 XY calls | conditional active | classified; trigger producer outside bounded 3C |
| 0x006E250B | 1 | TriggerAction case 0x2A weapon-selected strike | conditional active | classified; trigger producer outside bounded 3C |
| 0x006CD90C | 1 | SuperClass::Launch case 9, Genetic Mutator | conditional active standard YR | classified; excluded superweapon mechanism |
| 0x00469A83 | 1 | WarheadTypeClass::Detonate | active core | ordinary Bullet verified; death weapon and initial radiation converge here indirectly |
| 0x00425237 | 1 | NukeGroundZero::ApplyDamage | conditional active standard YR | classified; nuke orchestration outside bounded 3C |
| 0x00423EAB | 1 | AnimClass::AI bouncer/meteor impact | conditional active | classified; special animation producer deferred |
| 0x00424647 | 1 | AnimClass::AI per-frame damaging animation | conditional active | classified; special animation producer deferred |
| 0x0048A88B | 1 | orphan helper at 0x0048A700 | dormant/unreferenced | no code/data xref, caller, vtable binding, or export; historical ancestry unknown |
| 0x004A76AF | 1 | DiskLaserClass::AI | conditional active standard YR | scheduler position verified; full effect adapter deferred |
| 0x004CD9BB | 1 | FlyLocomotionClass::Process aircraft crash explosion | conditional active standard YR | classified; crash area call distinct from death-helper recursion |
| 0x006632C7 | 1 | RocketLocomotion::Detonate for V3ROCKET/DMISL/CMISL | conditional active standard YR | owner identity verified; exact special projectile lifecycle/payload deferred |
| 0x004B5D28, 0x004B5FC7 | 2 | DropPodLocomotion | DORMANT-TS in standard YR | excluded dormant legacy |
| 0x0051A6C1, 0x0051A79E, 0x0051A7D3 | 3 | InfantryClass::PerCellProcess C4 branches | conditional active standard YR | classified; C4 mechanism outside bounded 3C |
| 0x0071BABF | 1 | TerrainClass::Take_Damage lethal special branch | conditional active | classified; terrain receiver producer outside bounded 3C |
| 0x0074A1E1 | 1 | VoxelAnimClass::AI terminal impact | conditional active | classified; voxel-animation producer deferred |
| 0x00481E33, 0x00481E89 | 2 | Crate poison-gas center and neighbor hits | conditional; stock Powerups chance zero | classified; forced/map-selected crate route remains active-capable |
| 0x0048266D | 1 | crate Explosion randomized-offset hits | conditional; stock Powerups chance zero | classified; crate mechanism outside bounded 3C |
| 0x00482836 | 1 | crate Napalm handler | conditional; stock Powerups chance zero | classified; crate mechanism outside bounded 3C |
| **Total** | **33** |  |  | complete direct-xref inventory |

Periodic radiation is absent from this table because it calls the concrete Foot receiver directly. Death weapon is absent as a unique direct xref because its ephemeral Bullet converges through WarheadTypeClass::Detonate and the 0x00469A83 call.

## 7. Death-Weapon Contract

### 7.1 Reachability and selection

The helper at 0x0070D690 is reached by:

- active Techno lethal-receiver logic when Explodes, the veteran/elite EXPLODES ability, or current WeaponType Suicide gates the helper;
- active FlyLocomotion crash logic when health equals exactly zero;
- a TunnelLocomotion caller that is DORMANT-TS in standard YR.

An explicit DeathWeapon pointer is selection data, not by itself a reachability gate.

Selection order is:

1. TechnoType DeathWeapon when non-null;
2. virtual +0x3F weapon structure when both the structure and its WeaponType are non-null;
3. Rules default death weapon;
4. no detonation if the final weapon is null.

Explicit/virtual-fallback damage is Math__ftol(Weapon Damage × DeathWeaponDamageModifier). The Rules-default path is Math__ftol(type Strength × 0.5).

### 7.2 Ephemeral Bullet and recursion

The helper allocates a temporary Bullet using the dying object as both target and source/firer, initializes selected damage and warhead, stores the selected WeaponType, unlimbos, reads the dying object's exact coordinate, synchronously detonates, and destroys the temporary Bullet before returning. Allocation failure skips the detonation.

The temporary Bullet is not left for the live Logic scheduler. Its area packet is:

| Field | Native value |
|---|---|
| impact coordinate | dying object's exact vslot +0x48 CoordStruct |
| source object | dying object |
| source house | dying object's current owner read synchronously at detonation |
| warhead | selected death weapon warhead |
| damage | selected formula above; scalar 0x100 leaves it unchanged at area entry |
| affect_resource | true |
| receiver flags | false / false through the area dispatcher |

Passenger purge occurs before the death helper. The helper and every nested receiver finish before the outer lethal receiver continues to the attached-bomb check. If a nested receiver kills another eligible object, its death helper can recurse before the outer area record loop resumes. No global native recursion-depth clamp was found; inventing one would change behavior.

The helper itself consumes no RNG. Downstream detonation, area tail, receivers, and nested deaths own any draws.

## 8. Radiation Contract

### 8.1 Initial impact

At ordinary Bullet detonation, a selected WeaponType with RadLevel greater than zero creates or merges a RadSite before the ordinary 0x00469A83 area call:

1. convert exact impact XY to the center cell;
2. derive whole-cell spread from warhead CellSpread;
3. create and register a new 0x74-byte site, or merge into the existing center site;
4. perform ordinary Bullet area dispatch with ordinary Bullet source/house/warhead provenance.

Warhead detonation can consume up to two conditional RandomRanged draws before site creation. These are impact/warhead draws, not persistent RadSite RNG.

### 8.2 Persistent site state intentionally discards attribution

The verified 0x74-byte RadSite contains light ownership, two timer triples, center cell, spread, radius, activation level, decay/light step state, tint/light values, total duration, and remaining duration. It has no source object, source house, WeaponType, impact warhead, impact damage, or original exact impact coordinate field beyond center/spread.

The periodic consumer does not recover any attribution through the cell or light. Native periodic radiation intentionally discards it. Rust's lack of source/house/warhead fields in RadSite and RadDetonation is therefore not the mismatch.

### 8.3 Site order and same-pass visibility

The global driver processes existing RadSites in reverse starting-array order before the forward Logic object pass. A RadSite created later by a Bullet during that live-object pass has missed the current frame's site pass.

On a RadApplicationDelay frame:

- a Foot earlier than the creating Bullet in Logic order has already run and cannot observe the new deposit that frame;
- a Foot later than the Bullet can observe the deposit in its own AI;
- the next global site decay occurs on a later PerTick call.

Site create, merge, spread, decay, periodic HP block, and expiry contain no local RNG. Nested receiver/death behavior may consume shared Scenario RNG.

### 8.4 Exact periodic direct receiver

FootClass::AI requires alive membership, CurrentFrame modulo RadApplicationDelay equal to zero, not ImmuneToRadiation, not high flying, not in limbo, and a positive capped/truncated cell level.

Damage is two-stage:

1. level_i32 = Math__ftol(min(cell radiation double, Rules.RadLevelMax));
2. damage_i32 = Math__ftol(level_i32 × Rules.RadLevelFactor).

Fresh 0x004DA60F..0x004DA629 assembly maps the direct receiver packet to:

| Argument | Native periodic value |
|---|---|
| target | current Foot |
| incoming_damage | stack signed i32 from the two-stage formula |
| distance_leptons | 0 |
| warhead | current Rules+0x1834 RadSiteWarhead, read at receiver time |
| source object | null |
| ignore_defenses | false |
| arg6 | true |
| source house | null |

If the receiver clears IsAlive, Foot AI returns immediately. Buildings do not run this Foot path. Periodic radiation does not call Apply_area_damage.

## 9. Lightning Contract

### 9.1 Global ownership and delayed authority

LightningStorm::Start writes the global target cell and global owner pointer before checking whether a storm is already active or deferred. Later calls can therefore replace provenance observed by already-created tracked animations.

The owner and target globals, plus tracked cloud/bolt animation vectors, are persisted. Cleanup clears the owner global. Owner is not stored per bolt.

Process creates center/scatter clouds from global-frame modulo gates, then walks the damage-bearing animation vector in reverse. GroundStrike is called only when:

animation current frame > animation type total frames / 2

The comparison is strict. Damage is asset-frame delayed, not cloud-creation-time damage.

### 9.2 Exact GroundStrike packet

GroundStrike builds exact Cartesian world-lepton impact coordinates:

- X = cell X × 256 + 128;
- Y = cell Y × 256 + 128;
- Z = signed cell level × global level height, plus bridge height when the structural bridge bit is set.

Its area packet is:

| Field | Native value at the delayed strike |
|---|---|
| base damage | current Rules.LightningDamage |
| warhead | current Rules.LightningWarhead |
| source object | null |
| source house | current mutable global storm owner |
| affect_resource | true |
| receiver flags | false / false through Apply_area_damage |

Nested area receivers and death weapons complete before GroundStrike resumes.

### 9.3 Shared Scenario RNG sequence

The verified producer-local order is:

1. cloud creation rejects its special duplicate coordinate, then consumes one raw Next for WeatherConClouds;
2. delayed GroundStrike consumes one raw Next for WeatherConBolts and creates the visual before its duplicate early return;
3. a non-duplicate strike with sounds consumes one raw Next for LightningSounds even when count is one;
4. synchronous explosion, area transaction, receiver work, and nested draws;
5. only if the post-damage predicate requests scorch, RandomRanged(2,4), then one scorch-animation range draw per spawned scorch.

Scatter attempts consume X then Y inclusive RandomRanged draws for each of at most three attempts. Rejected attempts still consume both draws; there is no fourth fallback candidate.

## 10. Cross-Producer Ownership Matrix

| Producer | Scheduler/call-stack owner | Damage call | Provenance lifetime | Local RNG | Lifecycle completion |
|---|---|---|---|---|---|
| ordinary Bullet | live forward Logic vector | BulletDetonation → Warhead detonation → area dispatcher | source/warhead/damage scalar retained in Bullet; house read at impact; receiver target created by area records | trajectory-specific inventory remains outside this synthesis; downstream shared draws retain exact call position | detonate fully, then UnInit/conceal compact-removes Bullet |
| DiskLaser | separate pre-main global rung | direct area call in DiskLaser AI | effect-owned, not ordinary Bullet-owned | deferred | separate object/array lifecycle; full adapter deferred |
| Wave | separate post-main global rung | Wave splash area calls | effect-owned, not ordinary Bullet-owned | deferred | separate Wave array lifecycle; full adapter deferred |
| death weapon | current lethal receiver/crash stack | ephemeral Bullet detonation → area | dying object and selected weapon exist through nested call only | none in helper | temporary Bullet destroyed before helper returns |
| radiation initial | ordinary Bullet detonation | ordinary area call after site create/merge | ordinary Bullet provenance for initial hit | up to two conditional pre-site warhead draws | normal Bullet lifecycle |
| radiation periodic | current Foot AI after RadSite pass | direct receiver, no area | no attacker/house/original-warhead provenance; current Rules only | none locally | direct receiver returns to same Foot AI or exits if dead |
| lightning | global tracked-animation process before RadSites/Logic | delayed GroundStrike → area | coordinate in tracked animation; owner global mutable; damage/warhead read from current Rules | shared Scenario stream in exact sequence | tracking entry removed after strike; area recursion completes first |

These producers require distinct Rust-native owners. A single late damage-event queue cannot reproduce their order, state reads, RNG visibility, or nested lifecycle.

## 11. Focused Current-Rust Comparison

### 11.1 Substrate worth preserving

- src/sim/world/logic_vector.rs tail-appends, compact-removes without swap, preserves insertion order, serializes it verbatim, and src/sim/world/mod.rs:1015 provides a live-length iterator with native append/skip semantics.
- src/sim/combat/cell_spread.rs preserves the exact count table, 369 offsets, and band-11 duplicate.
- occupancy ordering preserves the useful non-structure-prepend/structure-append class.
- current RadSite/RadDetonation correctly omit native-nonexistent attacker, house, and original-warhead provenance.

These are useful pieces, not parity certification.

### 11.2 Verified drifts

| Native mechanism | Current Rust evidence | Verdict |
|---|---|---|
| CellSpread zero scans center and may hit exact-distance-zero target | combat_aoe.rs:93-95 returns empty | DRIFT |
| airborne candidates precede table/list records in native spatial order | combat_aoe.rs scans cells first, then EntityStore values | DRIFT |
| no dedup and repeated records remain repeated | combat_aoe.rs uses BTreeSet | DRIFT |
| exact signed X/Y/Z impact and native Sqrt_Approx/ftol lepton distance | API takes coarse cells/Z, centers impact, ignores Z in mobile distance, and uses exact integer sqrt divided to cells | DRIFT |
| fresh signed i32 raw receiver packet | AoE precomputes u16 Verses/prone damage and later applies a batch | DRIFT |
| live-state reread and nested receiver effects between records | damage_events are applied later with u16 saturating subtraction | DRIFT |
| Fire_At creates a persistent Bullet; live Logic AI owns impact | combat/mod.rs:2375-2505 creates damage and RadDetonation at fire time | DRIFT |
| same-pass appended Bullet AI | world/mod.rs:2119-2173 runs rocket/homing from a pre-combat snapshot; combat itself uses a snapshot at 2337 | DRIFT |
| projectile detonation drives damage/removal | returned rocket/homing detonation IDs are ignored | DRIFT |
| complete persistent Bullet payload and final world-lepton impact coordinate | HomingState and RocketState contain trajectory/tracking state but not source, warhead, signed damage/scalar, or native impact packet | DRIFT |
| projectile persistence/hash is symmetric and complete | world_hash hashes HomingState but has no RocketState hash path | DRIFT |
| death helper is synchronous, gated, recursive, and uses native selection/formulas | combat/mod.rs selects/collects death AoE after batched deaths, falls back differently, then directly subtracts HP | DRIFT |
| reverse RadSite pass before per-Foot direct receiver | world runs tick_decay after combat; combat folds all detonations then batches all victims | DRIFT |
| periodic radiation uses current Foot position and arg6=true direct packet | Rust pre-applies Verses, converts to u16, and queues generic damage_events | DRIFT |
| native site order is reverse insertion/global-vector order | RadiationState sites are BTreeMap key order | DRIFT |
| lightning waits for tracked animation and reads mutable globals/current Rules | lightning_storm.rs spawn_bolt immediately applies HP | DRIFT |
| lightning uses shared Scenario RNG, Rules animation lists, half-spread, all active clouds, and three attempts | Rust uses private superweapon_rng, hardcoded animations, full spread, last coordinate, ten attempts plus fallback | DRIFT |
| native lightning ownership is mutable globals/tracked animations | Rust stores active and queued owner/target records and hashes that different shape | DRIFT |

The comment at world/mod.rs:2356-2358 saying native RadSite updates occur after the object loop is contradicted by fresh binary assembly and must not guide implementation.

## 12. Rust-Facing Implementation Handoff

This is a behavior handoff, not a Rust patch or architecture decision.

| Verified behavior | Evidence | Current Rust delta | Affected surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| area collection emits ordered identity+i32-lepton records | Task 3A; Sections 4.2-4.5 | coarse Vec of final u16 damage | combat_aoe.rs and future damage area boundary | preserve air-first, exact table/list order, repeats, captured distance, and live-state dispatch | center-zero, air-before-ground, band-11 duplicate, moved/removed later target | do not dedup, sort, or precompute final damage |
| exact signed world-lepton conversion and native distance | 0x00489309..0x00489330; distance helpers | cell-centered integer-sqrt path | shared WorldLeptonCoord and area distance surface | preserve signed conversion, X/Y/Z, Sqrt_Approx/ftol bits, Building/Aircraft adjustments | negative coordinate and one-lepton boundary fixtures | do not use host sqrt, screen axes, or terrain level as Z |
| ordinary Bullet reveal joins the live current pass | 0x00468684; 0x005F5038..40; 0x0055B608..619 | movement/combat snapshots | logic_vector.rs, world scheduler, projectile owner chosen by separate design | create, reveal, AI, detonate, and remove within one live-length authority | [A,B,C] appends P; P first AI after B,C; compact removal skips shifted successor | do not add a fixed pre-combat projectile phase |
| Bullet impact payload is retained and source house is read at impact | Task 3B Sections 4, 7, 8 | payload absent from HomingState/RocketState | G2-owned projectile persistence and hash surfaces | retain exact source, warhead, damage/scalar, final CoordStruct, detach semantics; form target IDs only after area capture | source owner changes between launch/impact; tracked target differs from splash receivers | do not freeze source house at launch or equate tracking target with receiver target |
| DiskLaser and Wave use distinct rungs | Task 3B Section 9 | no verified separate adapters | separate effect owners | keep pre-main next-PerTick DiskLaser and post-main same-frame Wave positions | create both during main rung and compare earliest receiver frame | do not route both through ordinary Bullet timing |
| death weapons recurse synchronously with dying object as source | 0x0070D690; Task 3C Section 4 | deferred death AoE batch | lethal receiver/death-transition service | run selection/formula, ephemeral detonation, nested receivers, and destroy before outer receiver resumes | A death weapon kills B whose death weapon kills C; verify order and source houses | do not queue to an end-of-tick death phase |
| existing RadSites update before Foot reads; periodic call is direct false/true with null attribution | 0x0055B5CD..619; 0x004DA60F..629 | post-combat sorted-site decay and batched generic events | radiation.rs, world scheduler, Foot AI-equivalent receiver surface | reverse site order, same-pass visibility, current Rules warhead, two-stage damage, exact packet | Foot before and after creating Bullet on application-delay frame | do not invent attacker/house provenance or use area dispatch |
| lightning is delayed tracked-animation damage with mutable global owner/current Rules | Task 3C Section 6 | immediate spawn_bolt HP/private RNG/different state | lightning_storm.rs, effect tracking, shared RNG, persistence/hash | preserve strict half-frame gate, global reads, coordinate/Z, RNG sequence, and synchronous area call | overwrite owner after cloud creation but before strike; duplicate bolt visual with no damage | do not freeze owner per bolt or damage at cloud creation |
| all producer-local and nested RNG stays in native order | Task 3A late tail; Task 3C RNG ledgers | private/batched/relocated draws | shared Scenario RNG and synchronous executor | consume at exact producer/receiver positions with rejected-attempt receipts | lightning reject attempts, duplicate strike, bridge tail, nested death draws | do not allocate private producer streams |
| persistent state and hashes represent the chosen native-equivalent owners | Task 3B/3C save/hash evidence | incomplete/asymmetric projectile hash and different radiation/lightning state shape | save/load/world_hash plus approved G2/GS designs | serialize/hash every persistent authoritative field while preserving iteration order and same-tick visibility | save just before Bullet impact, RadSite decay, and lightning threshold; resume identical order | do not treat Rust-vs-Rust hash equality as gamemd parity |

Highest-leverage next dependency: run and approve a separate projectile-impact scheduling brainstorm/design, then produce and implement its dedicated plan. This report deliberately does not choose the Rust projectile storage type.

## 13. Contradiction and Correction Ledger

| Earlier or current claim | Reconciled result | Classification |
|---|---|---|
| Fire_At directly or instantly mutates HP for ordinary weapon routes | Fire_At creates effects/munitions; ordinary Bullet HP occurs from later Bullet AI impact | WRONG at the fire-call boundary |
| all projectile damage waits until the next tick/frame | same-pass Bullet first AI and same-frame Wave damage are possible; DiskLaser waits because its rung already passed | WRONG universal timing |
| area targets are distance-sorted | order is airborne spatial enumeration, then CellSpread table, then selected cell list | WRONG |
| area targets are generally cell-centered | mobiles/air use exact object coordinates; Building branches use cell centers | WRONG / MISLEADING |
| CellSpread band is simply ftol(CellSpread) | table count uses ftol(CellSpread + 0.99); airborne query separately uses ftol(CellSpread) | WRONG |
| CellSpread zero produces no targets | center cell is scanned and exact-distance-zero candidates can pass | WRONG |
| deduplication is harmless or native | no native dedup; band 11 deliberately repeats one cell in the shipped table | WRONG |
| Rules +0xB40/+0xB4C is a generic ProtectedFromAOE list | it is HarvesterUnit under the HarvesterImmune scenario gate | WRONG label |
| bridge selector intermediate is 2×H because the initializer multiplies by 0.5 | initializer makes 4×H and the selector halves it to 2×H | WRONG decode |
| FUN_00663030 is DiskLaser | direct owner/callers identify RocketLocomotion::Detonate | WRONG label |
| earlier receiver removal immediately frees later raw area records | standard UnInit clears alive and defers deletion, so later pointer remains valid and skips | RESOLVED stale uncertainty |
| source house for ordinary Bullet is frozen at launch | it is read from the retained source object at impact | WRONG |
| receiver target_id is the projectile tracking target | it comes from each ordered area record | WRONG boundary |
| RadSiteClass helper 0x0065BD00 damages units | it decreases cell radiation; FootClass::AI owns periodic HP | WRONG label inference |
| periodic radiation retains attacker/house/original warhead | layout and direct receiver prove null source/house and current global RadSiteWarhead | WRONG |
| RadSite pass is after the object loop | it is before the forward Logic loop | WRONG; current Rust comment is stale |
| DeathWeapon definition alone causes every death explosion | helper reachability still needs the verified lethal gate or crash caller | WRONG |
| death source is the original killer | source is the dying object and its current owner | WRONG |
| lightning damages when the bolt/cloud is spawned | damage waits until tracked animation current frame is strictly greater than half total frames | WRONG |
| lightning owner is frozen per bolt | GroundStrike reads the mutable global owner at damage time | WRONG |
| lightning uses a private RNG stream | verified producer draws use the shared Scenario RNG | WRONG |

## 14. Gate Consequences

### 14.1 G1

Task 3's defined G1 slice is resolved:

- all 33 direct area-dispatch xrefs are inventoried and reachability-classified;
- area record order, filters, signed coordinates, distance, lifetime, receiver packet, and non-HP relative order are fixed;
- ordinary Bullet, death weapon, radiation initial/periodic, and lightning have verified call points, scheduler positions, and argument provenance;
- DiskLaser/Wave timing is fixed without pretending their full effect adapters were investigated.

Global G1 remains FAILED. Task 1S explicitly leaves authority-critical receiver semantics and mutable-field provenance open. Task 2S explicitly leaves Infantry presentation/helpers, Building helper/effect internals, and raw persisted Building+0x52C state open. Those gaps remain shadow-only blockers.

### 14.2 G2

G2 remains FAILED / BLOCKED. Evidence alone does not satisfy the plan's pass condition. Rust needs an approved and implemented projectile lifecycle that:

- owns ordinary projectile creation and complete persistent payload;
- reveals into the unified live Logic order;
- runs first and later AI visits at the verified live scheduler position;
- produces the final exact world-lepton impact coordinate;
- synchronously enters area/receiver work;
- performs native-order uninit/conceal and cursor consequences;
- serializes and hashes every persistent authoritative field.

The existing for_each_live_object primitive is suitable infrastructure, but production combat does not use it as this owner.

### 14.3 G3

G3 remains FAILED / NOT RUN. This static report cannot provide:

- raw numeric input/output bits from retail execution;
- ordered collector and receiver observations;
- RNG cursor receipts;
- frame cursor positions;
- live-vector/RadSite/tracked-animation membership receipts;
- lifecycle termination receipts;
- zero Rust-versus-retail mismatches.

Task 4 must obtain an accepted Oracle handoff and later executable comparison. No static COMPLETE label in this report can replace that gate.

## 15. Coverage Ledger

| Area | Status | Evidence | What remains |
|---|---|---|---|
| Apply_area_damage entry, early returns, radii | verified | Task 3A; 0x00489280 | runtime trace only |
| air/table/list collection order | verified | Task 3A | runtime ordered observations |
| all collector/dispatch filters | verified | Task 3A | runtime ordered observations |
| coordinate frames and target-specific distances | verified | Task 3A worked fixtures | executable retail fixture |
| fixed record and standard prior-removal lifetime | verified | Task 3A lifecycle chain | nonstandard memory corruption is outside parity scope |
| exact area receiver packet | verified | Task 3A plus fresh 0x00489A78..AC3 | retail packet capture |
| pre-cell and post-receiver non-HP relative order | verified | Task 3A | inner mechanisms remain owned by their focused systems |
| direct xref inventory | verified, 33/33 | fresh get_xrefs_to plus Task 3A classification | excluded active routes need their own future producer reports before implementation |
| ordinary Bullet creation, live scheduling, impact packet, removal | verified for static implementation planning | Task 3B | G2 design/implementation and retail trace |
| exact latency for every stock projectile | deferred | Task 3B | per-projectile trajectory/target-state investigations |
| DiskLaser/Wave relative scheduler positions | verified | Task 3B | full effect payload, RNG, lifecycle, and adapter mechanisms deferred |
| other laser/electric/rad-beam/particle/RocketLocomotion internals | deferred | xref/callee identity only | separate bounded producer investigations |
| death-weapon named contract | verified | Task 3C | retail nested-recursion trace |
| radiation initial and periodic named contracts | verified | Task 3C plus fresh periodic packet check | broader whole-map CRC path deferred; retail same-pass trace |
| lightning named contract | verified | Task 3C | retail RNG/frame/membership trace |
| focused current Rust state | verified DRIFT | direct code reads in Section 11 | design and implementation |
| global G1 | deferred / blocked | Task 1S and Task 2S | close their named authority gaps |
| G2 | deferred / blocked | Task 3B and Section 14.2 | approved implemented projectile plan |
| G3 | deferred / blocked | no Oracle execution | accepted retail capture and zero mismatches |

## 16. Open-Question Ledger — Final State

- [RESOLVED] OQ-3S-01 — What is the area capture boundary? → All air/table/list collection finishes before receiver dispatch; each record freezes target identity plus signed i32 lepton distance. (evidence: Task 3A Sections 4 and 6)
- [RESOLVED] OQ-3S-02 — What is exact target order? → Air spatial order, then CellSpread initializer order, then selected CellClass list head-to-tail. (evidence: Task 3A Sections 4.1-4.3)
- [RESOLVED] OQ-3S-03 — Does native area damage deduplicate? → No; repeated records remain repeated, including the band-11 duplicate cell. (evidence: Task 3A Section 4.5)
- [RESOLVED] OQ-3S-04 — What remains live after capture? → Health, alive/limbo flags, +0x74, type-visible state, owner, and invulnerability are re-read; identity/distance stay fixed. (evidence: Task 3A Sections 5-6)
- [RESOLVED] OQ-3S-05 — Can an earlier receiver free a later record's object inline? → Standard UnInit clears alive and defers allocation deletion, so the later pointer remains valid and skips. (evidence: Task 3A Section 6)
- [RESOLVED] OQ-3S-06 — Does Fire_At apply ordinary HP? → No; ordinary damage is owned by later Bullet AI detonation. (evidence: Task 3B Sections 1 and 4)
- [RESOLVED] OQ-3S-07 — Can an ordinary Bullet damage in the firing frame? → Yes, if tail-appended first AI reaches impact later in the same live-count pass; not every shot does. (evidence: Task 3B Sections 5-6; fresh 0x0055B608..619)
- [RESOLVED] OQ-3S-08 — Are DiskLaser and Wave ordinary Bullet timing variants? → No; they use separate pre-main and post-main rungs respectively. (evidence: Task 3B Section 9)
- [RESOLVED] OQ-3S-09 — What is receiver target_id provenance? → Each ordered area fixed record, not the projectile's tracked target. (evidence: Task 3B Section 8)
- [RESOLVED] OQ-3S-10 — Is ordinary source house launch-captured? → No; it is read from the retained source object at impact. (evidence: Task 3B Section 7.2)
- [RESOLVED] OQ-3S-11 — Does a death weapon enter the normal projectile scheduler? → No; it detonates and destroys an ephemeral Bullet synchronously in the lethal call stack. (evidence: Task 3C Section 4)
- [RESOLVED] OQ-3S-12 — What is death-weapon source provenance? → Dying object and its current owner. (evidence: Task 3C Section 4.3)
- [RESOLVED] OQ-3S-13 — Does periodic radiation retain impact attribution or call area damage? → No to both; it calls the Foot receiver directly with null source/house and current Rules warhead. (evidence: Task 3C Sections 5.2 and 5.5)
- [RESOLVED] OQ-3S-14 — What is periodic radiation arg6? → true, while ignore_defenses is false. (evidence: fresh 0x004DA60F..629)
- [RESOLVED] OQ-3S-15 — What runs first, RadSite decay or Foot periodic damage? → Reverse RadSite pass first, then forward Logic/Foot AI. (evidence: fresh 0x0055B5CD..619)
- [RESOLVED] OQ-3S-16 — When and with whose ownership does lightning damage? → After the strict tracked-animation half-frame threshold, using current mutable global owner and current Rules damage/warhead. (evidence: Task 3C Section 6)
- [RESOLVED] OQ-3S-17 — Are all direct dispatcher xrefs accounted for? → Yes, 33/33 are listed and classified in Section 6. (evidence: fresh get_xrefs_to(0x00489280))
- [DEFERRED] OQ-3S-18 — What are exact latency tables for every stock Bullet trajectory? (category: out-of-scope; reason: ordinary scheduler/adapter is proved, but every trajectory family was not the bounded 3B slice; next-step-if-pursued: investigate one projectile family at a time with exact target-state fixtures)
- [DEFERRED] OQ-3S-19 — What are the complete DiskLaser and Wave payload, RNG, effect, and lifecycle adapters? (category: requires-different-system-context; reason: 3B proved only their scheduler positions; next-step-if-pursued: run separate bounded producer investigations)
- [DEFERRED] OQ-3S-20 — What are the complete RocketLocomotion, laser, electric, rad-beam, particle, Anim, crate, C4, nuke, trigger, and voxel producer mechanisms? (category: out-of-scope; reason: direct callsite identity/reachability was sufficient for Task 3S classification but not implementation authority; next-step-if-pursued: investigate each chosen active route from its owner)
- [DEFERRED] OQ-3S-21 — What was orphan 0x0048A700 historically intended for? (category: bounded-cost-too-high; reason: it has no current code/data xref, caller, vtable binding, or export, so ancestry does not affect active-YR reachability; next-step-if-pursued: compare historical binaries or symbols)
- [DEFERRED] OQ-3S-22 — What exactly occurs for a modded spread band above 11? (category: out-of-scope; reason: native indexes beyond the fixed table with no clamp and defines no safe compatible extension; next-step-if-pursued: use a controlled runtime fault/read trace)
- [DEFERRED] OQ-3S-23 — Which exact Rust type owns the persistent ordinary projectile payload? (category: requires-different-system-context; reason: the required separate G2 design has not been approved or implemented; next-step-if-pursued: brainstorm projectile impact scheduling, approve the design, then write its plan)
- [DEFERRED] OQ-3S-24 — Which exact Cell/map fields enter the complete native sync CRC? (category: requires-different-system-context; reason: Task 3C proved RadSite virtual ownership but not the whole map checksum; next-step-if-pursued: run a bounded map/Cell checksum investigation)
- [DEFERRED] OQ-3S-25 — Can global G1 pass? (category: requires-different-system-context; reason: Task 1S and Task 2S retain named authority gaps outside Task 3; next-step-if-pursued: close and reconcile those exact blocker rows)
- [DEFERRED] OQ-3S-26 — Does executable retail evidence match every static sequence? (category: needs-runtime-debugger; reason: game/debugger/Oracle execution was prohibited here; next-step-if-pursued: execute the accepted Task 4 Oracle contract and compare ordered raw observations)

There are no unresolved OPEN entries. Deferred items are explicit implementation, adjacent-producer, broader-checksum, or runtime-certification work; they are why the overall authority status remains PARTIAL.

## 17. Adversarial Review

1. **Does same-frame Bullet eligibility mean damage occurs before every pre-existing later attacker?** No. Tail append places the Bullet after all entries present at insertion; those entries run first.
2. **Could a later area record use a moved target's new distance?** No. Its state gates are live, but its signed distance was captured before any receiver call.
3. **Could native list invariants make dedup safe?** No. The verified band-11 duplicate repeats a whole cell, and the dispatcher contains no target set.
4. **Could a death weapon be delayed because it uses a Bullet object?** No. Its helper immediately detonates and destroys the temporary Bullet before returning.
5. **Could periodic radiation infer the attacker through the site's light or center cell?** No. The direct consumer pushes null source and null house and reads the current global RadSiteWarhead.
6. **Could the radiation PUSH 1 be source-house truthiness?** No. Seven-argument right-to-left ABI mapping places the first zero at source house and the following one at arg6.
7. **Could a lightning animation freeze owner internally?** GroundStrike is called with its coordinate and separately loads the mutable global owner immediately before area dispatch.
8. **Could Wave and DiskLaser share the Bullet adapter because all reach Apply_area_damage?** No. Converging on the dispatcher does not erase their distinct storage, scheduler rungs, and earliest-frame behavior.
9. **Does classifying an xref verify its whole producer?** No. Section 6 distinguishes route identity/reachability from an implementation-ready producer mechanism.
10. **Can a deterministic Rust batch be considered internal-only?** No. Batching changes same-call receiver recursion, live-state rereads, RNG order, membership, retaliation, and same-frame visibility.

## 18. Zero-Add Pass and Cold Spot-Checks

### 18.1 Reconciliation zero-add pass

After drafting the producer, xref, contradiction, gate, coverage, and open-question ledgers, the three child reports were cross-read again at their verdict, timing, argument-provenance, coverage, and final open-question sections. The 33-callsite count was recomputed from the fresh xref output. No new in-scope route, call field, ordering edge, contradiction, or unclassified direct callsite was added.

The zero-add result applies to the bounded Task 3S synthesis. It does not convert explicitly deferred adjacent special producers into exhausted investigations.

### 18.2 Cold spot-check A — shared scheduler

Fresh disassembly 0x0055B5C8 calls LightningStorm::Process, 0x0055B5CD..0x0055B5E8 iterates RadSites in reverse from starting count minus one, and 0x0055B608..0x0055B619 iterates Logic forward while reloading count at 0x0055B613. This independently reconfirms the shared timing spine.

### 18.3 Cold spot-check B — periodic radiation

Fresh 0x004DA60F..0x004DA629 disassembly pushes zero source house, one arg6, zero ignore flag, zero source, Rules+0x1834 warhead, zero distance, and the damage address before vtable +0x16C. The post-call alive check remains at 0x004DA62F.

### 18.4 Cold spot-check C — area receiver

Fresh 0x00489A79..0x00489A95 reconfirms positive health, +0x74, non-limbo, and inclusive maximum-distance gates. Fresh 0x00489AA7..0x00489AB6 reconfirms source house plus false/false/source/warhead/distance/fresh-damage packet.

## 19. Source and Citation Ledger

### 19.1 Primary current-session binary reads

- get_xrefs_to Apply_area_damage at 0x00489280;
- disassemble_bytes 0x0055B5BE..0x0055B623;
- disassemble_bytes 0x004DA60F..0x004DA634;
- disassemble_bytes 0x00489A78..0x00489AC3.

### 19.2 Binary evidence inherited from the fully read child reports

- Apply_area_damage at 0x00489280;
- airborne helpers 0x00412B40 and 0x004137A0;
- distance helpers 0x0041C380 and 0x004CAC40;
- CellSpread initializer 0x00561910;
- ObjectClass::UnInit at 0x005F65F0 and pending-delete drain 0x00725C70;
- TechnoClass::Fire_At at 0x006FDD50;
- BulletClass Init 0x004664C0, AI 0x004666E0, Fire 0x00468670, BulletDetonation 0x00468D80;
- WarheadTypeClass::Detonate at 0x004690B0;
- ObjectClass Reveal 0x005F4EC0, Conceal 0x005F4D30, UnInit 0x005F65F0;
- LogicClass::PerTickUpdate at 0x0055AFB0;
- DiskLaserClass::AI at 0x004A7340;
- Wave_splash_forces at 0x0053CBE0;
- death-weapon helper at 0x0070D690;
- RadSite constructor/setters/AI/save/load/destructor in the 0x0065B1E0..0x0065BD00 group;
- FootClass::AI periodic block at 0x004DA530;
- LightningStorm Start 0x00539EB0, CreateCloudBolt 0x0053A140, GroundStrike 0x0053A300, and Process 0x0053A6C0.

Every inherited claim retains the confidence and deferral boundary of its child report.

### 19.3 Documents

- docs/research/DAMAGE_AREA_DISPATCH_REINVESTIGATION_2026-07-13.md;
- docs/research/DAMAGE_PROJECTILE_IMPACT_TIMING_REINVESTIGATION_2026-07-13.md;
- docs/research/DAMAGE_SPECIAL_PRODUCER_TIMING_REINVESTIGATION_2026-07-13.md;
- docs/plans/2026-07-13-damage-authoritative-cutover-plan.md;
- docs/research/DAMAGE_RECEIVER_CORE_REINVESTIGATION_2026-07-13.md;
- docs/research/DAMAGE_CONCRETE_RECEIVER_REINVESTIGATION_2026-07-13.md.

### 19.4 Current Rust

- src/sim/combat/combat_aoe.rs;
- src/sim/combat/cell_spread.rs;
- src/sim/combat/mod.rs;
- src/sim/world/logic_vector.rs;
- src/sim/world/mod.rs;
- src/sim/movement/homing_movement.rs;
- src/sim/movement/rocket_movement.rs;
- src/sim/radiation.rs;
- src/sim/superweapon/lightning_storm.rs;
- src/sim/world/world_hash.rs.

## 20. Final Status

**Task 3S synthesis: COMPLETE.**  
**Task 3 bounded area/named-producer evidence: resolved for static implementation planning.**  
**Overall evidence authority: PARTIAL.**  
**Global G1: FAILED.**  
**G2: FAILED / BLOCKED.**  
**G3: FAILED / NOT RUN.**  

The next safe use of this report is as the Task 3 input to the plan refresh and to the separate projectile-impact scheduling design. It is not permission to patch the live Rust damage authority.
