# BuildingLightClass Beam Rasterization and CellAction 0x23 -- Ghidra Research Report

**Address(es):** `0x00435C10`, `0x005FF250`, `0x005FF850`, `0x005FF2D0`, `0x004BBCA0`, `0x004361D0`, `0x006E53A0`, `0x007264C0`, `0x0071E940`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** deferred Q17/Q18 from `BUILDINGLIGHTCLASS_SPOTLIGHT_PATH_GHIDRA_REPORT.md`: the visible `BuildingLightClass` beam/glow primitive path and the live effect of spotlight-triggered cell action `0x23`.  
**Non-Scope:** ordinary `LightSourceClass` radius lamps, map ambience, `ExtraLight`, `LightConvert` profiles, combat lights, particle lights, superweapon lighting, and full trigger/action taxonomy beyond event `0x23`.  
**Confidence:** High for path identity, call ordering, main pixel operation, and trigger-event side effect. Medium for screenshot-level perceptual parity because no runtime capture was taken in this slot.  
**Active in YR:** Conditional. The code is live in `gamemd.exe`, but it only runs when a placed building has `HasSpotlight=yes` and passes the operational gates; stock repo INI has no `HasSpotlight=` assignment.

## Working Notes Seed

- Target question: What exact visible primitive path does `BuildingLightClass` use for spotlight/searchlight pixels, and what does `ProcessCellAction(0x23)` do when the sweep finds an enemy?
- Non-goals: Do not re-cover map ambience, point lamps, `ExtraLight`, ordinary `LightConvert`, combat lights, particle lights, or superweapon ambient transitions.
- Evidence needed to mark COMPLETE: decompile plus disassembly/range evidence for `0x00435C10`, `0x005FF250`, `0x005FF850`, `0x005FF2D0`, `0x004BBCA0`, `0x004361D0`, `0x006E53A0`, and `0x007264C0/0x0071E940`; Rust and INI surface scans.
- Stop conditions: all Q17/Q18 entries resolved or explicitly deferred, no Rust/INI edits, one report plus the shared claims row only.

## 1. Overview

`BuildingLightClass` draws a spotlight as two distinct visual pieces. First, it creates a temporary 24-byte light/glow primitive at the current beam endpoint and immediately rasterizes/frees it. Second, it draws the beam body as two brightening line segments through the primary `DSurface` line routine. This is not cell ambience and not a colored point-light contribution.

The `0x23` cell action is also live but narrow: spotlight AI calls the generic attached-trigger/event processor on the owning building's tag/controller pointer. Event `0x23` is matched by exact event ID in `TriggerActionEntry__EvaluateConditions`; it does not directly damage, reveal, tint, or redraw anything by itself.

## 2. Key Objects / Fields

| Field / object | Behavior | Evidence | Active in YR |
|---|---|---|---|
| `BuildingLightClass+0x9C..0xA4` | Current beam endpoint passed to the glow primitive constructor. | `0x00435CA1..0x00435CC0` pushes endpoint coord and size `0x10` into `0x005FF250`. | Conditional |
| Glow primitive size `0x18` | Small heap object used by the light/glow raster helper. | `operator_new(0x18)` at `0x00435C93`, constructor body `0x005FF250..0x005FF2CC`. | Conditional |
| Glow primitive `+0x0/+0x4/+0x8` | Coordinate copied from constructor stack args. | `0x005FF25D`, `0x005FF263`, `0x005FF26E`. | Conditional |
| Glow primitive `+0xC` | Intensity/color index; initialized `0`, then spotlight draw writes `0x50` or clamped mode-3 value. | init `0x005FF268`; writes `0x00435DCC` or `0x00435DDB`. | Conditional |
| Glow primitive `+0x10` | Size/scale argument; spotlight passes `0x10`. | `0x00435CA1`, store `0x005FF271..0x005FF275`. | Conditional |
| Glow primitive `+0x14` | Flags/control word; constructor clears it. | `0x005FF26B`; raster reads low nibble at `0x005FF860..0x005FF866`. | Conditional |
| `DSurface vtable +0x38` | Concrete line/brighten worker `0x004BBCA0`. | Prior surface-vtable report plus vtable bytes `0x007E85D4+0x38 -> 0x004BBCA0`; decompile `0x004BBCA0`. | Yes |
| `DSurface vtable +0x78` | Full-surface bounds/clip helper `0x00411510`, used immediately before the line worker. | Prior surface-vtable report; spotlight calls `+0x78` at `0x0043615B` and `0x004361AF`. | Yes |
| Owner building `+0x34` | Pointer passed as `this` to the generic cell-action/event processor. | `0x004368C5..0x004368D4` moves `[owner+0x34]` to `ECX` then calls `0x006E53A0`. | Conditional |

## 3. Core Logic

### 3.1 Draw ordering and visible composition

The visible composition order in `0x00435C10` is:

1. Validate owner, operational gate, disabled byte, and optional scenario visibility gate. Active in YR: Conditional. Evidence: `0x00435C10..0x00435C8D`.
2. Allocate `0x18` bytes, construct glow primitive at beam endpoint with size `0x10`, and append it to global vector `0x00AC1678` when capacity allows. Active in YR: Conditional. Evidence: `0x00435C93..0x00435CC0`, `0x005FF250..0x005FF2CC`.
3. Compute distance bucket from owner to spotlight using `[Rules]+0x78C` and `[Rules]+0x788`, with the same `/10` denominator pattern as the parent report. Active in YR: Conditional. Evidence: `0x00435D5E..0x00435DA0`.
4. For mode `3` and endpoint distance beyond `SpotlightLocationRadius`, write glow index `clamp_nonnegative(bucket + 0x50, max 0x59)`. Otherwise write constant `0x50`. Active in YR: Conditional. Evidence: `0x00435DA4..0x00435DDB`.
5. Call `0x005FF850` to rasterize the endpoint glow, remove the primitive from the global vector with `0x005FF2D0`, then free the heap block with `0x007C8B3D`. Active in YR: Conditional. Evidence: `0x00435DE2..0x00435DFA`.
6. Compute two side points for the beam body using owner coord, endpoint coord, distance/intensity radius, matrix rotation, tactical projection, radar viewport offsets, and Z adjustment. Active in YR: Conditional. Evidence: `0x00435E22..0x0043611D`.
7. Submit two line segments through primary `DSurface +0x78` then `+0x38`. Both calls use brightness factor `0x4B - 6 * bucket`, Z arguments from `Tactical__AdjustForZ`, and final arg `0`. Active in YR: Conditional. Evidence: first call `0x00436123..0x00436165`, second call `0x0043617B..0x004361B9`.

### 3.2 Endpoint glow helper details

`0x005FF250` is a constructor/registration helper, not a beam-line rasterizer. It fills the 24-byte primitive, clears `+0xC` and `+0x14`, stores the size/scale at `+0x10`, and appends the pointer to the global vector at `0x00AC1678` if capacity permits. Active in YR: Conditional. Evidence: `0x005FF250..0x005FF2CC`.

`0x005FF850` is the endpoint glow rasterizer. It clips/converts the primitive through `0x006D2140`, rejects under the scenario visibility gate via `0x005865E0`, maps the `+0xC` index through signed table `0x0083358C`, optionally scales by `+0x10` when the table value is below `0x40`, selects a palette/convert entry through `0x00AC1698`, and draws into a surface buffer via `0x007BC040` plus either specialized pixel blitters or inline 16-bit pixel loops. Active in YR: Conditional. Evidence: `0x005FF850..0x005FFF81`.

Important boundary details:

- If the primitive low-nibble flags are `0` or `1`, the rasterizer performs an age/detail gate using `0x0055AF60` and global `0x00ABCD44`; nonzero low-nibble values other than `1` skip that gate. Evidence: `0x005FF860..0x005FF87F`. Active in YR: Conditional.
- For spotlight-created primitives, `+0x14` starts at zero and is not modified before the endpoint raster call in `0x00435C10`, so the low-nibble path is the zero path. Evidence: `0x005FF26B`, `0x00435DE2`. Active in YR: Conditional.
- `0x005FF2D0` removes the primitive pointer from the global vector by lookup through vector slot `+0x10`, decrements count, and shifts later entries left; it does not draw. Evidence: `0x005FF2D0..0x005FF31B`. Active in YR: Conditional.

### 3.3 Beam body line worker details

The beam body is not produced by `0x005FF250/850/2D0`. It is submitted after endpoint-glow teardown as two primary-surface line calls. The concrete `DSurface +0x38` target is `0x004BBCA0`; this worker:

- obtains/intersects the destination clip through `DSurface +0x78` and `AlphaShapeClass__ClipRect`;
- early-outs if clipped width/height are zero;
- clips line endpoints before rasterization;
- locks/queries the destination surface through vtable `+0x70`, `+0x5C`, `+0x74`, and unlocks through `+0x60`;
- uses Bresenham-like major-axis loops over 16-bit destination pixels;
- when `g_ZBuffer != 0`, compares the line depth against the z-buffer and optionally writes z only when the caller's final flag is nonzero;
- brightens existing destination RGB channels by `channel + ((channel * intensity) >> 8)` and clamps each channel to `0xFF` before repacking through DirectDraw pixel-format shift/loss globals.

Active in YR: Conditional on spotlight draw reaching the calls. Evidence: `0x004BBCA0` decompile, call sites `0x00436123..0x00436165` and `0x0043617B..0x004361B9`, vtable bytes `0x007E85D4+0x38 -> 0x004BBCA0`.

The beam therefore brightens the already-rendered scene; it is not a fixed-color SHP, not a palette-indexed cell tint, and not a `LightSourceClass` ambience contribution.

### 3.4 Cell action `0x23`

In mode `1`, spotlight AI searches nearby cells after computing a detection radius `ftol(bucket * const) + Rules.SpotlightRadius + 0x1E`. It scans a 3x3 cell area around the spotlight-relevant cell, tests each object for RTTI `0xF` or `1`, rejects allied objects via `0x004F9A90`, and sets a local hit flag if any accepted object is closer than the computed threshold. Active in YR: Conditional. Evidence: `0x0043676C..0x004368BD`.

If the hit flag is set, the AI calls `0x006E53A0` with:

| Argument | Value at spotlight call | Evidence |
|---|---|---|
| `this` | `[owner building + 0x34]` | `0x004368C5..0x004368D4` |
| arg1 | `0x23` | `push 0x23` at `0x004368D2` |
| arg2 | owner building pointer | `push eax` at `0x004368CD..0x004368D1` |
| arg3 | global `0x0089C4F0` | `0x004368BF..0x004368CD` |
| arg4 | `0` | `0x004368CB` |
| arg5 | `0` | `0x004368C9` |

`0x006E53A0` is the generic attached-trigger/event processor. It is suppressed in the map editor and when the object is already processing or disabled (`+0x35` / `+0x34` guard bytes). It requires a non-null trigger/type pointer at `this+0x24`; if missing, it returns false. Active in YR: Yes as generic code; conditional from spotlight. Evidence: `0x006E53A0..0x006E53D7`.

For each linked trigger/action entry from `this+0x28`, `0x006E53A0` calls `0x007264C0`. In `0x007264C0`, the event-condition evaluator reaches `0x0071E940`. Event `0x23` maps through the first event table to the generic-event branch `0x0071EC63`, then the second table maps event `0x23` to exact event-ID comparison at `0x0071EC9A`; the comparison requires caller arg1 (`0x23`) to equal the trigger event type. Active in YR: Conditional on an attached trigger event `0x23`. Evidence: jump table bytes at `0x0071F248` for event `0x23 -> index 11 -> 0x0071EC63`; second table `0x0071F284` maps `0x23 -> 0x0071EC9A`; equality check `0x0071EC9A..0x0071ECA9`.

After the equality match, event `0x23` falls through the generic object/house checks and reaches the success return path when the referenced house/event context is valid. It does not enter the special distance branch for event `0x22`, the ownership branches `0x35/0x36`, or any direct damage/reveal/tint code. Active in YR: Conditional. Evidence: `0x0071ECAF..0x0071F20C`; event `0x23` maps to final table entry `0x0071F1B1`, which returns true.

When an entry evaluates true, `0x006E53A0` applies normal trigger side effects based on trigger type field `[this+0x24]+0x9C`: type `0` and satisfied type `1` call play-voice/object action helper `0x007265C0`, enqueue/mark the trigger entry through `0x00726720`, and mark the owner for detach/list processing; type `2` plays voice/actions and returns true without the same enqueue flag. Active in YR: Conditional on trigger type and event entry. Evidence: `0x006E541D..0x006E5484`, `0x007265C0`, `0x00726720`.

## 4. INI / Data Activation

| Key / data | Status | Evidence | Active in YR |
|---|---|---|---|
| `HasSpotlight=` | Live parser and allocation gate from parent report; no repo INI assignments. | Parent report; current `rg "HasSpotlight"` over repo INI returned no matches. | Conditional |
| `[General] SpotlightRadius` | Used in detection/beam-radius threshold even if absent from repo comments. | Parent report and `Rules+0x7A8` reads at `0x0043677B`, `0x00435E95`. | Conditional |
| Trigger event `0x23` | Binary-supported generic event type; requires attached trigger data. | `0x004368D2`, `0x006E53A0`, `0x007264C0`, `0x0071E940`. | Conditional |

## 5. Integration Points

| Integration | Verified behavior | Evidence | Active in YR |
|---|---|---|---|
| Draw path | Endpoint glow is rasterized before beam body lines. | `0x00435C93..0x00435DFA`, then `0x00436123..0x004361B9`. | Conditional |
| Surface path | Beam body is two `DSurface +0x38` brightening line submissions after `+0x78` clip helper. | `0x0043615B/0x00436165`, `0x004361AF/0x004361B9`, `0x004BBCA0`. | Conditional |
| Trigger path | Enemy-in-beam condition calls `0x006E53A0(0x23, owner, 0x0089C4F0, 0, 0)` on owner `+0x34`. | `0x004368BF..0x004368D4`. | Conditional |
| Trigger event evaluator | Event `0x23` is exact-match generic event, not a bespoke spotlight branch. | `0x0071F248`, `0x0071EC63`, `0x0071EC9A`. | Conditional |

## 6. Current Rust Implementation Status

Current Rust still has map ambience and point lights only:

- `src/map/lighting.rs:365` defines `PointLight`; `src/map/lighting.rs:385` collects building point lights from `LightIntensity`.
- `src/map/lighting.rs:454` accumulates point lights into the per-cell light grid.
- `src/app_init.rs:180` and `src/app_init.rs:393` rebuild/apply the map lighting grid.
- `src/rules/object_type.rs:706` and `src/rules/object_type.rs:1112` cover `LightVisibility` / `LightIntensity`.
- No `HasSpotlight`, `BuildingLightClass`, endpoint glow primitive, beam line brightener, or trigger-event `0x23` spotlight path was found in `src/`.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Prior Q17 `FUN_005FF250/850/2D0` role | verified | `0x00435C93..0x00435DFA`, `0x005FF250`, `0x005FF850`, `0x005FF2D0` | none for helper role |
| Endpoint glow primitive layout | verified | `0x005FF250..0x005FF2CC` | exact visual screenshot comparison deferred outside Ghidra slot |
| Endpoint glow intensity write | verified | `0x00435DA4..0x00435DDB` | none |
| Beam body surface calls | verified | `0x00436123..0x004361B9`, `0x004BBCA0` | none for main pixel operation |
| `DSurface +0x38` pixel operation | verified | `0x004BBCA0`; vtable `0x007E85D4+0x38` | no screenshot capture in this slot |
| Prior Q18 spotlight call site | verified | `0x004368BF..0x004368D4` | none |
| `0x006E53A0` generic event processor | verified | `0x006E53A0..0x006E554E` | full trigger taxonomy out of scope |
| Event `0x23` evaluator branch | verified | tables `0x0071F248`, `0x0071F284`; branch `0x0071EC63..0x0071F20C` | none for event `0x23` |
| Rust implementation surface | verified | `rg` over `src/`; `src/map/lighting.rs`, `src/app_init.rs`, `src/rules/object_type.rs` | implementation absent |
| Repo INI activation | verified | `rg "HasSpotlight" ini/*.ini` no matches | retail map archive scan out of scope |

## 8. Open Questions -- Final State

- `[RESOLVED] OQ-01 -- Does `0x005FF250` draw beam pixels? -> No; it constructs/registers a 24-byte glow primitive.` (evidence: `0x005FF250..0x005FF2CC`)
- `[RESOLVED] OQ-02 -- What fields does the glow primitive hold? -> coord at `+0/+4/+8`, intensity index `+0xC`, size `+0x10`, flags `+0x14`.` (evidence: `0x005FF25D..0x005FF275`)
- `[RESOLVED] OQ-03 -- When is the endpoint glow rasterized? -> Immediately after the spotlight draw writes the intensity index, before the beam body lines.` (evidence: `0x00435DCC..0x00435DFA`)
- `[RESOLVED] OQ-04 -- Is `0x005FF2D0` a draw helper? -> No; it removes the primitive from the global vector.` (evidence: `0x005FF2D0..0x005FF31B`)
- `[RESOLVED] OQ-05 -- What draws the beam body? -> Two primary `DSurface +0x38` brightening line calls after `+0x78` clip/bounds calls.` (evidence: `0x00436123..0x004361B9`, `0x004BBCA0`)
- `[RESOLVED] OQ-06 -- Does the beam body write fixed colors? -> No; `0x004BBCA0` brightens existing 16-bit destination RGB and clamps to `0xFF`.` (evidence: `0x004BBCA0`)
- `[RESOLVED] OQ-07 -- Does the beam body update z-buffer? -> The worker can update z when requested, but spotlight passes final arg `0`, so the line does not request z writes.` (evidence: pushes `0` at `0x0043613B` and `0x00436196`; z branch in `0x004BBCA0`)
- `[RESOLVED] OQ-08 -- What brightness value does spotlight pass? -> `0x4B - 6 * bucket`, with bucket from distance/rules division.` (evidence: `0x00436136..0x0043614D`, `0x00436184..0x004361A1`)
- `[RESOLVED] OQ-09 -- Is `ProcessCellAction(0x23)` live from spotlight AI? -> Yes, conditional on mode 1, live owner, operational gate, non-null owner trigger pointer, enemy-in-threshold hit.` (evidence: `0x00436641..0x004368D4`)
- `[RESOLVED] OQ-10 -- What arguments are passed for cell action `0x23`? -> `this=[owner+0x34]`, arg1 `0x23`, arg2 owner building, arg3 `0x0089C4F0`, arg4/arg5 zero.` (evidence: `0x004368BF..0x004368D4`)
- `[RESOLVED] OQ-11 -- Does event `0x23` have a bespoke spotlight branch? -> No; it maps to generic exact event-ID comparison and generic object/house checks.` (evidence: `0x0071F248`, `0x0071EC63`, `0x0071EC9A`)
- `[RESOLVED] OQ-12 -- What side effect occurs after event `0x23` matches? -> Normal trigger evaluation side effects: play voice/actions, mark/enqueue trigger entry or owner depending on trigger type; no direct visual/gameplay mutation is hardcoded for spotlight itself.` (evidence: `0x006E541D..0x006E554E`, `0x007265C0`, `0x00726720`)
- `[RESOLVED] OQ-13 -- Is the path standard YR or TS-only? -> The code is in live YR binary and not gated by TS-only fog/path flags; activation is content/trigger conditional because stock repo INI has no `HasSpotlight=` assignment.` (evidence: listed call sites; repo INI scan)

## 9. Visual Composition Ledger

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---|---|---|---|---|---|---|---|
| 1 | `0x005FF250` from `0x00435CC0` | Owner/draw gates passed; heap alloc succeeds | none | world coord `BuildingLightClass+0x9C..0xA4`, size `0x10` | none yet | conditional | endpoint primitive construction |
| 2 | `0x005FF850` from `0x00435DE4` | primitive exists; low flags zero; visibility/detail gates pass | internal alpha/convert tables | screen-space converted primitive footprint | table `0x0083358C` and `0x00AC1698` | conditional | endpoint glow |
| 3 | `0x005FF2D0` + `0x007C8B3D` | primitive pointer non-null | none | none | none | conditional | endpoint primitive removal/free |
| 4 | `DSurface +0x78 -> 0x00411510` then `+0x38 -> 0x004BBCA0` | `0x007BC2B0` clip test passes | none | owner projected point to beam side point A | brightens existing 16-bit pixels | conditional | first beam edge/body line |
| 5 | `DSurface +0x78 -> 0x00411510` then `+0x38 -> 0x004BBCA0` | second `0x007BC2B0` clip test passes | none | owner projected point to beam side point B | brightens existing 16-bit pixels | conditional | second beam edge/body line |

Asset role matrix:

| Asset | Loaded | Drawn | Visible in target | Content/preview | Chrome/container | Overlay | Transition-only | Inactive | Evidence |
|---|---|---|---|---|---|---|---|---|---|
| SHP/asset file | no evidence | no | no | no | no | no | no | yes | draw path uses primitive/surface helpers, not SHP loader calls |
| Endpoint glow primitive | runtime heap | yes | conditional | no | no | yes | no | no | `0x00435C93..0x00435DFA` |
| Beam body line | runtime surface line | yes | conditional | no | no | yes | no | no | `0x00436123..0x004361B9`, `0x004BBCA0` |

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Endpoint glow is a temporary 24-byte primitive at the beam endpoint, rasterized before beam body, then removed/freed. | `0x00435C93..0x00435DFA`, `0x005FF250`, `0x005FF850`, `0x005FF2D0` | missing | future render overlay/light-effect layer, not `src/map/lighting.rs::PointLight` | Draw a transient endpoint glow with mode-dependent index `0x50..0x59` and size `0x10`. | Custom map/mod fixture with one `HasSpotlight=yes` building shows a glow at the moving endpoint before/with beam body. Proposed test: `spotlight_endpoint_glow_uses_mode3_clamped_index_and_size_16`. | Do not model endpoint glow as cell RGB ambience. |
| Beam body is two `DSurface +0x38` line brighteners with factor `0x4B - 6 * bucket`, not a fixed-color sprite. | `0x00436123..0x004361B9`, `0x004BBCA0` | missing | render overlay line raster path | Brighten existing rendered pixels along two clipped screen lines; clamp RGB to `0xFF`; do not write z for spotlight calls. | Beam crossing varied terrain brightens underlying terrain colors rather than replacing them with a flat color. Proposed test: `spotlight_beam_lines_brighten_destination_pixels_without_z_write`. | Do not draw a yellow/white textured cone unless a later screenshot audit justifies an approximation layer. |
| Spotlight enemy detection fires generic trigger event `0x23` through owner `+0x34`, with args `(0x23, owner, 0x0089C4F0, 0, 0)`. | `0x004368BF..0x004368D4`, `0x006E53A0`, `0x007264C0`, `0x0071E940` | missing | sim/object trigger-event integration; render may expose only visual state | When mode-1 search finds a non-ally object within threshold, evaluate attached trigger events of type `0x23`; no direct spotlight damage/reveal side effect. | A scripted map with a `HasSpotlight=yes` building and attached event `0x23` trigger fires only when an enemy enters the spotlight threshold. Proposed test: `spotlight_search_fires_attached_trigger_event_23_once_enemy_enters_beam`. | Do not implement `0x23` as direct damage, shroud reveal, tint, or global trigger fire. |

### Negative Facts / Do Not Do

- Do not treat `0x005FF250` as the beam-body rasterizer. Active in YR: No for beam body; it constructs/registers the endpoint glow primitive. Evidence: `0x005FF250..0x005FF2CC`, body line calls at `0x00436123..0x004361B9`.
- Do not leave `0x005FF2D0` in an implementation as a visual draw step. Active in YR: No; it removes the temporary primitive from the vector. Evidence: `0x005FF2D0..0x005FF31B`.
- Do not implement the beam body as map-cell RGB lighting or `LightVisibility`/`LightIntensity`. Active in YR: No for this path; surface worker brightens existing pixels. Evidence: `0x004BBCA0`; no `LightSourceClass`/cell RGB writes in this draw path.
- Do not make event `0x23` a spotlight-specific damage/reveal action. Active in YR: No; it is generic exact trigger-event matching. Evidence: `0x0071EC9A`, `0x006E541D..0x006E554E`.
- Do not assume the stock `Hollywood Spotlight` named building activates this path. Active in YR: No in repo INI data; no `HasSpotlight=` matches. Evidence: repo INI scan.

### Stale Docs / Follow-up Docs

- `docs/research/BUILDINGLIGHTCLASS_SPOTLIGHT_PATH_GHIDRA_REPORT.md`: replace Q17 deferred wording with: "`FUN_005FF250/850/2D0` construct, rasterize, remove, and free the endpoint glow primitive; the beam body is two `DSurface +0x38` brightening line calls after `+0x78` clip/bounds calls. The line worker brightens existing 16-bit RGB channels and clamps to `0xFF`, with spotlight passing z-write flag `0`."
- `docs/research/BUILDINGLIGHTCLASS_SPOTLIGHT_PATH_GHIDRA_REPORT.md`: replace Q18 deferred wording with: "`ProcessCellAction(0x23)` is live from mode-1 spotlight search when a non-ally object is inside threshold. It calls the generic attached-trigger/event processor on owner `+0x34`; event `0x23` is exact-match generic trigger event evaluation, not a direct spotlight-specific damage/reveal/tint side effect."
- `docs/research/MAP_LIGHTING_FINAL_SYSTEM_MODEL_SYNTHESIS.md` and `docs/research/MAP_LIGHTING_POST_REINVESTIGATION_SYSTEM_MODEL_SYNTHESIS.md`: any line saying spotlight rasterization/cell action is deferred can be updated to point at this report while keeping spotlights outside ordinary ambience/lamp implementation scope.

## Remaining Uncertainty

- None for the scoped Ghidra questions Q17/Q18.
- Runtime screenshot comparison for exact perceptual beam appearance remains a later visual QA task, not a binary-path uncertainty.
- Retail map archive activation outside repo INI was not scanned; this only affects content frequency, not the verified code path.

## Sources

- Ghidra decompile/read-only spot checks: `0x00435C10`, `0x004361D0`, `0x004BBCA0`, `0x006E53A0`.
- Static disassembly/read-only binary inspection: `0x005FF250`, `0x005FF850`, `0x005FF2D0`, `0x007264C0`, `0x0071E940`, event tables `0x0071F248`, `0x0071F284`, `0x0071F2F8`, `0x0071F350`.
- Existing docs: `BUILDINGLIGHTCLASS_SPOTLIGHT_PATH_GHIDRA_REPORT.md`, `ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md`, `skirmish-ui/SKIRMISH_PRIMITIVE_BEVEL_SURFACE_VTABLE_0X30_RASTER_CONTRACT_GHIDRA_REPORT.md`, `skirmish-ui/SKIRMISH_PREVIEW_SURFACE_VTABLE_AND_CLIPPING_GHIDRA_REPORT.md`.
- Repo scans: `src/map/lighting.rs`, `src/app_init.rs`, `src/rules/object_type.rs`, `ini/rules.ini`, `ini/rulesmd.ini`, `ini/art.ini`, `ini/artmd.ini`.
