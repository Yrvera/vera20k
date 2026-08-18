# ZoneMap BuildZoneLevel Temp Edge Bucket Keys -- Ghidra Research Report

**Address(es):** `0x00581F90`, `0x005824A0`, `0x00582D70`, `0x0058AF80`, `0x00567110`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** exact temporary edge bucket key derivation, duplicate comparison, insertion order, and final adjacency emission order for `ZoneMap__BuildZoneLevel` production hierarchy construction.  
**Non-Scope:** retry producer, `CellClass+0x122`, layered A*, slope, explicit direction-8 tube marker behavior, and stock Carville route oracle.  
**Confidence:** High for static bucket key, duplicate comparison, append order, and final emission order; Medium for exact runtime route impact because no live stock-map trace was run.  
**Active in YR:** Yes. `FUN_00567110` builds levels `2 -> 1 -> 0` through `ZoneMap__BuildZoneLevel`; the resulting hierarchy arrays are consumed by normal `Zone_precheck` pathing in standard YR.

## 0. Working Notes Contract

Target question: Verify exact temp-edge bucket key derivation, exact duplicate comparison, insertion order, and final adjacency emission order for production `ZoneHierarchy` builder.

Non-goals: Do not investigate retry producer, `CellClass+0x122`, layered A*, slope, explicit tube direction-8 marker behavior, or stock Carville route.

Evidence needed to mark COMPLETE: decompile plus assembly for scanline temp insertion, bridge/tube temp insertion, append helper, and final emission; caller evidence that `ZoneMap__BuildZoneLevel` runs in YR hierarchy build.

Stop conditions: Stop after the bucket key and emission algorithm are implementation-ready; record route-oracle and repeated lifecycle questions as remaining uncertainty instead of expanding scope.

## 1. Overview

`ZoneMap__BuildZoneLevel` does not emit final zone adjacency while flood-filling. It first stages temporary 12-byte edge entries in 256 buckets, then drains those buckets in numeric bucket order and entry insertion order into final per-zone edge arrays.

The key implementation fact is that the bucket key is not a hash of full zone ids and not sorted/canonicalized. For a packed temporary pair `(high_endpoint << 16) | low_endpoint`, the bucket index is:

```text
bucket = ((high_endpoint & 0xF) << 4) | (low_endpoint & 0xF)
```

Duplicate suppression compares only the full packed pair in entry dword 0. Reversed endpoints are a different packed pair and land in the reversed-nibble bucket. The final writer emits the low-halfword endpoint's directed edge first, then the high-halfword endpoint's reverse edge.

## 2. Class Layout / Key Offsets

| Structure | Offset / stride | Meaning | Evidence | Active in YR |
|---|---:|---|---|---|
| `MapClass` temp bucket vector array | `MapClass+0x80 + level*4` points to 256 vector headers | staging surface for temporary 12-byte edge entries | `ZoneMap__FloodFillScanline @ 0x005824A0`; `FUN_00582D70 @ 0x00582D70` | Yes |
| Temp bucket header | stride `0x18`; bucket header at `base + bucket*0x18` | vector header with data pointer at `+4`, count at `+0x10` | assembly `0x00582625..0x00582631`, final loop `0x0058236E..0x00582381` | Yes |
| Temp edge entry | stride `0x0C` | dword0 packed pair, dword1 duplicate packed pair, dword2 flag dword | append helper `0x0058AF80`; final read `0x00582395..0x00582398` | Yes |
| Final edge record | stride `0x08` | neighbor zone id at `+0`, flag dword at `+4` | final writer `0x005823EF..0x00582402`, `0x00582448..0x0058245B` | Yes |
| Per-level cell zone id | `cell_zone_base + (level + cell_index*5)*2` | source zone ids packed into temp edges | `FUN_00582D70` decompile | Yes |

## 3. Core Logic

### 3.1 Hierarchy build path is live in YR

`FUN_00567110` initializes map cell attributes and bridge-zone records, then calls `ZoneMap__BuildZoneLevel` for `iVar6 = 2`, `1`, and `0`.

Evidence: decompile of `FUN_00567110 @ 0x00567110` shows `MapClass__InitCellAttributes`, `MapClass__ComputeBridgeZones`, `MapClass__UpdateBridgeZonesHelper`, then `iVar6 = 2` and looped `ZoneMap__BuildZoneLevel(iVar6)` with decrement. Active in YR: Yes; this is the normal hierarchy construction path used before `Zone_precheck`.

### 3.2 Scanline temp bucket key uses low nibbles of full directed packed pair

Every scanline insertion site follows the same shape:

1. Build packed pair as `(existing_zone << 16) | current_zone`.
2. Build bucket as `((existing_zone & 0xF) << 4) | (current_zone & 0xF)`.
3. Address bucket vector at `temp_base + bucket*0x18`.
4. Scan existing entries in that bucket for exact packed-pair equality.
5. Append the 12-byte entry if not found.

Assembly evidence for one insertion site:

- `0x00582604..0x00582612`: move existing zone to `EBP`, shift `EBP << 16`, OR current low 16 bits to form the packed pair.
- `0x0058260F..0x0058261F`: mask existing zone with `0xF`, shift left four, mask current zone with `0xF`, OR to form the bucket.
- `0x00582627`: `LEA EAX,[EAX + EAX*0x2]`; with following `*0x8`, this addresses `bucket*0x18`.
- `0x0058262A..0x00582631`: reads bucket count and data pointer from `base + bucket*0x18 + 0x10/+4`.

Active in YR: Yes; `ZoneMap__FloodFillScanline` is called by `ZoneMap__BuildZoneLevel` during normal hierarchy construction.

### 3.3 Duplicate comparison is exact packed pair only

At scanline insertion, the duplicate loop compares the candidate packed pair against dword 0 of each 12-byte entry. It does not compare dword 1, does not compare the flag dword, and does not canonicalize endpoint order.

Evidence:

- `0x00582635..0x00582643`: loop over entries; `CMP EBP,dword ptr [EDX]`, then `ADD EDX,0xC`.
- `0x00582687..0x0058268C`: append writes dword0 packed pair, dword1 duplicate packed pair, dword2 flag dword.
- Similar duplicate comparisons appear in later scanline branches (`0x005827CD`, `0x00582CAD`) and bridge/tube insertion decompile.

Active in YR: Yes.

### 3.4 First inserted duplicate wins position and flag

If an exact packed pair already exists in the bucket, the insertion branch jumps over append. There is no replacement of entry dword 2. Therefore the first inserted copy keeps both its bucket position and its flag dword.

Evidence: duplicate hit jumps to `LAB_0058269F` after `CMP EBP,dword ptr [EDX]`; append writes only occur on the no-duplicate path at `0x00582671..0x0058268C`. Active in YR: Yes.

### 3.5 Reversed endpoints are distinct

Because the duplicate key is the full packed pair, `(A << 16) | B` and `(B << 16) | A` are different. They also usually use different buckets: `((A & 0xF) << 4) | (B & 0xF)` versus `((B & 0xF) << 4) | (A & 0xF)`.

Evidence: all duplicate loops compare the candidate packed dword exactly; no min/max, swap, or canonicalization appears in `ZoneMap__FloodFillScanline` or `FUN_00582D70`. Active in YR: Yes.

### 3.6 Append helper copies exactly three dwords

`FUN_0058AF80` appends a temp entry by copying three dwords from the caller's local tuple into the selected bucket vector. The second dword is copied from caller input and is equal to the packed pair in verified callers, but duplicate comparisons use only dword 0.

Evidence: decompile of `FUN_0058AF80 @ 0x0058AF80` shows `*dst = *src`, `dst[1] = src[1]`, `dst[2] = src[2]`. Active in YR: Yes; bridge/tube insertion calls this helper and scanline insertion contains equivalent inline writes.

### 3.7 Bridge/tube temp insertion uses the same bucket key and duplicate rule

`FUN_00582D70` computes three bridge/tube connection pairs. For each pair it:

1. reads the two endpoint zones for the current hierarchy level;
2. packs `local_c = zone_a << 16 | zone_b`;
3. computes bucket `((zone_a & 0xF) << 4) | (zone_b & 0xF)`;
4. scans that bucket for exact `local_c`;
5. appends through `FUN_0058AF80` only if absent.

The bridge/tube local flag byte is set to zero before append.

Evidence: `FUN_00582D70` decompile around the three `local_c` constructions; duplicate checks compare `local_c == *puVar10`; calls to `FUN_0058AF80`; `local_4 = 0` before insertion. Active in YR: Yes; `ZoneMap__BuildZoneLevel` calls `FUN_00582D70` after scanline discovery for active bridge records.

### 3.8 Final emission drains buckets `0..255`, then insertion order within each bucket

After bridge/tube temp insertion, `ZoneMap__BuildZoneLevel` initializes offset `0` and loops until `< 0x1800`, stepping by `0x18`. Since each bucket header is `0x18` bytes, that is exactly 256 buckets in ascending numeric order.

Evidence:

- `0x0058236E..0x00582381`: load current bucket header and count from `base + offset`.
- `0x0058247D..0x0058248A`: `ADD EDX,0x18`, `CMP EDX,0x1800`, loop while less.
- `0x00582467`: after each entry, advance temp entry pointer by `0x0C`.

Active in YR: Yes.

### 3.9 Final directed edge order is low-halfword endpoint first, then high-halfword reverse

For each temp entry:

1. Read packed pair dword.
2. `low = packed & 0xFFFF`.
3. `high = packed >> 16`.
4. Append edge `low -> high` with the temp flag.
5. Append edge `high -> low` with the same temp flag.

Evidence:

- `0x00582395..0x005823BD`: read packed pair, compute low in `EDI`, high in `EBP`.
- `0x005823EF..0x00582402`: first append writes neighbor `EBP` into zone record indexed by `EDI`.
- `0x00582448..0x0058245B`: second append writes neighbor `EDI` into zone record indexed by `EBP`.

Active in YR: Yes.

### 3.10 Final writer does not sort and does not final-dedup

The final writer appends to each zone's edge vector in bucket/entry order. No final sorted insertion or scan for existing final neighbor is visible in `0x00582395..0x00582480`.

Evidence: final loop appends by growing each zone's vector and writing into `edge_count * 8`; no comparison against existing final neighbor entries occurs in the verified range. Active in YR: Yes.

### 3.11 Flag dword propagates from temp entry dword 2 to both final directions

The low byte of temp entry dword 2 is read once and written to both final directed edges' `edge+4` dword. Scanline branches can set the low byte to `1` for cross-block/range boundary contacts; bridge/tube insertions set it to `0`.

Evidence:

- `0x00582398`: reads byte at temp entry `+8`.
- `0x005823A4`, `0x005823AB`: stores that byte into stack locals.
- `0x00582402` and `0x0058245B`: write the flag dword to final `edge+4` for both directions.
- `ZoneMap__FloodFillScanline` decompile sets low byte `1` when adjacent cross contact is outside the current block range; `FUN_00582D70` sets `local_4 = 0`.

Active in YR: Yes.

## 4. INI Keys

No INI key directly controls this temp bucket key or final emission order.

| Key / data | Default / effect | Evidence | Active in YR |
|---|---|---|---|
| `MovementZone=` | selects later passability rows, not the bucket key | existing movement-zone parser/report context; not read in scoped writer code | Yes, but out of this writer-key scope |
| Bridge/tube records | map/terrain-derived records feed `FUN_00582D70` after scanline discovery | `ZoneMap__BuildZoneLevel` call to `FUN_00582D70`; bridge record active-byte check | Yes |

## 5. Integration Points

| Producer / consumer | Relationship | Evidence | Active in YR |
|---|---|---|---|
| `FUN_00567110` | full hierarchy build caller; calls levels `2 -> 1 -> 0` | decompile `0x00567110` | Yes |
| `ZoneMap__BuildZoneLevel` | owns temp bucket staging and final emission | decompile `0x00581F90`; assembly `0x0058236E..0x0058248A` | Yes |
| `ZoneMap__FloodFillScanline` | creates scanline temp edges and exact duplicates | decompile `0x005824A0`; assembly `0x00582604..0x0058268C` | Yes |
| `FUN_00582D70` | injects active bridge/tube temp edges with same key and zero low-byte flag | decompile `0x00582D70` | Yes |
| `FUN_0058AF80` | appends 12-byte bridge/tube temp entries | decompile `0x0058AF80` | Yes |
| `Zone_precheck` | later reads final edge arrays in stored order | prior report `BRIDGE_ZONE_PRECHECK_HIERARCHY_WRITER_ORDER_GHIDRA_REPORT.md` | Yes |

## 6. Current Rust Implementation Status

| Surface | Current status vs verified bucket/emission behavior | Evidence |
|---|---|---|
| `src/sim/pathfinding/zone_hierarchy.rs` | has ordered `Vec` edge storage and test scaffolding, but no production temp-bucket builder yet | `ZoneLevelGraph`, `ZoneEdgeRecord`, `push_edge` around lines 99..156 |
| `src/sim/pathfinding/zone_build.rs::extract_adjacency` | one-level row-major adjacency extraction; not 256-bucket temp staging and not packed-pair duplicate semantics | source scan hits around lines 594..625 |
| `src/sim/pathfinding/zone_build.rs::build_node_adjacency` | sorts/dedups node adjacency and must not feed exact hierarchy order | source scan around lines 302..345 |
| `src/sim/pathfinding/zone_build.rs::inject_bridge_adjacency` | appends bridge adjacency after base extraction but lacks per-level temp bucket placement, exact packed-pair duplicate rule, and flag dword propagation | source scan around lines 633..666 |
| `src/sim/pathfinding/zone_map.rs::ZoneGrid::build_with_terrain` | builds/stores one-level maps and can store optional hierarchies, but does not build production `ZoneHierarchy` yet | source scan around lines 182..255 |

No Rust files were modified.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Full hierarchy caller liveness | verified | `FUN_00567110 @ 0x00567110` | none for this scope |
| Scanline packed pair construction | verified | `ZoneMap__FloodFillScanline`; asm `0x00582604..0x00582612` | none |
| Scanline bucket key | verified | asm `0x0058260F..0x00582631` | none |
| Scanline duplicate comparison | verified | asm `0x00582635..0x00582643`; decompile later branches | none |
| Scanline insertion order | verified | append at current vector count; no sort before final emission | none |
| Bridge/tube bucket key | verified | `FUN_00582D70` decompile | none |
| Bridge/tube duplicate comparison | verified | `FUN_00582D70` decompile; `local_c == *puVar10` | none |
| Append helper copy shape | verified | `FUN_0058AF80 @ 0x0058AF80` | none |
| Final bucket drain order | verified | asm `0x0058236E..0x0058248A` | none |
| Final directed append order | verified | asm `0x00582395..0x0058245B` | none |
| Current Rust source surfaces | touched-not-exhausted | `rg` over `zone_hierarchy.rs`, `zone_build.rs`, `zone_map.rs` | exact implementation still pending |
| Runtime stock-map route impact | deferred | no runtime trace in this slot | trace selected map after implementation if exact route oracle is desired |

## 8. Open Questions -- Final State of the Investigation Log

- `[RESOLVED] OQ-1 -- Is `ZoneMap__BuildZoneLevel` active in standard YR hierarchy construction? -> Yes; `FUN_00567110` calls it for levels `2,1,0` after cell/bridge initialization.` (evidence: `0x00567110`; Active in YR: Yes)
- `[RESOLVED] OQ-2 -- What is the temp bucket key? -> `((high & 0xF) << 4) | (low & 0xF)`, where packed pair is `(high << 16) | low`.` (evidence: `0x00582604..0x00582631`; Active in YR: Yes)
- `[RESOLVED] OQ-3 -- Does the bucket key use full zone ids? -> No; only endpoint low nibbles choose the bucket, while the full packed pair is stored in the entry.` (evidence: `0x0058260F..0x0058261F`; Active in YR: Yes)
- `[RESOLVED] OQ-4 -- What is the duplicate comparison? -> exact equality against temp entry dword 0 packed pair only.` (evidence: `0x00582635..0x00582643`; Active in YR: Yes)
- `[RESOLVED] OQ-5 -- Does duplicate comparison canonicalize undirected edges? -> No; reversed endpoints are distinct packed pairs and distinct bucket keys unless low nibbles coincide symmetrically.` (evidence: `ZoneMap__FloodFillScanline`, `FUN_00582D70`; Active in YR: Yes)
- `[RESOLVED] OQ-6 -- Does a later duplicate update the flag dword? -> No; duplicate hit skips append and does not rewrite the existing entry.` (evidence: branch to `LAB_0058269F` before append writes; Active in YR: Yes)
- `[RESOLVED] OQ-7 -- What does the append helper copy? -> three dwords: packed pair, duplicate packed pair, flag dword.` (evidence: `FUN_0058AF80`; Active in YR: Yes)
- `[RESOLVED] OQ-8 -- Do bridge/tube temp edges use the same key? -> Yes; `FUN_00582D70` computes the same nibble bucket from the two endpoint zones.` (evidence: `0x00582D70`; Active in YR: Yes)
- `[RESOLVED] OQ-9 -- Are bridge/tube temp edge flags nonzero? -> No for this insertion path; local flag byte is zeroed before append.` (evidence: `FUN_00582D70`; Active in YR: Yes)
- `[RESOLVED] OQ-10 -- What is final bucket drain order? -> ascending 256 bucket headers, offset `0` to `<0x1800` stepping `0x18`.` (evidence: `0x0058247D..0x0058248A`; Active in YR: Yes)
- `[RESOLVED] OQ-11 -- What is final directed edge order per temp entry? -> low-halfword endpoint gets edge to high-halfword endpoint first, then high gets reverse to low.` (evidence: `0x00582395..0x0058245B`; Active in YR: Yes)
- `[RESOLVED] OQ-12 -- Is there final sorting or final dedup? -> No evidence in final writer; it appends to zone edge vectors as entries are drained.` (evidence: `0x00582395..0x00582480`; Active in YR: Yes)
- `[RESOLVED] OQ-13 -- Can current Rust `extract_adjacency` reproduce this exactly? -> No; it lacks 256-bucket staging and packed-pair duplicate semantics.` (evidence: `src/sim/pathfinding/zone_build.rs`; Active in YR: Rust delta)
- `[RESOLVED] OQ-14 -- Can sorted/deduped Rust node adjacency feed parity hierarchy? -> No; `build_node_adjacency` sorts/dedups, while gamemd final order is bucket/insertion order.` (evidence: `src/sim/pathfinding/zone_build.rs`, `0x0058236E..0x0058248A`; Active in YR: Rust delta)
- `[DEFERRED] OQ-15 -- Exact stock-map route impact of bucket collisions.` (category: needs-runtime-debugger; reason: static order is proved, but exact route cells require runtime map/path trace; next-step-if-pursued: trace rebuilt hierarchy ids and selected paths on a stock bridge-collapse scenario)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Temp bucket key is `((high & 0xF) << 4) | (low & 0xF)` for packed pair `(high << 16) | low`. Active in YR: Yes. | `0x00582604..0x00582631`; `FUN_00582D70` | missing | `src/sim/pathfinding/zone_build.rs`; future hierarchy temp-edge staging helper | Stage hierarchy edges into 256 ordered buckets by endpoint low nibbles before final emission. | Edges `(0x21,0x32)` and `(0x11,0x42)` collide into the same bucket and preserve insertion order. Proposed test name: `zone_hierarchy_temp_bucket_key_uses_endpoint_low_nibbles` | Do not use `ZoneId` sort order, a hash map's iteration order, or min/max canonical key for builder staging. |
| Duplicate suppression compares exact packed pair dword 0 only; first inserted entry keeps position and flag. Active in YR: Yes. | `0x00582635..0x00582643`; append writes `0x00582687..0x0058268C`; `FUN_0058AF80` | missing | `src/sim/pathfinding/zone_build.rs`; `src/sim/pathfinding/zone_hierarchy.rs` tests | Within one bucket, skip only exact packed-pair duplicates and never update an existing entry's flag. | A scanline entry `(A<<16)|B` with flag 1 followed by same bridge entry flag 0 keeps flag 1 and original position. Proposed test name: `zone_hierarchy_exact_duplicate_keeps_first_entry_and_flag` | Do not dedup as undirected; do not replace an existing scanline edge with a later bridge edge. |
| Final emission drains bucket `0..255`, entry insertion order, then emits low-halfword directed edge before high-halfword reverse. Active in YR: Yes. | `0x0058236E..0x0058248A`; `0x00582395..0x0058245B` | missing/partial: `ZoneLevelGraph` can store ordered edges, but no production finalizer exists | `src/sim/pathfinding/zone_build.rs`; `src/sim/pathfinding/zone_hierarchy.rs` | Finalize temp entries into ordered per-zone `Vec<ZoneEdgeRecord>` exactly in bucket/entry order, appending two directed records per temp entry. | A temp entry `(5<<16)|2` produces `2 -> 5` before `5 -> 2`, and later bucket entries append after earlier buckets. Proposed test name: `zone_hierarchy_finalizer_emits_low_halfword_edge_before_reverse` | Do not sort final neighbor lists or run a final global dedup pass. |

## Negative Facts / Do Not Do

- Do not build hierarchy adjacency directly from row-major neighbor discovery order. Evidence: final adjacency drains 256 temp buckets in numeric order at `0x0058247D..0x0058248A`; Active in YR: Yes.
- Do not use undirected canonical `(min,max)` as the temp duplicate key. Evidence: duplicate compares exact packed pair dword 0 at `0x0058263C`; Active in YR: Yes.
- Do not update an existing duplicate's flag. Evidence: duplicate hit jumps over append/write path; Active in YR: Yes.
- Do not sort final per-zone neighbors by `ZoneId`. Evidence: final writer appends in bucket/insertion order without sort at `0x00582395..0x00582480`; Active in YR: Yes.
- Do not treat bridge/tube insertion as a separate unordered post-process for parity hierarchy. Evidence: bridge/tube entries are inserted into the same temp bucket surface before final emission in `ZoneMap__BuildZoneLevel`; Active in YR: Yes.

## Remaining Uncertainty

- Exact runtime route impact of bucket collisions and duplicate first-wins behavior on a stock map remains untraced; static writer order is implementation-ready, but route oracle tests need runtime path logs.

## Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/BRIDGE_ZONE_PRECHECK_HIERARCHY_WRITER_ORDER_GHIDRA_REPORT.md`
  - Add after the existing temp-bucket finding: "The exact temp bucket key is `((packed >> 16) & 0xF) << 4 | (packed & 0xF)`. Duplicate suppression compares only temp entry dword 0, the full directed packed pair. Reversed endpoints are distinct; first inserted duplicate keeps position and flag."
- `C:/Users/enok/Documents/ra2-rust-game/docs/plans/2026-05-24-production-flat-bridge-zone-hierarchy-activation-plan.md`
  - Replace Task 2's "If the exact temp-bucket key cannot be derived..." gate with: "Use the verified key `((high & 0xF) << 4) | (low & 0xF)` for packed pair `(high << 16) | low`; exact duplicate suppression is packed-pair dword 0 only, preserving the first entry and flag."

## Sources

- Ghidra decompiled: `FUN_00567110 @ 0x00567110`, `ZoneMap__BuildZoneLevel @ 0x00581F90`, `ZoneMap__FloodFillScanline @ 0x005824A0`, `FUN_00582D70 @ 0x00582D70`, `FUN_0058AF80 @ 0x0058AF80`.
- Ghidra assembly contexts: `0x00582604..0x00582631`, `0x00582635..0x00582643`, `0x00582687..0x0058268C`, `0x0058236E..0x0058248A`, `0x00582395..0x0058245B`.
- Existing docs referenced: `C:/Users/enok/Documents/ra2-rust-game-docs/BRIDGE_ZONE_PRECHECK_HIERARCHY_WRITER_ORDER_GHIDRA_REPORT.md`.
- Plan referenced: `C:/Users/enok/Documents/ra2-rust-game/docs/plans/2026-05-24-production-flat-bridge-zone-hierarchy-activation-plan.md`.
- Rust scanned: `src/sim/pathfinding/zone_hierarchy.rs`, `src/sim/pathfinding/zone_build.rs`, `src/sim/pathfinding/zone_map.rs`.
