# Shared Techno Constructor Scenario-RNG Implementation Contract

**Date:** 2026-08-28
**Status:** READY_FOR_PLAN
**Owning slice:** Active-retail bridge parity, P0 shared prerequisite
**Implementation authority:** active-retail `gamemd.exe` and Yuri's Revenge retail data only

## Exact parity gap

Every active `TechnoClass` construction consumes one raw `ScenarioClass` RNG value and stores its low word as persistent per-object state before placement can succeed or fail, but Rust currently has neither that state nor a construction-mode boundary and therefore cannot preserve the native cursor, failure, generated-object, upgrade, save/load, or later bridge-report-selection semantics.

## Scope

This contract closes the shared constructor prerequisite needed before bridge mechanisms can be implemented and criticized independently. It covers:

- fixed-map authored techno objects in retail section order;
- authored structure upgrades as distinct techno objects linked to their parent and slot;
- PostMap starting MCV and extra starting-unit construction;
- all active runtime placed and limbo techno construction paths;
- generated-map objects whose constructor state was already consumed by the generation cursor;
- save/load, deterministic hashing, and direct-construction enforcement.

It does not implement bridge damage, repair, collapse, destruction, rendering, report audio, or the generated-map builder itself. Those remain later bridge-owned mechanisms. It also does not import behavior from OpenTS; OpenTS is navigation evidence only.

## Evidence baseline

| Evidence | Role | Authority used here |
|---|---|---|
| `docs/research/bridges/00-system-models/RMG_BRIDGE_DUAL_RNG_LIFECYCLE_REINVESTIGATION_GHIDRA_REPORT.md` at `a776f270` | Primary research handoff | Direct active-retail constructor, load, PostMap, later-read, and Rust-ingress evidence |
| `TechnoClass` constructor `0x6F2B90`, raw draw/store at `0x6F3254` | Primary native | One raw scenario draw; low word stored at `TechnoClass + 0x3C8` |
| `ScenarioClass__Read_INI @ 0x6864B0` and concrete fixed-section readers | Primary native | Fixed authored construction order and pre-constructor rejection boundaries |
| `BuildingClass__ReadFromINI @ 0x44F820`, upgrade loop `0x44FD50..0x44FDC3` | Primary native | Valid upgrade slots construct separate `BuildingClass` objects and call virtual `+0xD8` (Unlimbo) at the parent location |
| `ScenarioClass__Post_Map_Init @ 0x686890`, `MultiplayerGameMode__Generate_Starting_Units @ 0x5D6D80`, callbacks `0x5D7030` and `0x5D70F0` | Primary native | Starting-object type selection, one construction before placement attempts, failure deletion, and extra-unit path |
| `DiskLaserClass__AI @ 0x4A7340` and `TechnoClassFireAtSpawnsBullet @ 0x6FDD50` | Primary native | Persistent constructor word is later read for report selection |
| `src/map/entities.rs`, `src/sim/runtime.rs`, `src/sim/world/world_spawn.rs`, `src/sim/world/world_hash.rs`, `src/sim/snapshot.rs` at `dfe44984` | Primary Rust | Current ownership, missing fields/modes, construction funnels, hash, and snapshot behavior |
| `docs/plans/2026-08-28-active-retail-bridge-parity-design.md` revision 7 at `dfe44984` | Derived design | Approved Rust architecture and mechanism partition |
| `C:\Users\enok\Documents\OpenTS` | Navigation only | Function/mechanism discovery aid; no parity conclusion or porting authority |

No material requirement below relies on OpenTS, an unaudited ownership row, or an uncited compatibility assumption.

## Required behavior and implementation deltas

| ID | Evidence class | Delivery class | Mechanism/result | Active-retail requirement | Current Rust state | Required Rust delta | Evidence | Acceptance |
|---|---|---|---|---|---|---|---|---|
| P0-01 | REQUIRED_FIX | MILESTONE | Persistent constructor word | Every active techno construction performs exactly one raw `ScenarioClass` RNG draw and stores `draw & 0xFFFF` before placement. The value survives for the object's lifetime. | `GameEntity` has no equivalent field; `new_at_frame` performs no scenario draw. | Add a serialized `u16` constructor/report-selection word to `GameEntity`. Populate it only through an explicit construction-init mode. Never derive it from stable ID, frame, or a second RNG. | `0x6F2B90`, draw/store `0x6F3254`; later reads `0x4A76CC`, `0x6FF36B`. | Raw-sequence tests prove one low-word capture per fresh object and persistent equality after later simulation work. |
| P0-02 | REQUIRED_FIX | MILESTONE | Fixed authored object order and rejection boundary | After the earlier scenario `Mark` draw, authored technos construct in retail section order: `Units`, `Aircraft`, `Infantry`, `Structures`. Rows rejected before constructor entry consume no constructor draw. Once construction begins, later reveal/Unlimbo failure does not refund the draw. | Parser preserves section/key insertion order, but projection has no constructor draw or explicit pre-constructor validation contract. | Keep the fixed order. Resolve and validate row, type, and owner before construction. Then construct once with `FreshScenario`; later placement failure may remove the object but must leave the RNG advanced. | Fixed-section reader trace in the cited research report; current parser order `src/map/entities.rs`; scenario bootstrap order `src/sim/runtime.rs`. | A mixed-section fixture with invalid pre-constructor rows and a post-constructor placement failure matches an independently stepped raw RNG sequence and final cursor. |
| P0-03 | REQUIRED_FIX | COMPOUNDING | Authored structure upgrades | Each valid authored upgrade slot constructs a distinct `BuildingClass`, consumes its own constructor draw, and is Unlimboed at the base building location. It remains a distinct live techno associated with the parent/slot; parent occupancy must not reject it as an ordinary independent structure footprint. Invalid or empty slots that never enter the constructor consume nothing. | Structure upgrade keys are intentionally not parsed; no upgrade techno or parent/slot identity exists. | Parse retail upgrade slots in authored order. For each valid resolved slot, construct a distinct `GameEntity` with `FreshScenario`, a stable ID, and `StructureUpgradeLink { parent_stable_id, slot }`. Reveal it through a dedicated attached-upgrade Unlimbo path at the parent location. Do not register a competing blocking footprint. Serialize and hash the link. | `BuildingClass__ReadFromINI @ 0x44F820`; loop `0x44FD50..0x44FDC3`; constructor `0x43B740`; virtual Unlimbo call at `0x44FDB9`. | Upgrade fixture proves base draw then one draw per valid slot, distinct stable IDs, correct parent/slot links, non-limbo/live lifecycle, same parent location, and no parent-footprint collision rejection. |
| P0-04 | REQUIRED_FIX | MILESTONE | PostMap construct-once/place-many | A starting MCV or chosen extra unit is constructed once before exact/fallback placement attempts. Its constructor draw precedes placement-search RNG. Total placement failure deletes that same object without refund; fallback attempts never reconstruct it. | `place_starting_object_near_base` searches for a cell before calling `spawn_object`, so a future draw inside `spawn_object` would occur too late and disappear on total failure. | Split limbo construction from placement. Construct one techno with `FreshScenario`, then try exact/fallback placement on that existing object. Delete on total failure without rewinding. Extra-unit type/candidate selection remains before its one constructor draw. | Start-base callback `0x5D7030` and extra callback `0x5D70F0..0x5D7498`; factory call `0x5D7393`; cited PostMap trace. | Direct-success, fallback-success, and total-failure fixtures prove one construction draw and native cursor order; total failure leaves no object but retains the draw. Extra-unit fixture proves candidate selection -> constructor -> placement-search ordering. |
| P0-05 | REQUIRED_FIX | MILESTONE | Runtime placed and limbo construction | Each active runtime techno constructor consumes exactly one scenario draw whether the object starts limboed, places successfully, or fails after construction. | Production ingress converges on three `world_spawn` constructor sites, all lacking explicit init/RNG. | Make the world spawn funnels accept a scenario RNG plus `TechnoConstructorInit`, or an equivalent capability that cannot omit initialization. Route production, placement, sell, refinery, slave miner, spawn manager, genetic converter, paradrop, world-mod building placement, and MCV deploy/undeploy through `FreshScenario`. Preserve one draw on post-constructor failure. | Rust construction census in the research report and design revision 7; `src/sim/world/world_spawn.rs`. | Focused tests cover one placed success, one placed failure after construction, and one limbo construction; each advances the raw cursor exactly once and stores the low word. Existing runtime callers compile only through the explicit mode. |
| P0-06 | REQUIRED_FIX | MILESTONE | Generated-object preconsumed binding | An RMG techno whose constructor draw was already consumed by the generation cursor must install the captured word without advancing the simulation cursor. Generated-object binding is exact: missing, duplicate, or mismatched bindings are load errors, not fallback fresh draws. | No generated constructor-state DTO or preconsumed mode exists. | Add `TechnoConstructorInit::PreconsumedGenerated(GeneratedTechnoInit)` and a deterministic binding identity suitable for the later RMG DTO. Install the supplied word with zero RNG calls and return a deterministic load error on binding mismatch. Do not implement or infer the RMG generation trace in this slice. | Dual-cursor lifecycle and generated handoff in the cited RMG research report; approved design revision 7. | Synthetic binding tests prove zero simulation draws, exact value installation, deterministic binding, and errors for missing/duplicate/mismatched records. |
| P0-07 | REQUIRED_FIX | COMPOUNDING | Restore, snapshot, and hash | Restoring a techno reinstalls its saved constructor word and optional upgrade link without consuming RNG. Both affect deterministic state comparison. | `GameEntity` derives serde, but the fields do not exist; world hash does not include them; snapshot version is 103. | Add `TechnoConstructorInit::Restored(u16)` at any reconstruction boundary that needs it, while normal serde restore remains draw-free. Serialize and hash the word and upgrade link. Bump `SNAPSHOT_VERSION` to 104 with a version-history entry and update its tests. | Native persistence/later reads; `src/sim/world/world_hash.rs`; `src/sim/snapshot.rs`. | Save/load round-trip produces zero new draws, preserves word/link exactly, and preserves world hash. Changing only the word or link changes the hash. Version tests expect 104. |
| P0-08 | TEST_ONLY | COMPOUNDING | Direct-construction boundary | Production code must not be able to create a techno while silently omitting constructor initialization. Test-only helpers may remain explicit and cannot leak into production. | Three production `GameEntity::new_at_frame` calls exist in `world_spawn`; `new_at_frame_zero_for_test` is test-only. | Centralize production construction behind one explicit initializer. Make the lower-level constructor private or require the init value so source review can distinguish intentional test construction. | Rust source census at `dfe44984`. | Source scan and compile-time signatures show all production ingress uses an explicit mode; only `#[cfg(test)]` helpers may synthesize a value without simulation RNG. |
| P0-09 | TEST_ONLY | MILESTONE | Earlier scenario RNG ownership | The fixed-map constructor sequence begins from the cursor left by prior scenario loading, including the low `Mark` draw. P0 must not move, suppress, or duplicate that earlier draw. | Runtime already threads bootstrap RNG into simulation after scenario setup; the design's earlier high/low Mark ambiguity is resolved. | Preserve the existing prior draw ownership. Tests prepare the constructor cursor by stepping the earlier Mark draw rather than attributing it to the first techno. | Mark-before-authored trace in the cited research report and approved design revision 7. | Fixed-map raw-sequence test fails if Mark is reordered, duplicated, or omitted. |

There are no `BLOCKED` or `UNKNOWN` requirements in this prerequisite. Later bridge mechanisms remain open by design, not because P0 evidence is approximate.

## Required Rust shape

Names may change for local clarity, but these capability boundaries are mandatory:

```rust
enum TechnoConstructorInit {
    FreshScenario,
    PreconsumedGenerated(GeneratedTechnoInit),
    Restored(u16),
}

struct StructureUpgradeLink {
    parent_stable_id: u64,
    slot: u8,
}
```

- `FreshScenario` requires mutable access to the single active simulation/scenario RNG and performs exactly one `next_u32()` call.
- `PreconsumedGenerated` and `Restored` perform zero RNG calls.
- The generated DTO includes a deterministic binding identity plus the already captured low word. Its producer belongs to the later RMG lifecycle mechanism.
- Fixed authored projection validates pre-constructor rejections, constructs once, then attempts reveal/Unlimbo.
- PostMap separates construction from placement attempts so the same stable entity survives exact/fallback search.
- Authored upgrades use a dedicated attached reveal path. Native Unlimbo is required, but ordinary independent structure occupancy is not: the base and upgrade coexist at the same location.
- The field and upgrade link participate in serde and the explicit deterministic world hash.

Expected owner files are:

- `src/map/entities.rs` for authored upgrade parsing and generated binding input shape if map-owned;
- `src/sim/runtime.rs` and `src/sim/scenario_bootstrap.rs` for fixed/PostMap ordering and failure behavior;
- `src/sim/world/world_spawn.rs` and its entity definition owner for construction modes, persistent fields, and attached upgrade reveal;
- `src/sim/world/world_hash.rs` for deterministic hashing;
- `src/sim/snapshot.rs` for version 104 and compatibility bookkeeping;
- focused module tests beside those owners.

The builder may choose a smaller placement for the DTO/types if dependencies remain one-way and the explicit construction boundary is preserved.

## Acceptance suite

The builder must add focused `--lib` tests that collectively prove:

1. **Fixed authored sequence:** after a prepared prior-Mark draw, valid `Units`, `Aircraft`, `Infantry`, and `Structures` rows store consecutive raw low words in native section/key order. Invalid type/owner rows consume nothing. A valid constructed row that later cannot Unlimbo consumes once and leaves no live placed object.
2. **Authored upgrades:** the parent construction is followed by valid upgrade-slot constructions in slot order. Each upgrade is a distinct live entity with its own word, stable ID, parent/slot link, parent location, and no competing blocking footprint.
3. **Starting MCV:** exact success, fallback success, and total failure each construct once. Constructor RNG precedes fallback-cell RNG, and total failure retains the cursor advance while deleting the object.
4. **Extra starting unit:** type/candidate selection occurs before its constructor draw; placement randomness follows it; fallback attempts reuse the same object.
5. **Runtime fresh construction:** representative placed success, post-constructor failure, and limbo creation each draw once and store the exact low word.
6. **Generated binding:** a synthetic preconsumed table installs exact words with zero simulation draws and rejects missing, duplicate, and mismatched bindings deterministically.
7. **Persistence:** snapshot round-trip performs no draw and preserves word/link/hash; a word-only or link-only mutation changes the hash; snapshot version is 104.
8. **Boundary census:** production code has no uninitialized direct techno-construction call site.

While working, run only scoped `cargo test -p vera20k --lib <filter>` commands after checking that no other session owns Cargo. The repository-wide `cargo test -p vera20k --lib` remains reserved for the final bridge-wide completion pass, exactly once under `ENGINE.md`.

## Evidence-backed nonrequirements

- Do not wire the constructor word into DiskLaser/spawn-bullet report playback in P0; the persistent state is supplied here and the owning consumers close later.
- Do not implement the complete structure-upgrade gameplay system. P0 includes only parsing, distinct techno construction, attached Unlimbo lifecycle, identity, persistence, and hashing required by constructor parity.
- Do not implement the RMG generation cursor or infer its draw schedule. P0 defines and validates only the zero-draw preconsumed handoff boundary.
- Do not reorder or relabel the earlier scenario `Mark` draw as a techno draw.
- Do not add TS-only, dormant, editor-only, campaign-only, or OpenTS-derived behavior without independent active-retail proof.
- Do not use an approximate per-entity RNG, stable-ID hash, or retry draw.
- Do not broaden this prerequisite into bridge damage, collapse, repair, rendering, audio, or AI behavior.

## Blockers and assumptions

**Blockers:** none.

The only translation judgment is the Rust representation of authored upgrades. Native evidence proves a distinct constructed `BuildingClass` and a virtual Unlimbo call at the base location. It does not license treating the upgrade as a second independently blocking building footprint. The minimum faithful Rust result is therefore a distinct live techno with parent/slot identity and attached placement semantics that cannot fail merely because its parent occupies the location. Later upgrade gameplay effects remain separately owned.

## Ghidra annotation candidates

None. The required addresses and lifecycle evidence are captured in the cited research handoff; this implementation contract does not request metadata synchronization.

## Handoff

The next safe action is to implement P0 as one bounded builder-owned prerequisite on `feature/bridge-movement-parity`, validate it with focused `--lib` tests, commit the coherent slice, and give a fresh read-only critic this contract, the native evidence, the diff, and literal test output. Any critic finding must be fixed and rechecked by a new critic before later bridge mechanisms begin.
