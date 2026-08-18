# TIBTRE Retail SHP Frame Counts And Midpoint Ticks - Ghidra Research Report

**Address(es):** `0x0071C730` (`TerrainClass::AI`), `0x00426630` (`CDTimerClass::GetTimeRemaining`), `0x005F9070` (object image load helper), `0x0071DEA0` (`TerrainTypeClass::ReadINI_Full`)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** Exact retail SHP filenames, sources, frame counts, and stock `AnimationRate=3` midpoint tick conversion for `TIBTRE01`, `TIBTRE02`, and `TIBTRE03` in standard YR theaters.
**Non-Scope:** TIBTRE placement gates, tiberium type selection, existing-ore behavior, growth queues, terrain-object damage/lifecycle, savegame animation-counter persistence, nonstandard modded theater fallback behavior.
**Confidence:** High for retail asset frame counts and timing math.
**Active in YR:** Yes. Stock `rulesmd.ini` registers `TIBTRE01`, `TIBTRE02`, and `TIBTRE03`; `artmd.ini`/`art.ini` mark them `Theater=yes`; standard YR maps can place these terrain objects.

## Working Notes

Target question: What exact retail SHP frame counts are loaded for `TIBTRE01/02/03`, and what do those counts mean for `TerrainClass::AI` midpoint spawn timing under stock `AnimationRate=3`?

Non-goals: Do not re-investigate `SpreadTiberium`, `CanPlaceTiberium`, `PlaceTiberium`, terrain light keys, AnimClass ore spawning, or Rust implementation patches.

Evidence needed to mark COMPLETE: retail asset filename/source plus parser output for every standard theater variant; INI proof that the objects use theater-specific image names and rate 3; binary proof that midpoint uses loaded image-data `frame_count / 2`; tick conversion from probability-hit tick to spawn tick.

Stop conditions: all TIBTRE standard theater variants checked; generic `.SHP` fallback presence checked; no Ghidra mutation; write only this report plus `.swarm-claims.md`.

## 1. Overview

All retail standard-YR TIBTRE theater variants have the same SHP header: canvas `84x56`, `22` frames. `TerrainClass::AI` compares current animation frame against `frame_count / 2`, so the live midpoint frame for stock TIBTRE is `22 / 2 = 11`.

With stock `AnimationRate=3`, a probability hit does not spawn ore immediately. It starts the terrain animation on hit tick `H`, then the frame increments once every 3 logic ticks. The midpoint spawn call happens on the 11th expiry, `H + 33`, assuming the object receives one `TerrainClass::AI` call per logic frame.

## 2. Asset Resolution And Retail Frame Counts

Stock art data:

| Source | Section | Keys | Active in YR |
|---|---|---|---|
| `ini/artmd.ini:12653..12663` | `[TIBTRE01]`, `[TIBTRE02]`, `[TIBTRE03]` | `Theater=yes`, `Foundation=1x1` | Yes |
| `ini/art.ini:8575..8585` | `[TIBTRE01]`, `[TIBTRE02]`, `[TIBTRE03]` | base fallback also `Theater=yes`, `Foundation=1x1` | Yes |
| `ini/rulesmd.ini:28109..28152` | `[TIBTRE01]`, `[TIBTRE02]`, `[TIBTRE03]` | `SpawnsTiberium=yes`, `IsAnimated=yes`, `AnimationRate=3`, `AnimationProbability=.003`, `Immune=yes` | Yes |

Read-only binary spot-check:

| Behavior | Evidence | Active in YR |
|---|---|---|
| `TerrainTypeClass::ReadINI_Full` reads `SpawnsTiberium`, `IsAnimated`, `AnimationRate`, and `AnimationProbability` for terrain types | `0x0071DEA0` decompile | Yes |
| Object image loading honors the `Theater=` branch by formatting the current theater extension into the filename | `0x005F9070` decompile; also consistent with `BRIDGE_BODY_ASSET_RESOLUTION_GHIDRA_REPORT.md` section 3.2 | Yes |
| `TerrainClass::AI` calls the image-data getter and reads signed word `[image_data + 6]` as frame count | `0x0071C730` decompile | Yes |

Retail asset probe method:

- Retail root: `C:/Users/enok/Documents/Command and Conquer Red Alert II/`
- Existing repo readers used: `AssetManager::new`, `AssetManager::load_all_disk_mixes`, `AssetManager::load_nested`, `ShpFile::from_bytes`.
- Theater mixes explicitly loaded for the probe: `temperat.mix`, `snow.mix`, `urban.mix`, `urbann.mix`, `desert.mix`, `lunar.mix` plus their iso/md companions where present.
- Parser cross-check: raw SHP header word at offset `+6` matched `ShpFile::frames.len()` for every file below.

| Theater | Extension | Retail file loaded | Source archive reported by probe | Bytes | Header canvas | Header frame count | Parsed frames | Active in YR |
|---|---|---|---|---:|---|---:|---:|---|
| TEMPERATE | `.tem` | `TIBTRE01.tem` | `nested:temperat.mix` | 17776 | `84x56` | 22 | 22 | Yes |
| TEMPERATE | `.tem` | `TIBTRE02.tem` | `nested:temperat.mix` | 17776 | `84x56` | 22 | 22 | Yes |
| TEMPERATE | `.tem` | `TIBTRE03.tem` | `nested:temperat.mix` | 17776 | `84x56` | 22 | 22 | Yes |
| SNOW | `.sno` | `TIBTRE01.sno` | `nested:snow.mix` | 17776 | `84x56` | 22 | 22 | Yes |
| SNOW | `.sno` | `TIBTRE02.sno` | `nested:snow.mix` | 17776 | `84x56` | 22 | 22 | Yes |
| SNOW | `.sno` | `TIBTRE03.sno` | `nested:snow.mix` | 17776 | `84x56` | 22 | 22 | Yes |
| URBAN | `.urb` | `TIBTRE01.urb` | `nested:urban.mix` | 17776 | `84x56` | 22 | 22 | Yes |
| URBAN | `.urb` | `TIBTRE02.urb` | `nested:urban.mix` | 17776 | `84x56` | 22 | 22 | Yes |
| URBAN | `.urb` | `TIBTRE03.urb` | `nested:urban.mix` | 17776 | `84x56` | 22 | 22 | Yes |
| NEWURBAN | `.ubn` | `TIBTRE01.ubn` | `nested:urbann.mix` | 17776 | `84x56` | 22 | 22 | Yes |
| NEWURBAN | `.ubn` | `TIBTRE02.ubn` | `nested:urbann.mix` | 17776 | `84x56` | 22 | 22 | Yes |
| NEWURBAN | `.ubn` | `TIBTRE03.ubn` | `nested:urbann.mix` | 17776 | `84x56` | 22 | 22 | Yes |
| DESERT | `.des` | `TIBTRE01.des` | `nested:desert.mix` | 17776 | `84x56` | 22 | 22 | Yes |
| DESERT | `.des` | `TIBTRE02.des` | `nested:desert.mix` | 17776 | `84x56` | 22 | 22 | Yes |
| DESERT | `.des` | `TIBTRE03.des` | `nested:desert.mix` | 17776 | `84x56` | 22 | 22 | Yes |
| LUNAR | `.lun` | `TIBTRE01.lun` | `nested:lunar.mix` | 17776 | `84x56` | 22 | 22 | Yes |
| LUNAR | `.lun` | `TIBTRE02.lun` | `nested:lunar.mix` | 17776 | `84x56` | 22 | 22 | Yes |
| LUNAR | `.lun` | `TIBTRE03.lun` | `nested:lunar.mix` | 17776 | `84x56` | 22 | 22 | Yes |

Generic `.SHP` check:

| Candidate set | Probe result | Meaning |
|---|---|---|
| `TIBTRE01.SHP`, `TIBTRE02.SHP`, `TIBTRE03.SHP` | missing from the loaded retail archive stack | Standard retail YR TIBTRE uses theater-extension files, not generic `.SHP`, for these objects. |

## 3. Midpoint Timing Math

Binary facts:

1. A successful idle probability roll writes current frame `+0xAC = 0`, timer start `+0xB4 = g_CurrentFrameCounter`, duration `+0xBC = AnimationRate`, and active mirror `+0xC0 = AnimationRate`. Evidence: `TerrainClass::AI @ 0x0071C730`.
2. `CDTimerClass::GetTimeRemaining @ 0x00426630` expires when `g_CurrentFrameCounter - start >= duration`; otherwise it returns the remaining frame count.
3. On each expiry while active, `TerrainClass::AI` increments current frame by constructor-initialized step `+0xC4 = 1`, then rearms the timer. Evidence: `TerrainClass::AI @ 0x0071C730`, constructor `0x0071BB90`.
4. TIBTRE spawn occurs only if current frame equals `(int)*(short *)(image_data + 6) / 2`. Evidence: `TerrainClass::AI @ 0x0071C730`.
5. The retail TIBTRE frame count is `22`, so the midpoint comparison target is frame `11`.

Stock timing conversion:

| Symbol | Value | Evidence |
|---|---:|---|
| `frame_count` | 22 | retail SHP parser output above |
| midpoint frame | 11 | binary signed integer division by 2 at `0x0071C730` |
| `AnimationRate` | 3 | `rulesmd.ini:28119`, `28134`, `28149`; binary read at `0x0071DEA0` |
| frame increment interval | 3 logic ticks | `CDTimerClass::GetTimeRemaining @ 0x00426630` |
| expiries before spawn | 11 | current frame starts at 0 and increments by 1 per expiry |
| spawn delay after hit tick | 33 logic ticks | `11 * 3` |

Timeline under stock rate 3:

| Tick relative to probability hit `H` | Timer result / action | Current frame after action | Spawn? |
|---:|---|---:|---|
| `H` | probability hit starts timer; elapsed 0, remaining 3 | 0 | No |
| `H+1` | remaining 2 | 0 | No |
| `H+2` | remaining 1 | 0 | No |
| `H+3` | expiry; increment | 1 | No |
| `H+6` | expiry; increment | 2 | No |
| `...` | one increment every 3 ticks | `...` | No |
| `H+30` | expiry; increment | 10 | No |
| `H+33` | expiry; increment to midpoint, reset, call `SpreadTiberium(1)` | reset to 0 before call | Yes |

## 4. Current Rust Implementation Status

As of the 2026-05-24 TIBTRE implementation pass, current Rust has been updated to carry loaded terrain frame counts into the terrain-spawner state machine while preserving the `sim/` layering boundary.

| Rust surface | Current behavior | Delta from verified behavior |
|---|---|---|
| `src/sim/terrain_spawn.rs` file comment | documents the two-phase model | no known stale timing wording in this file |
| `TerrainSpawnerState` | stores type, probability, animation rate, loaded frame count, midpoint frame, and idle/active phase | no timing/frame-count field gap known in this slice |
| `tick_terrain_spawners_stateful` | rolls probability only while idle, arms animation on hit, suppresses active rolls, and delays placement RNG/spawn until midpoint | matches the verified frame-count timing contract for this slice |
| seeding from terrain object type | accepts `terrain_frame_counts` from app/render-side asset loading | preserves `sim/` layering; modded asset parity depends on the atlas supplying the loaded frame count |

## 5. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| TIBTRE stock rules flags/rate | verified | `rulesmd.ini:28109..28152`; `0x0071DEA0` | none |
| TIBTRE art `Theater=yes` | verified | `artmd.ini:12653..12663`, `art.ini:8575..8585`; `0x005F9070` | exact failure fallback order for missing mod assets is outside this report |
| Standard YR theater extension set | verified | repo theater table and retail probe: `.tem/.sno/.urb/.ubn/.des/.lun` | none for standard YR |
| Retail TIBTRE01 all standard theaters | verified | parser output table | none |
| Retail TIBTRE02 all standard theaters | verified | parser output table | none |
| Retail TIBTRE03 all standard theaters | verified | parser output table | none |
| Generic `TIBTRE*.SHP` presence | verified | retail probe missed all three names in loaded archive stack | none |
| Binary frame-count source | verified | `TerrainClass::AI @ 0x0071C730` reads `[image_data+6]` | none |
| Timer expiry boundary | verified | `CDTimerClass::GetTimeRemaining @ 0x00426630` | none |
| Spawn tick conversion | verified | asset count 22 plus binary midpoint/rate facts | none |
| Current Rust frame-count/timing surface | verified-source-scan | `src/sim/terrain_spawn.rs`, `src/app_init.rs` | native placement variant/queue effects are outside this report |

## 6. Open Questions - Final State of Investigation Log

- `[RESOLVED] OQ-1 - Which image files are used for TIBTRE in standard YR theaters? -> Theater-extension files: `.tem`, `.sno`, `.urb`, `.ubn`, `.des`, `.lun`.` (evidence: `artmd.ini`, `0x005F9070`, retail parser output)
- `[RESOLVED] OQ-2 - Do TIBTRE01/02/03 have different frame counts? -> No; every checked retail standard-theater variant has 22 frames.` (evidence: retail parser output table)
- `[RESOLVED] OQ-3 - Do theater variants differ by theater? -> Not for header size/count; every checked variant is `84x56`, 22 frames, 17776 bytes.` (evidence: retail parser output table)
- `[RESOLVED] OQ-4 - Is a generic `.SHP` version present and possibly used for stock TIBTRE? -> No generic `TIBTRE01/02/03.SHP` was found in the loaded retail archive stack.` (evidence: retail parser output)
- `[RESOLVED] OQ-5 - What does `TerrainClass::AI` use as frame count? -> Signed 16-bit SHP header word at image data offset `+6`.` (evidence: `0x0071C730`)
- `[RESOLVED] OQ-6 - What midpoint frame does 22 produce? -> `11`.` (evidence: `0x0071C730` division by 2 plus retail frame count)
- `[RESOLVED] OQ-7 - Under stock `AnimationRate=3`, how many expiries before spawn? -> 11 expiries.` (evidence: `0x0071C730`, `0x00426630`)
- `[RESOLVED] OQ-8 - Under stock `AnimationRate=3`, how many logic ticks after the hit until spawn? -> 33 logic ticks, assuming one AI call per logic frame.` (evidence: `0x00426630`; `rulesmd.ini`)
- `[RESOLVED] OQ-9 - Can hit and spawn happen on the same stock tick? -> No; elapsed 0 is less than duration 3 on hit tick.` (evidence: `0x00426630`)
- `[RESOLVED] OQ-10 - Does current Rust have the needed fields? -> Yes for this timing slice as of the 2026-05-24 TIBTRE implementation pass: `TerrainSpawnerState` stores frame count, rate, midpoint, and active/idle phase, and seeding accepts frame counts from app/render-side loading.` (evidence: `src/sim/terrain_spawn.rs`, `src/app_init.rs`)
- `[DEFERRED] OQ-11 - Exact binary source of each theater extension string/table entry.` (category: `out-of-scope`; reason: existing binary and repo evidence is enough for standard asset names; exact table offsets are not needed for the frame-count/tick handoff; next-step-if-pursued: trace scenario theater index string table around the formatter used by `0x005F9070`)
- `[DEFERRED] OQ-12 - Whether frame 11 is ever visually presented before the reset on the same AI call.` (category: `requires-different-system-context`; reason: this report scopes spawn timing, not terrain draw ordering after AI; next-step-if-pursued: trace terrain draw read of current frame relative to logic tick presentation)
- `[DEFERRED] OQ-13 - Save/load restoration of active animation counters.` (category: `out-of-scope`; reason: previous lifecycle slot explicitly deferred binary savegame serialization; next-step-if-pursued: investigate TerrainClass save/load fields for `+0xAC..+0xC0`)

## 7. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Retail TIBTRE frame count is 22 in every standard YR theater variant | retail parser output; `artmd.ini` `Theater=yes` | implemented through frame-count handoff | `src/sim/terrain_spawn.rs` seeding and app/render-side handoff surface | seed terrain spawner metadata with loaded frame counts without adding asset dependencies to `sim/` | `tibtre_retail_frame_count_is_22_for_all_standard_theaters` | do not hardcode only by `TIBTRE01` name if modded assets can override art/image data |
| Stock midpoint spawn target is frame 11 | `TerrainClass::AI @ 0x0071C730`; retail count 22 | implemented by `midpoint_frame = frame_count / 2` | `TerrainSpawnerState`, `tick_terrain_spawners_stateful` | spawn only when active animation increments to frame `frame_count / 2` | `tibtre_22_frame_asset_spawns_on_frame_11` | do not treat `AnimationRate` as frame count |
| Stock rate 3 delays spawn until 33 logic ticks after a successful probability hit | `rulesmd.ini`, `0x00426630`, `0x0071C730` | implemented by active timer/rate state | `tick_terrain_spawners_stateful`, RNG consumer sequence | on hit tick arm timer; no placement RNG until tick `H+33`; reset active state before/around placement call consistent with binary | `tibtre_stock_rate3_spawns_33_ticks_after_probability_hit` | do not collapse to average rate or keep rolling probability while active |

### Stale Docs / Follow-up Docs

- `TIBTRE_TERRAINCLASS_AI_TIMING_AND_RNG_GHIDRA_REPORT.md` should use this resolved wording: "Retail `TIBTRE01/02/03` standard-theater SHPs are all 22 frames, so the midpoint spawn comparison targets frame 11; with stock `AnimationRate=3`, spawn occurs 33 logic ticks after the probability-hit tick."
- The previous note that `src/sim/terrain_spawn.rs` had a stale collapsed-model header is no longer current after the 2026-05-24 implementation pass.

## 8. Negative Facts / Do Not Do

- Do not use generic `TIBTRE01.SHP`, `TIBTRE02.SHP`, or `TIBTRE03.SHP` for stock standard YR; retail probe found theater-extension files and no generic SHP entries.
- Do not assume different midpoint timing per standard theater; all checked retail standard-theater variants have the same 22-frame count.
- Do not spawn on frame 10 for a 22-frame file; binary uses integer `frame_count / 2`, which is 11.
- Do not count the probability-hit tick as the first frame-advance expiry under stock rate 3; elapsed 0 is not expired.
- Do not store this metadata by making `sim/` depend directly on `assets/` or `render/`; pass resolved frame counts into sim state at setup.

## 9. Remaining Uncertainty

- Exact binary table offsets for standard theater extension strings were not re-derived in this slot; standard extension behavior is covered by `0x005F9070`, existing bridge asset-resolution research, repo theater definitions, and successful retail asset lookup.
- Terrain draw ordering for the transient midpoint frame before reset was not traced. This does not affect the spawn tick, but it matters if a future visual parity task models the TIBTRE animation itself.
- Save/load persistence of an in-progress terrain animation remains deferred to a terrain serialization investigation.

## Sources

- Ghidra read-only decompilation: `TerrainClass::AI @ 0x0071C730`, `CDTimerClass::GetTimeRemaining @ 0x00426630`, `FUN_005F9070`, `TerrainTypeClass::ReadINI_Full @ 0x0071DEA0`, `TerrainClass` constructor `0x0071BB90`.
- Retail install: `C:/Users/enok/Documents/Command and Conquer Red Alert II/`.
- Retail parser output from existing repo readers: `AssetManager` plus `ShpFile::from_bytes`.
- INI: `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini`, `ini/artmd.ini`, `ini/art.ini`.
- Prior research: `TIBTRE_TERRAINCLASS_AI_TIMING_AND_RNG_GHIDRA_REPORT.md`, `BRIDGE_BODY_ASSET_RESOLUTION_GHIDRA_REPORT.md`.
- Rust comparison: `C:/Users/enok/Documents/ra2-rust-game/src/sim/terrain_spawn.rs`.
