# Loading `mmpb` Exact Marker Assignment and Composition - Ghidra Report

**Date:** 2026-07-27

**Address(es):** `FUN_00640A40 @ 0x00640A40` primary; loading renderer caller `0x00552D60` / callsite `0x00553687`; `ScenarioClass__Full_Init @ 0x00687500`; projection helper `0x006D62E0`; projected-bounds writer `0x0058B820`; assignment helpers `0x005D6BE0`, `0x005EE6F0`, `ScenarioClass__AssignStartingPoints @ 0x005EE9D0`, `ScenarioClass__Gather_Start_Positions @ 0x00688380`; house/color writer `ScenarioClass__Create_Houses @ 0x00687F10`.

**Investigation Mode:** exhaustive-slice.

**Claimed Scope:** exact standard offline selected-map Skirmish loading-marker contract: wrapper and destination ownership/ABI, `mmpb.shp` identity/frame/footprint, width-keyed composition rectangles, numeric-prefix iteration, assignment writer/order, signed coordinate projection and integer rounding, house-color selection, clipping boundary, and readiness before the first loading renderer.

**Non-Scope:** random-map image generation, setup-dialog `STARTBUT.SHP` marker rendering, post-marker text geometry/content, generic `CC_Draw_Shape` raster internals, pixel comparison against a captured retail loading frame, and Rust implementation changes.

**Confidence:** High for the active-YR binary mechanism, ABI, ordering, integer formulas, assignment/color writers, and exact `(x,y,w,h)` region tuples; Medium-High for the retail `mmpb.shp` `12x12`/one-frame metadata because the current pass corroborated the exact archive hash and 200-byte entry while dimensions/frame count come from the existing retail asset probe.

**Active in YR:** Yes, conditionally, on the standard offline selected-map Skirmish scenario-start path. `ScenarioClass__Full_Init` prepares houses, selected/automatic starts, and the preview state before calling the loading renderer, which calls `0x00640A40` before the first progress milestone `3`.

## 1. Executive Finding

`0x00640A40` does more than draw colored icons over an already composed loading screen. It owns a three-surface composition:

1. It receives a four-byte wrapper whose first field is the **source map-preview surface**.
2. It burns black `4x4` start-position rectangles into that source preview.
3. It aspect-fits the source preview into a fixed-size temporary `DSurface`.
4. It draws assigned-house `mmpb.shp` frame-0 markers onto that temporary surface.
5. It copies the completed temporary preview region into the separate loading-screen destination surface supplied as the one stack argument.

The prior source/destination interpretation was reversed. The wrapper is not the loading destination. At `0x0055367B..0x00553687`, `ECX=ESI` is the preview wrapper and `[EBP+0x60]` is pushed as the final loading destination; `0x00640A40` returns with `RET 4`.

The other load-bearing correction is the fixed region tuple. Native constants are `(x,y,w,h)`, not `(origin_x,size_x,size_y,origin_y)`:

| Live screen width comparison | Region `(x,y,w,h)` in loading-destination pixels | Evidence |
|---|---:|---|
| `g_ScreenWidth == 800` | `(499,379,216,166)` | assembly constant block in `0x00640A40`; `read_memory(gamemd.exe, 0x007F5BE0, 16)` gives `640,800,1024,480` |
| `g_ScreenWidth == 1024` | `(570,424,300,260)` | same |
| every other width | `(385,270,200,200)` | same |

The comparisons are equality tests, not `>=` breakpoints. Consequently, widths such as `801`, `1023`, and `1920` take the fallback tuple in native code.

## 2. Target, Questions, and Stop Conditions

### Target questions

- What are the wrapper, inner pointer, stack argument, and final surface ownership roles?
- Which retail asset/frame is drawn, with what size, tint source, and gate?
- Does native enumerate every valid waypoint, a contiguous prefix, or a derived prefix bound?
- Who writes `ScenarioClass+0x1180`, in what player order, and before which renderer?
- What coordinate units, signed conversions, integer rounding stages, aspect-fit terms, offsets, and clipping boundary determine a marker pixel anchor?
- Is each marker colored by the local player, by waypoint number, or by the house assigned to that start?
- What state already exists before the first loading-screen draw?

### Stop conditions met

- Primary body and caller were checked in decompile and assembly.
- Projection, projected-extent writer, waypoint accessors, assignment writers, house/color writer, and first-render parent were checked.
- Both composition loops and the final copy were frame-tracked.
- Retail asset identity was anchored in the binary and retail archive index.
- Five adversarial questions and two cold spot checks were completed.
- A zero-add pass found no remaining material question inside the claimed implementation contract.

## 3. ABI and Surface Ownership

### 3.1 Exact callable contract

Assembly at `0x00640A40`:

```text
ECX                 = pointer to a 4-byte preview wrapper
[entry ESP + 4]     = final loading-screen destination surface
wrapper[0]          = source map-preview surface
return convention   = RET 4
```

Evidence:

- `0x00640A46` loads `EAX=[ECX]` and early-outs if the inner source is null.
- Caller `0x00553568` allocates four bytes and `0x006406E0` initializes the field to null.
- Selected-map caller setup passes the wrapper to `0x00641EE0`, which populates the preview source.
- `0x0055367B` restores `ECX=ESI` (the wrapper); `0x00553683` pushes `[EBP+0x60]`; `0x00553687` calls `0x00640A40`.
- `0x006406F0`, called after the marker subpass, destroys the wrapper-owned source and zeros the field.

### 3.2 Direction of every copy

| Stage | Source | Destination | Evidence | Active in YR |
|---|---|---|---|---|
| source decoration | valid-prefix waypoint coordinates | wrapper-owned preview source | fill loop `0x00640B93..0x00640CDC` | Yes |
| aspect-fit copy | wrapper-owned preview source | temporary `DSurface(region_w, region_h)` | allocation/copy `0x00640D36..0x00640E3D` | Yes |
| colored marker draw | `mmpb.shp` frame `0` | temporary region surface | `0x00640F3A..0x0064103E` | Conditional |
| final composition copy | completed temporary region surface | caller's loading destination at region `(x,y,w,h)` | `0x00641071..0x00641098` | Yes |

The final loading destination is not modified by the black waypoint fill directly. Those black pixels are first written into the source preview and then resampled with it.

## 4. Asset Role Matrix

| Asset/surface | Load/ownership | Frame/dimensions | Palette/remap role | Composition role | Active in YR |
|---|---|---|---|---|---|
| selected map `[Preview]` surface | wrapper populated before `0x00640A40`; wrapper owns/destroys it | map-defined preview dimensions | already decoded preview colors | base of the loading-screen map-preview inset; receives black start rectangles before scaling | Yes for selected maps with a valid preview |
| temporary `DSurface` | allocated inside `0x00640A40`, fixed to region `w x h` | `200x200`, `216x166`, or `300x260` | destination for preview copy and markers | clipping/composition boundary before final destination copy | Yes |
| `mmpb.shp` | lazy asset load from string `0x00836DF4` at `0x00640E44..0x00640E53` | frame `0`; retail probe: one frame, `12x12`; current archive corroboration: hash `0xA107C32F`, `ra2md.mix -> localmd.mix` entry size `200` | assigned house's runtime color-scheme convert pointer at scheme `+0x30C` | colored assigned-player marker | Conditional |
| loading country/background surface | owned by `0x00552D60` manager, supplied as stack argument | screen/loading-manager sized | LS country palette/convert outside this slice | final destination that already contains LS art | Yes |
| post-marker text | later helpers in `0x00552D60` | not drained here | selected UI/text color path | drawn after the completed preview/marker region | Yes; geometry out of scope |

`mmpb.shp` is not `STARTBUT.SHP`, not the setup-dialog numbered available-start icon, and not a preview backing. The draw uses `mmpb.shp` frame `0`, flags including `0x400`, and a literal draw scale of `1000`. That draw-scale literal is distinct from the `1,000,000` fixed normalization used in marker coordinate math.

The `12x12` asset footprint is not centered by the final `(-3,-2)` adjustment. Those offsets are the native anchor convention; replacing them with half-width/half-height centering would move the marker.

### 4.1 UI composition ledger

| Order | Producer | Output | Gate | Geometry / clip | Color source | Player-visible role |
|---:|---|---|---|---|---|---|
| 0 | `0x00552D60` | LS country/background frame | loading assets/convert available | loading-manager destination | LS country palette | loading backdrop |
| 1 | `0x00640A40` source fill loop | black `4x4` rectangles | derived numeric-prefix waypoint is valid | preview-source coordinates; source-surface clip | zero RGB through destination pixel format | black source start indicators incorporated before colored assignment composition |
| 2 | temporary-surface copy | aspect-fitted selected preview | wrapper inner source non-null | centered within exact fixed `w x h` temporary surface | source preview pixels | map-preview inset |
| 3 | `CC_Draw_Shape` in `0x00640A40` | `mmpb.shp` frame `0` | valid prefix waypoint, assigned house, asset, convert | temporary-region-local anchor; normal temp-surface clip | assigned house scheme `+0x30C` | colored assigned-player marker |
| 4 | final surface copy | completed temporary region | temporary allocation/copy path succeeds | fixed loading-destination `(x,y,w,h)` | already composed | preview plus markers on LS backdrop |
| 5 | later `0x00552D60` helpers | post-marker text | mode/content-specific | not investigated here | text/UI scheme | loading description/chrome |
| 6 | `FUN_0069AE90(3)` after renderer return | first progress milestone | selected-map progress path | progress-manager surface | local loading scheme | first progress display |

## 5. Waypoint Enumeration and Black Source Markers

### 5.1 Validity accessor

`FUN_0068BD80` returns valid only when:

```text
0 <= index < 0x2BE
and Scenario waypoint[index] != packed invalid-cell sentinel
```

`FUN_0068BCC0` returns the packed signed-`i16` cell pair from `ScenarioClass+0x632+index*4`.

### 5.2 The non-obvious numeric-prefix contract

The primary function first calls `FUN_0068BD80(i)` for every `i=0..7` and counts how many are valid. Call that count `N`.

Both later loops use numeric indices `s=0..N-1`, recheck `FUN_0068BD80(s)`, and skip an invalid slot. They do **not** enumerate the original set of valid indices from `0..7`.

Consequences:

- With valid `{0,1,2,3}`, `N=4`, both loops visit `0..3`.
- With valid `{0,1,4,5}`, `N=4`, both loops visit `0..3`; `2` and `3` fail, while valid `4` and `5` are never visited.
- This odd hole behavior is native and must not be silently “fixed” if exact mechanism is selected.

Normal retail maps use a contiguous numeric start prefix, so the difference is mainly malformed/custom-map behavior. `ScenarioClass__Gather_Start_Positions @ 0x00688380` independently scans the initial `0..7` waypoints and breaks at the first invalid sentinel, further showing that the ordinary contract expects a contiguous prefix.

### 5.3 Black rectangles baked into the source

For each visited valid prefix slot:

```text
px = projected_x / 60
py = projected_y / 30

black_rect_x = (px - min_x) * 2 - 1
black_rect_y = (py - min_y)     - 1
black_rect_w = 4
black_rect_h = 4
```

The division is signed truncation toward zero. RGB is zero through the current DirectDraw pixel-format masks, producing black. The rectangle is filled into the wrapper-owned preview source before aspect-fit scaling.

The `*2` is only in the source-preview X convention. It does not appear in the final colored-marker normalization, whose divisor is the projected X extent.

## 6. Projection Units, Signedness, Bounds, and Rounding

### 6.1 Cell to centered leptons

Each waypoint packs two signed 16-bit cell coordinates. The renderer sign-extends both and converts each axis to a cell-center lepton:

```text
lepton_x = signed_cell_x * 256 + 128
lepton_y = signed_cell_y * 256 + 128
```

Evidence: `MOVSX`, `SHL 8`, and `+0x80` at `0x00640EB4..0x00640EC8`. Negative cells remain negative until the `+128` center bias.

### 6.2 Isometric projection helper

`FUN_006D62E0` computes:

```text
raw_x = (lepton_x * 60) / 2 + (lepton_y * -60) / 2
raw_y = (lepton_x * 30) / 2 + (lepton_y *  30) / 2

screen_x = trunc_toward_zero(raw_x / 256) + 15360
screen_y = trunc_toward_zero(raw_y / 256)
```

The emitted assembly form applies `(value + ((value >> 31) & 0xFF)) >> 8`, which implements signed truncation toward zero for division by `256`.

Consumers then perform signed truncation toward zero for:

```text
projected_x = screen_x / 60
projected_y = screen_y / 30
```

The optimized `0x88888889` multiply sequences plus sign correction in `0x00640A40` are equivalent to those signed divisions.

### 6.3 Stored projected playfield bounds

`FUN_0058B820` walks every in-playfield cell, projects it, derives `projected_x/projected_y`, and writes:

```text
Scenario+0x112C = min_projected_x
Scenario+0x1130 = min_projected_y
Scenario+0x1134 = max_projected_x - min_projected_x   // extent_x
Scenario+0x1138 = max_projected_y - min_projected_y   // extent_y
```

These fields are projected extents, not `[Map] LocalSize`, raw cell width/height, or destination pixels. `0x00640A40` also recomputes local `min_x/min_y` while scanning the live playfield, then uses the stored `+0x1134/+0x1138` as the normalization divisors.

Ordinary valid maps must therefore provide positive extents. The marker path contains no explicit zero-divisor guard immediately before its signed `IDIV`; do not invent a silent `max(1)` parity rule.

## 7. Aspect Fit and Exact Marker Formula

Let:

```text
region = (region_x, region_y, region_w, region_h)
source = (source_w, source_h)
```

Native calculates:

```text
scale_1000 = min(
    trunc_toward_zero(region_h * 1000 / source_h),
    trunc_toward_zero(region_w * 1000 / source_w)
)

fit_w = trunc_toward_zero(source_w * scale_1000 / 1000)
fit_h = trunc_toward_zero(source_h * scale_1000 / 1000)

pad_x = trunc_toward_zero((region_w - fit_w) / 2)
pad_y = trunc_toward_zero((region_h - fit_h) / 2)
```

The preview source is copied into the temporary region surface at `(pad_x,pad_y)` with fitted size `(fit_w,fit_h)`.

For each visited valid prefix slot assigned to a house:

```text
dx = projected_waypoint_x - local_min_x
dy = projected_waypoint_y - local_min_y

fraction_x_1e6 = trunc_toward_zero(dx * 1_000_000 / extent_x)
fraction_y_1e6 = trunc_toward_zero(dy * 1_000_000 / extent_y)

local_marker_x =
    pad_x + trunc_toward_zero(fraction_x_1e6 * fit_w / 1_000_000) - 3
local_marker_y =
    pad_y + trunc_toward_zero(fraction_y_1e6 * fit_h / 1_000_000) - 2

final_screen_x = region_x + local_marker_x
final_screen_y = region_y + local_marker_y
```

The repeated `LEA *5` operations followed by `SHL 6` produce the first `*1,000,000`; the `0x431BDE83` sequence with sign correction performs the later signed `/1,000,000`. Preserve the two-stage truncation. Algebraically collapsing the expression to a float ratio can move a marker by a pixel.

There is no explicit coordinate clamp. `CC_Draw_Shape` draws into the temporary `region_w x region_h` surface, whose normal destination clipping clips any marker pixels crossing its edge. The completed temporary surface is then copied as the fixed region.

### Concrete arithmetic fixture

This fixture demonstrates the verified integer stages; it is not a retail-map golden:

```text
screen width = 800
region        = (499,379,216,166)
source        = 200x80
extent        = (100,80)
waypoint dxy  = (25,20)

scale_1000 = min(166000/80, 216000/200) = min(2075,1080) = 1080
fit         = (216,86)
pad         = (0,40)
fraction    = (250000,250000)
local       = (0 + 250000*216/1000000 - 3,
               40 + 250000*86/1000000 - 2)
            = (51,59)
screen      = (550,438)
```

The Y fitted size is `86`, not `86.4`; the nested normalization then produces `21`, not a rounded `22`.

## 8. Assignment Table Writer and Selection Order

### 8.1 Table identity

`ScenarioClass+0x1180..+0x11BF` is a 16-entry `i32` table:

```text
table[start_index] = house_array_index
-1                 = unassigned
```

`ScenarioClass__Full_Init` clears all 16 entries to `-1` before non-campaign setup. The loading marker renderer only examines the native derived prefix within waypoint indices `0..7`.

### 8.2 Start-position list

`ScenarioClass__Gather_Start_Positions @ 0x00688380`:

1. Counts the contiguous valid waypoint prefix beginning at index `0`, stopping on the first invalid sentinel or after eight.
2. Computes the number of starts required by active human and AI participants.
3. Copies original waypoint cells in numeric order.
4. If starts are deficient, appends random nearby passable cells to the temporary start-position vector.

Those fallback cells are not written back to the original `ScenarioClass+0x632` waypoint array. Therefore the assignment machinery can assign a participant to an appended fallback start that `0x00640A40` cannot render: the loading marker reads original waypoint coordinates only.

### 8.3 Explicit selected starts before automatic assignment

In the selected-mode vtable `+0x80` implementation at `0x005D6BE0`:

- houses are visited in ascending `g_HouseClass_Array` index;
- `House+0x16058` supplies the explicit start choice;
- `-2` means no explicit selection;
- otherwise it chooses the corresponding gathered start vector cell, writes the house start coordinate, and writes `table[start_index]=house_index`.

`ScenarioClass__Full_Init` calls selected mode `+0x80` first. When `DAT_00A8B244 == 2`, it then calls `ScenarioClass__AssignStartingPoints`; otherwise it calls selected mode `+0x84`. Thus explicit choices already occupy table entries before automatic assignment in the ordinary assignment branch.

### 8.4 Automatic player order

`ScenarioClass__AssignStartingPoints @ 0x005EE9D0`:

1. Builds an occupied-byte vector from the 16 table entries.
2. Visits eligible **human houses first**, ascending house-array index.
3. A human already present in the table receives that table index's gathered cell.
4. An unassigned human invokes `0x005EE6F0` with the human flag set.
5. Visits eligible **AI houses second**, ascending house-array index.
6. Every AI invokes `0x005EE6F0` with the human flag clear.
7. The picker marks the chosen start occupied, writes `table[chosen_index]=house_index`, and returns the chosen cell for the house's start coordinate.

The exact picker branches are:

- no occupied starts + human flag: random candidate over the full vector;
- exactly two occupied starts + AI flag: random candidate among free entries;
- occupied count other than one: free candidate maximizing the sum of integerized Euclidean distances to occupied starts;
- exactly one occupied start: free candidate minimizing that distance sum.

Distances subtract packed signed-`i16` cell axes, square in floating-point inside this shell/setup helper, take `Sqrt_Approx`, convert through native `ftol`, and sum. This is pre-simulation setup logic; it is not marker projection math.

## 9. House and Color Selection

`ScenarioClass__Create_Houses @ 0x00687F10` constructs the human and AI houses before assignment and the first renderer.

For both human nodes and AI slots:

```text
priority = configured player color priority
scheme_index = SessionClass__PriorityToColorScheme(priority)
House+0x16054 = scheme_index
```

`SessionClass__PriorityToColorScheme @ 0x0069A310` maps priorities `0..8` through the signed-byte table at `0x0083ED14`:

```text
[3,11,21,29,13,25,17,15,5]
```

Priority `-2` uses the byte at `0x0083ED1C` (`5` in the active image). Other values pass through.

For a marker assignment:

```text
house_index   = Scenario+0x1180[start_index]
house         = g_HouseClass_Array[house_index]
scheme_index  = house+0x16054
scheme        = g_ColorSchemeArray[scheme_index]
convert       = scheme+0x30C
```

The marker draws only if the assignment is not `-1`, `mmpb.shp` loaded, and `convert` is non-null. There is no explicit scheme-index bounds check in `0x00640A40`; validity is established by earlier house construction.

Every marker uses its **assigned house's** convert. It is incorrect to tint every marker with the local player's loading-bar color, to tint by waypoint number, or to apply the loading-bar `priority -> [Colors] entry -> /2` shortcut indiscriminately to this native pointer lookup.

House array order and table start-index order are separate axes. A later renderer must preserve `table[start] -> house -> color`, not zip player slots and waypoint coordinates by position.

## 10. First-Renderer Readiness and Composition Order

The active selected-map order in `ScenarioClass__Full_Init` is:

1. Clear `ScenarioClass+0x1180..+0x11BF`.
2. Read selected non-campaign scenario setup.
3. Create houses and write each `House+0x16054` color-scheme index.
4. Prepare preview/projected bounds (`Scenario+0x112C..+0x1138`) and start-position data.
5. Apply selected-mode explicit start choices.
6. Run automatic assignment or the selected-mode alternate assignment callback.
7. Construct `LoadProgressMgr`.
8. Call `ScenarioClass__DrawLoadingScreen @ 0x00552D60`.
9. Inside it, draw LS country/background art, call `0x00640A40`, destroy the preview wrapper, then execute post-marker text helpers.
10. Return to `ScenarioClass__Full_Init`, then call `FUN_0069AE90(3)`.

Therefore assigned coordinates, assigned house indices, house colors, projected bounds, and the selected preview source all exist before the first loading renderer. This is why native can show the map preview and assigned markers on its first composition.

Post-marker text is later in the same `0x00552D60` call. No text geometry/content claim is added here.

## 11. Current Rust Handoff

This section is a read-only handoff, not a completion claim.

| Verified contract | Current Rust evidence | Required effect | Acceptance check | Risk / do not do |
|---|---|---|---|---|
| Region tuples are exact `(x,y,w,h)` and selected by equality | `src/render/loading_screen_chrome.rs::MmpbRegionRect` currently names/stores them as `(origin_x,size_x,size_y,origin_y)` and `mmpb_region_rect` uses `>=` | Represent the tuple unambiguously as `x,y,width,height`; decide deliberately whether exact native equality or a documented modern-width policy is desired | Assert 640/800/1024 exact tuples; if exact mechanism, also assert 801/1023/1920 fallback | Do not consume `379` as width or `166` as Y at 800 |
| First composition includes selected preview, black starts, and assigned colored markers | `LoadingScreenAtlas` and `build_native_loading_instances` currently compose background, bar backing, bar, and side icon only | Add a bounded loading-preview composition layer before post-marker text/progress handoff | First selected-map frame contains the actual decoded selected preview and every renderable assigned marker | Do not substitute a decorative minimap or `STARTBUT.SHP` |
| Marker state is ready before the first renderer | `LoadingRequest` retains a launch session and filename, but `NativeLoadingScreenState` has no selected scenario record, waypoint coordinates, preview source/bounds, or resolved start-to-house table; `InitialMapSelection` occurs after the first presentation path | Snapshot/prepare the selected scenario preview, projected bounds, assignments, and per-house colors before first-frame rendering, using already scanned `SkirmishScenarioRecord` where possible | A first-frame-only test proves marker data exists before `first_frame_presented` | Do not wait for the later legacy map-load result and then claim native first-frame order |
| Scenario records already carry useful source data | `SkirmishScenarioRecord` contains `preview`, `multiplayer_start_waypoints`, and `preview_source_bounds` | Preserve these data into the loading request/snapshot instead of reparsing after the first frame | Selected record and launch session jointly produce a deterministic marker snapshot | Do not derive projected extents from `[Map] LocalSize` |
| Assignment is start-index -> house-index, human first then AI, after explicit choices | `SkirmishLaunchSession` stores slot start selections and colors but no native-derived assignment table or waypoint cells | Materialize a start-index ownership table before rendering; keep participant and waypoint orders separate | Explicit starts, automatic starts, and a hole/fallback fixture exercise table semantics | Do not zip waypoint list with local+AI slot order |
| Per-marker color comes from assigned house | current native loading state holds only the local player's color/ramp for the progress bar | Carry or derive one marker remap/tint per assigned house | Two differently colored participants render two corresponding `mmpb` colors | Do not reuse `NativeLoadingScreenState.color_index` for all markers |
| Two-stage integer normalization and `(-3,-2)` anchor | constants exist, but no marker renderer consumes the formula | Implement signed integer helpers with explicit truncation stages; direct-to-screen is equivalent only if it adds region origin after local fit/pad math | Concrete arithmetic fixture above returns `(550,438)` | Do not float-collapse, round-to-nearest, half-size center, or clamp |
| Retail asset is runtime data | atlas does not currently include `mmpb.shp` | Decode retail frame 0 and apply the assigned-house remap path | Atlas/instance test proves retail frame 0 and `12x12` footprint | Do not hardcode marker pixels |

### Delivery classification

- Missing preview/assigned-marker composition is player-visible on every ordinary selected-map Skirmish load and is a reasonable retail-convincing follow-up.
- Exact malformed-hole behavior and equality behavior for non-retail widths are real mechanism differences but can be recorded as exactification residuals if the implementation intentionally chooses a modern-width policy.
- The swapped tuple field meanings are not merely an exactification residual if code begins consuming them: they would visibly place/size the preview region incorrectly at 800 and 1024.

## 12. Coverage Ledger

| Area / function / branch | Status | Evidence | Remaining boundary |
|---|---|---|---|
| primary ABI and early-out | verified | caller assembly plus `0x00640A40` entry/epilogue | none |
| source black-marker loop | verified | `0x00640B93..0x00640CDC` | generic pixel-format helper internals not renamed |
| fixed region selection | verified | primary assembly plus static width constants | no runtime test at non-retail widths |
| aspect-fit and surface direction | verified | `0x00640D36..0x00640E3D`, final copy `0x00641071..0x00641098` | generic virtual surface method names remain role-based |
| colored marker loop/gates | verified | `0x00640E69..0x0064106B` | generic SHP raster internals out of scope |
| retail `mmpb` identity/frame/size | verified for implementation input | string/xref, archive hash/entry, prior retail metadata probe | exact pixel-index matrix deferred |
| projection helper and signed rounding | verified | `0x006D62E0`, primary assembly | none |
| projected extent writer | verified | `0x0058B820` | none |
| waypoint validity/prefix semantics | verified | `0x0068BD80`, `0x0068BCC0`, both primary loops | malformed-map runtime screenshot not needed |
| gathered/fallback start list | verified | `0x00688380` | fallback marker absence not runtime-captured |
| explicit assignment writer | verified | `0x005D6BE0` assembly | selected-mode identity inherited from verified caller/vtable context |
| automatic assignment and picker | verified | `0x005EE9D0`, `0x005EE6F0` | none for table/order contract |
| house/color writer | verified | `0x00687F10`, `0x0069A310`, table bytes | exact ConvertClass pixel remap delegated to retail asset pipeline |
| readiness before first renderer | verified | `0x00687500` order and `0x00553687` | Rust runtime presentation test remains implementation work |
| post-marker text | verified for relative order only | caller body after `0x00553687` | content/layout deliberately out of scope |
| current Rust handoff | inspected | named current source symbols | no Rust changes in this report |

## 13. Tiny-Detail Ledger

| # | Detail | Status |
|---:|---|---|
| 1 | Wrapper is exactly four bytes in the caller. | VERIFIED |
| 2 | `wrapper[0]` is the preview source; stack arg is final loading destination. | VERIFIED |
| 3 | Empty wrapper early-outs before any region composition. | VERIFIED |
| 4 | Function uses `RET 4`. | VERIFIED |
| 5 | Native screen-width tests are equality against static `800` and `1024`. | VERIFIED |
| 6 | All other widths use the 640-style fallback tuple. | VERIFIED |
| 7 | Region tuple order is `(x,y,w,h)`. | VERIFIED |
| 8 | Valid-count scan examines all indices `0..7`. | VERIFIED |
| 9 | Both draw loops then visit numeric `0..N-1`, not the valid set. | VERIFIED |
| 10 | Each loop rechecks validity and skips holes inside that prefix. | VERIFIED |
| 11 | Black source rectangles are `4x4`, at X `*2-1`, Y `-1`. | VERIFIED |
| 12 | Black rectangles are drawn before preview resampling. | VERIFIED |
| 13 | Waypoint cell halves are sign-extended `i16`. | VERIFIED |
| 14 | Cell centers add `128` leptons after multiplying by `256`. | VERIFIED |
| 15 | Projection adds `15360` only to X. | VERIFIED |
| 16 | `/256`, `/60`, `/30`, fit math, and marker math truncate toward zero. | VERIFIED |
| 17 | Projected extent fields are not `[Map] LocalSize`. | VERIFIED |
| 18 | Marker normalization uses `1,000,000`; draw call scale uses `1000`. | VERIFIED |
| 19 | Final marker offsets are `-3` and `-2`, not half-frame centering. | VERIFIED |
| 20 | No explicit marker-coordinate clamp occurs. | VERIFIED |
| 21 | Clipping is against the temporary region surface. | VERIFIED |
| 22 | Assignment table has 16 entries but marker coordinates are limited to original indices `0..7`. | VERIFIED |
| 23 | Gathered fallback passable cells are not written back as original waypoint coordinates. | VERIFIED |
| 24 | Explicit selected starts are applied before automatic assignment. | VERIFIED |
| 25 | Automatic assignment visits humans before AIs, each in house-array order. | VERIFIED |
| 26 | Assignment values are house-array indices. | VERIFIED |
| 27 | Each marker obtains color from the assigned house's `+0x16054` scheme. | VERIFIED |
| 28 | Null `mmpb` or null convert skips only that colored marker; preview composition continues. | VERIFIED |
| 29 | Completed temp region is copied after marker loop. | VERIFIED |
| 30 | Post-marker text follows the region copy; progress `3` follows renderer return. | VERIFIED |

## 14. Adversarial Questions

1. **Could the wrapper be the final loading target and the stack argument the preview source?** No. Caller construction/population/destruction follows the wrapper; first copy reads `wrapper[0]`; final copy targets the stack argument.
2. **Could the count loop be equivalent to enumerating every valid slot?** No. A hole fixture separates the all-eight count from the later numeric-prefix bound.
3. **Could the 800/1024 tuple be `(x,width,height,y)` because existing Rust names it that way?** No. Temporary allocation uses the third/fourth constants as width/height; the final copy positions it with the first/second constants.
4. **Could `1000` in `CC_Draw_Shape` be the projection scale?** No. Coordinate normalization already used `1,000,000`; the pushed `1000` is a separate generic shape-draw scale argument.
5. **Could out-of-range anchors be explicitly clamped before draw?** No clamp instruction is present. The temporary surface provides the normal draw clip.
6. **Could first-frame Rust wait for map loading without changing native order?** No. Native assignment/preview state is ready before `0x00552D60`; current Rust presents before `InitialMapSelection`.

## 15. Cold Spot Checks

### Cold spot A: width constants

`read_memory(program="gamemd.exe", address=0x007F5BE0, length=16)` returned:

```text
80 02 00 00  20 03 00 00  00 04 00 00  E0 01 00 00
640           800           1024          480
```

This independently pins the equality operands and prevents treating `DAT_007F5BE4`/`DAT_007F5BE8` as unknown runtime thresholds.

### Cold spot B: null marker resources

Branches at `0x00640F64..0x00640F78` skip a colored marker if the `mmpb` pointer or assigned-house convert is null, but the loop advances and the function still reaches the final temporary-to-loading-destination copy. A missing marker resource does not suppress the selected preview inset.

## 16. Open Questions - Final State

- [RESOLVED] OQ-1 - Wrapper/source/destination roles and ABI. See Section 3.
- [RESOLVED] OQ-2 - Exact region tuple order and screen-width selector. See Sections 1 and 7.
- [RESOLVED] OQ-3 - Numeric-prefix iteration and hole behavior. See Section 5.
- [RESOLVED] OQ-4 - Signed projection, extent writer, aspect fit, rounding, marker offsets, and clipping. See Sections 6 and 7.
- [RESOLVED] OQ-5 - Assignment table writer and participant order. See Section 8.
- [RESOLVED] OQ-6 - House/color selection. See Section 9.
- [RESOLVED] OQ-7 - State readiness before first renderer. See Section 10.
- [RESOLVED] OQ-8 - Retail asset identity/frame/footprint required for implementation. See Section 4.
- [DEFERRED] OQ-9 - Exact `mmpb.shp` frame-0 pixel-index matrix. **Category:** out-of-scope / unnecessary duplication. **Reason:** production should decode the retail asset; this pass pinned filename, archive entry, frame, and dimensions, while a pixel matrix would not change the mechanism contract.
- [DEFERRED] OQ-10 - Retail-vs-Rust pixel capture for the complete loading frame. **Category:** needs-runtime-capture. **Reason:** this report establishes inputs and composition, not a pixel-parity certification.

Zero-add pass: no unresolved material implementation question remains inside the claimed marker/assignment/composition slice.

## 17. Negative Facts / Do Not Do

- Do not reverse the wrapper source and final loading destination.
- Do not treat `(499,379,216,166)` as `(x,width,height,y)`.
- Do not describe native selection as `>=800` / `>=1024`.
- Do not use `[Map] LocalSize` as `Scenario+0x1134/+0x1138`.
- Do not enumerate all sparse valid indices if claiming exact native behavior.
- Do not show generated fallback start cells unless a separate verified source-coordinate contract is added.
- Do not color every marker with the local player's progress-bar color.
- Do not replace `mmpb.shp` with `STARTBUT.SHP`, a circle, or hardcoded pixels.
- Do not center the `12x12` frame by subtracting `(6,6)`; native uses `(-3,-2)`.
- Do not clamp anchors explicitly or float-collapse the nested integer divisions while claiming exact mechanism.
- Do not delay marker data creation until after the first displayed Rust loading frame and call it native order.
- Do not claim native/Rust pixel parity from formula/unit tests alone.

## 18. Stale Wording and Required Corrections

The structurally stale `skirmish-ui/SKIRMISH_MMPB_ASSIGNED_PLAYER_MARKER_CONTEXT_GHIDRA_REPORT.md` should be read with these replacements:

- Replace “caller-provided surface” as a single ambiguous object with the exact wrapper-source / stack-destination ABI in Section 3.
- Replace “temporary surface is blitted back to the caller surface” with “temporary surface is copied to the separate stack-argument loading destination; the wrapper owns the source preview.”
- Replace `Scenario+0x1134/+0x1138` “preview source width/height” wording with “projected playfield X/Y extents.”
- Replace “assignment population touched-not-exhausted” with the verified writer/order in Section 8.

The handoff `LOADING_SCREEN_MARKERS_BAR_HANDOFF_2026_05_30.md` has a now-closed “centering UNRESOLVED” block:

> `scaleX/scaleY` are the aspect-fitted preview dimensions `(fit_w,fit_h)`, while `offsetX/offsetY` are the temporary-surface letterbox pads `(pad_x,pad_y)`. The colored marker is drawn in temporary-region-local coordinates, then the completed region is copied to loading-destination origin `(region_x,region_y)`.

Its tuple description `{385,270,200,200}; {499,379,216,166}; {570,424,300,260} as origin_x,size_x,size_y,origin_y` is stale. The correct tuple is `(x,y,w,h)`.

## 19. Sources

### Live Ghidra, read-only, active `gamemd.exe`

- `audit_globals_in_function(0x00640A40)`
- decompile plus full/range assembly for `0x00640A40`
- caller assembly `0x00553568..0x0055369A`
- wrapper constructor/destructor `0x006406E0`, `0x006406F0`
- waypoint validity/accessors `0x0068BD80`, `0x0068BCC0`
- projection `0x006D62E0`
- projected-bounds writer `0x0058B820`
- `ScenarioClass__Gather_Start_Positions @ 0x00688380`
- explicit selected-start writer assembly at `0x005D6BE0`
- picker `0x005EE6F0`
- `ScenarioClass__AssignStartingPoints @ 0x005EE9D0`
- `ScenarioClass__Create_Houses @ 0x00687F10`
- `SessionClass__PriorityToColorScheme @ 0x0069A310`
- `ScenarioClass__Full_Init @ 0x00687500`
- `read_memory(0x007F5BE0,16)` and `read_memory(0x0083ED14,16)`

### Retail and repository evidence

- Active retail root: `<ra2-install>/`
- Fresh filename-hash corroboration: `MMPB.SHP -> 0xA107C32F`
- Existing compiled retail archive probe: `ra2md.mix -> localmd.mix`, matching `0xA107C32F` entry size `200`
- Prior retail metadata probe recorded in `skirmish-ui/SKIRMISH_MAP_PREVIEW_MARKER_ASSET_LAYOUT_GHIDRA_REPORT.md`: `mmpb.shp`, one frame, `12x12`
- Current Rust read-only touchpoints: `src/render/loading_screen_chrome.rs`, `src/app_loading.rs`, `src/skirmish_scenarios.rs`, `src/skirmish_launch.rs`

**Status:** COMPLETE for the scoped standard offline selected-map Skirmish `mmpb` marker assignment and composition mechanism. Exact complete-frame pixel parity remains UNCHECKED.
