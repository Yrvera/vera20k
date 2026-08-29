# Bridge Load Inputs and Raw Facts Implementation Contract

**Date:** 2026-08-29
**Status:** READY_FOR_PLAN
**Owning slice:** Active-retail bridge parity, closure unit 1
**Coverage:** BR-M01, BR-M06, BR-M24
**Implementation authority:** active-retail `gamemd.exe` and Yuri's Revenge retail data only

## Exact parity gap

Rust loads only six of the ten active `[General]` bridge-piece keys, so the bottom-left and bottom-right pavement/edge classes needed by later high-bridge owners disappear at the theater boundary. The remaining unit-1 behavior is already substantially present but lacks exact oracles: raw cell bridge bits must remain independent, `TIBTRE` placement must reject precisely raw mask `0x500`, mode-specific bridge-destruction inputs must remain distinct, and automatic TubeClass shells must be classified against the complete shipped theater corpus rather than assumed dormant from low-bridge deck data.

## Scope

This contract closes only the immutable inputs and classifications on which later bridge mechanisms depend:

- the ten active theater bridge-piece indices and their distinct Rust storage;
- preservation of the raw cell flag word and the exact `0x500` Tiberium-placement mask;
- the active automatic-shell predicate and a shipped-retail corpus oracle;
- preservation of scenario/session bridge-destruction inputs and already-correct bridge rules/body-asset resolution;
- evidence-backed exclusions at this load boundary.

It does not implement high-bridge edge restamping, low-bridge mutation, TubeClass traversal, destruction admission, collapse/repair, rendering, audio, or RMG topology. Those remain with their later closure units.

## Evidence baseline

| Evidence | Role | Authority used here |
|---|---|---|
| `Read_Theater_TileSets_INI @ 0x00545150` in `docs/research/bridges/01-assets-map-load-overlay/ASSET_PARSING_BRIDGES_GHIDRA_REPORT.md` | Primary native | Exact ten active `[General]` bridge-piece reads and distinct global destinations |
| `docs/research/HIGH_BRIDGE_RIM_REFRESH_ALGORITHM_GHIDRA_REPORT.md`, `docs/research/bridges/01-assets-map-load-overlay/LAT_RETRIGGER_AND_BRIDGE_DAMAGE_VARIANT_GHIDRA_REPORT.md`, and `docs/research/bridges/05-damage-collapse-repair-cabhut/HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md` | Primary native | Bottom-right pair owns east-edge pavement classification; bottom-left pair owns south-edge pavement classification; neither pair is interchangeable with top/middle ramp pieces |
| `CellClass::CanPlaceTiberium @ 0x004838E0`, assembly `0x004838FC..0x00483905`, in `docs/research/CELL_FLAGS_0X500_TIBTRE_PLACEMENT_SEMANTICS_GHIDRA_REPORT.md` | Primary native | Early rejection is exactly `(CellClass+0x140 & 0x500) != 0`; it is not a generalized walkability test |
| `OverlayClass::Mark @ 0x005FC570` in the same report | Primary native | Active `TIBTRE01..03` resource-spawn path reaches `CanPlaceTiberium` after bridge-state setters write the raw word |
| `IsoTileTypeClass` terrain-byte table in `docs/research/ISOMETRIC_TILE_TYPE_CLASS_GHIDRA_REPORT.md` | Primary native | TMP terrain byte `5` maps to final LandType `10` |
| `CellClass::RecalcAttributes` automatic TubeClass branch in `docs/research/bridges/04-locomotion-height-tubes/LOW_BRIDGE_TUBECLASS_GHIDRA_REPORT.md` | Primary native | Shell eligibility is LandType 10, no existing tube index, and tile membership in the first four entries of Tunnels/TrackTunnels/DirtTunnels/DirtTrackTunnels; the tile's zero-based offset within the matched four-tile band indexes `[2,4,6,0]`, and shells are same-cell zero-step records |
| `docs/research/bridges/00-system-models/ACTIVE_RETAIL_BRIDGE_COVERAGE_REINVESTIGATION_GHIDRA_REPORT.md` | Primary native synthesis | Low Road overlays return before the automatic-shell branch; active YR has no `TrainBridgeSet` key reader; WaterBridge names and `RAILBRDG` art do not establish train-bridge topology |
| `docs/research/bridges/05-damage-collapse-repair-cabhut/SPECIALFLAGS_DESTROYABLEBRIDGES_DEFAULT_AND_MODES_GHIDRA_REPORT.md` | Primary native | Scenario `[SpecialFlags] DestroyableBridges` owns campaign/editor, session `BridgeDestruction` owns skirmish/multiplayer, and `[CombatDamage] DestroyableBridges` is not the active authority |
| `docs/research/bridges/01-assets-map-load-overlay/BRIDGE_BODY_ASSET_RESOLUTION_GHIDRA_REPORT.md` plus retail `rulesmd.ini`/`artmd.ini` | Primary native/retail | `BRIDGE1/2 -> Image=BRIDGE`, `BRIDGEB1/2 -> Image=BRIDGB`, and both image types are theater assets |
| `ini/temperatmd.ini`, `snowmd.ini`, `urbanmd.ini`, `urbannmd.ini`, `desertmd.ini`, `lunarmd.ini` | Primary retail | All six active MD theaters provide the same exact ten relative values; configured tunnel-set bounds are the corpus input |
| Installed retail MIX chain under `C:\Users\enok\Documents\Command and Conquer Red Alert II` | Primary retail | Exact automatic-shell corpus result: 40 loaded active TMP assets, 540 present subcells, 36 qualifying subcells |
| `src/map/theater.rs`, `src/map/bridge_facts.rs`, `src/map/resolved_terrain.rs`, `src/sim/tiberium/mod.rs`, `src/map/basic.rs`, and `src/rules/ruleset.rs` at `5bb72d04` | Primary Rust | Current owners and preservation points |
| `C:\Users\enok\Documents\OpenTS\code\isotype.cpp`, `cell.cpp`, and `map.cpp` | Navigation only | Located the inherited ten-key, automatic-shell, and bottom-piece mechanisms; no parity conclusion below depends on OpenTS |

The retail corpus was read through the repository's released asset loader. It enumerated each configured set's first four base tiles and the loader's actually present contiguous variants, then decoded every present TMP subcell through the active terrain-byte mapping.

## Exact theater inputs

`Read_Theater_TileSets_INI` reads and stores these ten distinct BridgeSet-relative indices:

| INI key | Native destination | Retail value in every active MD theater | Rust owner after this slice |
|---|---:|---:|---|
| `BridgeTopLeft1` | `DAT_00ABC2B4` | 1 | `bridge_top_left_1` |
| `BridgeTopLeft2` | `DAT_00AA1130` | 2 | `bridge_top_left_2` |
| `BridgeBottomRight1` | `DAT_00ABC1E8` | 3 | `bridge_bottom_right_1` |
| `BridgeBottomRight2` | `DAT_00AA0E38` | 3 | `bridge_bottom_right_2` |
| `BridgeTopRight1` | `DAT_00AA1548` | 4 | `bridge_top_right_1` |
| `BridgeTopRight2` | `DAT_00AA0740` | 5 | `bridge_top_right_2` |
| `BridgeBottomLeft1` | `DAT_00ABC1D0` | 6 | `bridge_bottom_left_1` |
| `BridgeBottomLeft2` | `DAT_00AA1540` | 6 | `bridge_bottom_left_2` |
| `BridgeMiddle1` | `DAT_00ABAD30` | 7 | `bridge_middle_1` |
| `BridgeMiddle2` | `DAT_00AA1028` | 12 | `bridge_middle_2` |

Repeated retail values do not merge ownership: each key remains independently configurable. Missing or native `-1` values remain absent. The four bottom fields are raw theater inputs for later edge/pavement consumers and must not be inserted into `BridgeRampTileTable`, whose current top/middle membership has a different native predicate.

## Automatic-shell retail verdict

Automatic shells are active on shipped content, but they are not low-bridge traversal. The exact positive corpus is:

| Theater | Set | Qualifying TMP/subtile entries |
|---|---|---|
| URBAN | Tunnels | `tunnel01.urb:[3,6,9]`, `tunnel02.urb:[1,2,3]`, `tunnel03.urb:[3,6,9]`, `tunnel04.urb:[1,2,3]` |
| URBAN | DirtTrackTunnels | `dtunn01.urb:[3,6,9]`, `dtunn02.urb:[1,2,3]`, `dtunn03.urb:[3,6,9]`, `dtunn04.urb:[1,2,3]` |
| NEWURBAN | Tunnels | `tunnel01.ubn:[3,6,9]`, `tunnel02.ubn:[1,2,3]`, `tunnel03.ubn:[3,6,9]`, `tunnel04.ubn:[1,2,3]` |

TEMPERATE, SNOW, DESERT, and every other active set in NEWURBAN contribute zero qualifying subcells; LUNAR declares zero tiles in all four relevant sets. The complete aggregate is 40 loaded TMP assets, 540 present subcells, and 36 LandType-10 shell candidates. No contiguous `a` variants are present for these inputs.

Every qualifying record has start cell equal to end cell. Its direction is `[2,4,6,0][tile - matched_range_base]`, independently of which of the four tunnel families supplied the matching range. It supplies an attribute shell to later hierarchy code; it is not joined into a moving tunnel and does not authorize direction-8 traversal. Stock low-bridge overlay deck cells resolve to Road and return before this branch.

## Required behavior and implementation deltas

| ID | Evidence class | Delivery class | Mechanism/result | Active-retail requirement | Current Rust state | Required Rust delta | Acceptance |
|---|---|---|---|---|---|---|---|
| U1-01 | REQUIRED_FIX | MILESTONE | Ten theater bridge-piece inputs | Preserve all ten distinct BridgeSet-relative values, including independently configured duplicates and absent/`-1` values. | Six top/middle fields are parsed; four bottom fields are discarded. | Add four optional bottom fields to `TheaterData`, parse the exact key names, and update every construction boundary without changing ramp-table membership. | Synthetic differing-value fixture proves ten independent fields and absent/`-1` behavior; six-repo-INI fixture proves the retail vector `1,2,3,3,4,5,6,6,7,12`. |
| U1-02 | TEST_ONLY | MILESTONE | Bottom-piece semantic boundary | Bottom-right and bottom-left pairs are later east/south pavement-edge inputs, not aliases for top/middle pieces. | The fields do not exist, so later consumers cannot receive them. | Retain explicit names and document/test that `BridgeRampTileTable` remains built only from its current top/middle inputs. | A fixture with disjoint values proves the new bottom values do not classify as ramp pieces. |
| U1-03 | TEST_ONLY | PRESERVATION-BLOCKING | Raw flag preservation and TIBTRE mask | Preserve the raw flag word. Tiberium placement rejects if either `0x100` or `0x400` is set and does not reject because only `0x200` or `0x40000` is set. This gate is independent of generalized walkability. | Raw constants and `resolved_cell_accepts_tiberium` already implement `0x500`, but focused exact-mask fixtures are incomplete. | Preserve implementation; add table-driven raw-mask fixtures, including unrelated-bit retention through the existing bridge flag writer boundary. | `0x100`, `0x400`, and `0x500` reject; `0x000`, `0x200`, and `0x40000` pass when the separate non-bridge gates pass; writer round trip preserves unrelated bits. |
| U1-04 | TEST_ONLY | CONTENT-CONDITIONAL-BLOCKING | Automatic-shell predicate | Final LandType must be 10, tile must be among the first four entries of one configured tunnel family, and an existing tube index suppresses shell creation. Tile offsets `0..3` map exactly to directions `[2,4,6,0]` for every family. | Predicate and directions appear implemented, but only partial synthetic coverage exists. | Preserve implementation and add exact synthetic boundary tests for byte-5 mapping, four-entry bounds, per-offset directions, every family, and existing-tube suppression. | Tests reject wrong land, fifth tile, absent/zero-length set, and existing tube; accept every offset/family combination with exact same-cell endpoints and direction. |
| U1-05 | TEST_ONLY | MILESTONE | Shipped automatic-shell corpus | The active six-theater corpus produces exactly the 36 positive subcells listed above from 40 loaded assets/540 present subcells. | No executable retail corpus oracle exists. | Add an ignored retail-data test using the normal theater/asset/TMP loaders. It must enumerate the loader-visible corpus rather than hard-code only positive filenames. | With configured retail data, exact aggregate and exact positive set match; no unlisted theater/set/subtile qualifies. |
| U1-06 | TEST_ONLY | MILESTONE | Low Road exclusion | A low-bridge overlay deck cell takes the low-overlay Road return and cannot acquire an automatic shell from the LandType-10 branch. | Current resolution appears to preserve the early return; coverage is indirect. | Add a focused resolved-terrain negative test without changing low-bridge mutation ownership. | A low Road overlay on otherwise qualifying terrain remains Road and produces no automatic shell. |
| U1-07 | TEST_ONLY | MILESTONE | Destruction-authority inputs | Campaign/editor retains scenario `[SpecialFlags] DestroyableBridges` with reset default true; skirmish/multiplayer retains session `BridgeDestruction`; `[CombatDamage] DestroyableBridges` does not overwrite either. | The mode-aware inputs and tests already exist. | Preserve code and keep focused regression coverage green. Do not add the weapon/CABHUT admission matrix here. | Existing plus focused tests prove map override, session ownership, default true, and CombatDamage non-ownership. |
| U1-08 | TEST_ONLY | COMPOUNDING | Rules/body asset inputs | Existing `BridgeStrength`, bridge explosion/debris lists, repair sound/hut type, and `BRIDGE/BRIDGB` theater-image resolution remain available to their later owners. | Current Rust parses/resolves these inputs and already has targeted tests. | Preserve current behavior; no speculative asset fallback or render-table rewrite in this unit. | Existing focused rules, object-type, and body-resolution tests remain green. |
| U1-09 | DOC_ONLY | COMPOUNDING | Negative-fact boundary | `TrainBridgeSet` is not parsed by active YR despite stale retail INI text. `RAILBRDG` is art data, WaterBridge names are not topology, and automatic shells are not traversable tubes. | Rust has no `TrainBridgeSet` theater field, which is correct. | Keep these mechanisms absent and make tests/contracts fail if the unit invents a TrainBridgeSet topology owner. | Source/diff review shows no parser field or behavior added for these excluded leads. |

There are no `BLOCKED` or `UNKNOWN` requirements in closure unit 1.

## Required Rust shape

Names below are normative because later contracts need stable, unambiguous inputs:

```rust
pub bridge_bottom_right_1: Option<u16>,
pub bridge_bottom_right_2: Option<u16>,
pub bridge_bottom_left_1: Option<u16>,
pub bridge_bottom_left_2: Option<u16>,
```

- Values are relative indices exactly like the six existing fields and use the same absent/`-1` conversion.
- `BridgeRampTileTable` must remain limited to the native top/middle inputs until a later contract proves another consumer.
- The raw bridge flag word remains the inter-owner representation. Unit 1 must not replace it with one generalized `is_bridge` or `is_walkable` boolean.
- The retail corpus oracle may be ignored when no configured retail install is available, but it must use the shipped data when explicitly selected and must report the exact aggregate and positive set.
- The regular test suite must still contain deterministic synthetic coverage for every branch so ordinary validation does not depend on an external retail installation.

Expected owner files are:

- `src/map/theater.rs` and `src/map/theater_tests.rs` for the four fields, ten-key parsing, and retail corpus oracle;
- `src/map/resolved_terrain.rs` and its tests for automatic-shell/low-Road classification;
- `src/sim/tiberium/mod.rs` and `src/sim/terrain_spawn.rs` tests for the exact raw-mask admission boundary;
- existing `src/map/bridge_facts.rs`, `src/map/basic.rs`, `src/rules/ruleset.rs`, and bridge asset tests as preservation owners.

The builder may place tests beside the narrowest owner, but may not move later behavior into unit 1.

## Acceptance suite

After confirming no other session owns Cargo, the builder must run focused `--lib` validation that collectively proves:

1. **Ten-key synthetic parsing:** all ten independent values, missing values, and `-1` conversion; bottom values do not enter the ramp table.
2. **Six-theater retail INIs:** each active MD definition produces the exact ten-value vector.
3. **Raw mask:** exact `0x500` reject truth table and unrelated-bit preservation, independent of walkability.
4. **Automatic-shell synthetic matrix:** LandType conversion, set bounds, offset-indexed directions for every family, same-cell endpoints, existing-index suppression, and low Road early return.
5. **Automatic-shell retail corpus:** ignored external-data test returns 40 loaded assets, 540 present subcells, 36 candidates, and exactly the listed URBAN/NEWURBAN positives.
6. **Destruction input preservation:** campaign/editor map authority, skirmish/multiplayer session authority, default true, and CombatDamage non-owner.
7. **Rules/assets preservation:** existing bridge rule, repair-hut, body-image, and theater-asset tests remain green.

Only focused commands are permitted during this unit, all with `cargo test -p vera20k --lib <filter>` or `cargo check -p vera20k --lib`. The repository-wide `cargo test -p vera20k --lib` remains reserved for the single final bridge-wide validation after all closure units pass.

## Evidence-backed nonrequirements

- Do not parse, store, or subtract `TrainBridgeSet`; the corresponding OpenTS topology is a TS-only lead, not active YR authority.
- Do not classify WaterBridge filenames or `RAILBRDG` art as a separate physical bridge family.
- Do not make automatic same-cell shells traversable, connect them, or add direction 8. Explicit TubeClass load/hierarchy/traversal closes in unit 5.
- Do not infer automatic shells from stock low-bridge overlay decks. Those cells are Road and return earlier.
- Do not wire the four bottom fields into high-bridge edge refresh yet; unit 4 owns that consumer.
- Do not implement weapon, CABHUT, attached-bomb, collapse, or repair authority. Unit 10 and later transaction units own those gates.
- Do not change bridge-body frame/render semantics or the unresolved `C_SHADOW`/`RAILBRDG` presentation relationship. Unit 20 owns rendering.
- Do not replace raw flags with terrain speed, occupancy, or a generalized passability predicate.
- Do not add TS-only, dormant, editor-only writing UI, or OpenTS-derived behavior without independent active-retail proof.

## Blockers and assumptions

**Blockers:** none.

The corpus count treats a TMP subcell as present only when the retail loader exposes that subcell and decodes its terrain byte. It enumerates the configured first-four set entries and their contiguous loader-visible variants, matching the active shell predicate rather than scanning unrelated MIX contents. The absence of `a` variants is therefore a measured corpus fact, not an assumed naming rule.

## Ghidra annotation candidates

None. The native addresses and table destinations required here are already captured in the cited research documents.

## Handoff

Implement closure unit 1 on `feature/bridge-movement-parity`, validate only the focused `--lib` targets above, and commit the coherent slice. Then give a fresh read-only critic this contract, the cited native/retail evidence, the exact diff, and literal validation output. Any material finding reopens unit 1; fix the largest finding and submit the complete updated bundle to a new critic who rechecks prior fixes. Unit 2 may not begin until a fresh critic passes unit 1 with no unresolved or approximate behavior.
