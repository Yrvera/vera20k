# Authored OverlayPack Inline Transaction — Ghidra Re-investigation

Date: 2026-08-31
Status: **COMPLETE for the bounded active-YR owner/callee corridor**
System: GSI-04.12 / GSI-04.13 authored overlay map-load bridge transaction
Mode: `/re-investigate`, exhaustive-slice; read-only Ghidra

## Verdict

`[ACTIVE-YR: YES]` Active-retail `ReadMapOverlayPacks @ 0x005FD2E0` owns one
decoded `OverlayPack` transaction. When `NewINIFormat > 1` and the section decodes
to a positive length, it visits exactly `y=0..511` outer and `x=0..511` inner.
Every admitted coordinate constructs one ephemeral `OverlayClass`; its
`Unlimbo -> virtual Mark(1)` finishes synchronously before `x` advances. High,
low-procedural, low-body, and ordinary overlays therefore interleave in decoded
coordinate order. There is no high-first pass, low post-pass, or component batch.

`[ACTIVE-YR: CONDITIONAL]` The eight procedural low trigger IDs
`0x7A..0x7D` and `0xE9..0xEC` are reachable from valid authored YR content and
execute inline on the active Scenario RNG. The scanned stock-map corpus contains
no such trigger rows, so stock occurrence is zero while executable reachability is
not dormant. The settled low body/search tables were spot-checked only at their
entry, RNG, write, and common-tail boundaries; they were not re-investigated.

`[ACTIVE-YR: YES]` The independent `OverlayDataPack` traversal follows the
entire identity/Mark traversal and blindly overwrites `Cell+0x11E` on every native
in-radar cell. No Recalc occurs in the reader after this overwrite. Back in
`ScenarioClass::Full_Init @ 0x00686B20`, a whole-map
`CellClass::RecalcAttributes(-1) @ 0x0047D2B0` sweep follows the reader and precedes
`[Terrain]` and all authored Techno sections. For map width `W=Map+0xF4` and
height `H=Map+0xF8`, it makes exactly `H*(2W-1)` Recalc calls in native
anti-diagonal order, not y/x order. That sweep is outside the `NewINIFormat` and
pack-presence gates and runs the full LAT, CliffBack, zone/cache, and conditional
terrain-animation-latch body. It is the **post-data sweep**, not the last Recalc
of Full_Init: `MapClass::InitCellAttributes(0) @ 0x00568BB0` later destroys the
transient terrain-attached animations, clears their per-cell latches, and performs
another equal-count anti-diagonal Recalc sweep after authored map objects.

## Authority and prior-work boundary

- `[ACTIVE-YR: YES]` Ghidra program `gamemd.exe`, x86 PE image base `0x00400000`,
  executable `C:\Users\enok\Documents\Command and Conquer Red Alert II\gamemd.exe`,
  SHA-256 `1CDD1180E49024FBDA8AD568CAAC2E86E856063FF67AB38F62B7D2C7BB84298C`.
  All Ghidra work in this pass was decompile/disassembly/xref/memory inspection;
  no metadata was changed.
- `[RETAIL-DATA]` `ini/rulesmd.ini`, `ini/artmd.ini`, and installed retail
  `Lostlake.mmx`, `Killer.mmx`, and `Shrapnel.mmx` were read directly.
- `[LEAD ONLY]` `C:\Users\enok\Documents\OpenTS\code\overlay.cpp:156..467`
  was used to navigate the inherited skeleton. Every material conclusion below
  was independently proved in active YR.
- `[PRIOR VERIFIED + COLD BOUNDARY CHECK]` The exact low internal tables and
  `3*L` raw-`Next & 3` body transaction remain owned by:
  `LOW_OVERLAY_MARK_FIXED_MAP_STAMP_RNG_TRANSACTION_GHIDRA_REPORT.md`,
  `LOW_OVERLAY_MARK_SCENARIO_LOAD_ACTIVATION_BOUNDARY_GHIDRA_REPORT.md`, and
  `LOW_OVERLAY_MARK_ALL_LOAD_CONTEXT_SCENARIO_RNG_LIFECYCLE_GHIDRA_REPORT.md`.
  This report extends them through the complete owner/filter/interleaving/data/
  final-Recalc corridor.
- `[OUT OF THIS SLOT]` The shared-dummy field census and the full fresh-load/
  restore context matrix are separate investigations. No conclusion here assumes
  their unresolved details.

## Integrated active-YR timeline

| Order | Active in YR | Owner / evidence | Exact effect |
|---:|---|---|---|
| 1 | **Yes** | `ScenarioClass::Read_INI_Basic`, key string `0x0083E128`, read call `0x0068A13F`, store `0x0068A156` | Reads `[Basic] NewINIFormat` with default `0` into `0x00A8ED7C`. |
| 2 | **Yes** | `Full_Init 0x00686B20`, disassembly `0x00686B45..4F` | Increments the load-suppression counter `0x00A8E7AC` and keeps it nonzero through this corridor. |
| 3 | **Yes** | `Full_Init+0xEEB @ 0x00687A0B` | Reads explicit `[Tubes]` through `MapClass::ReadTubesINI`; this is a separate, earlier owner. |
| 4 | **Yes** | sole direct reader call `Full_Init+0xF14 @ 0x00687A34` | Enters `ReadMapOverlayPacks`; no other direct caller exists. |
| 5 | **Conditional** | reader `CMP [0x00A8ED7C],1 / JLE @ 0x005FD2EC..F3` | Both pack bodies execute only for signed `NewINIFormat > 1`. The `<=1` arm still reaches `DrainDeferredFinalizationQueue @ 0x00725C70`; it is not a literal function return. |
| 6 | **Conditional** | `ReadINIBase64BinarySectionSourceOrder @ 0x00526FB0` with `OverlayPack` string `0x00833484` | Identity traversal starts only when returned decoded-source length is signed-positive. |
| 7 | **Conditional** | reader loops `0x005FD3F4..0x005FD51C`; disassembly confirms inner `ESI` then outer `EDI` bounds `0x200` | Visits `(x,y)` as `y=0..511` outer, `x=0..511` inner. One-byte read and the complete constructor/Mark/restore transaction finish before inner increment. |
| 8 | **Conditional** | `OverlayClass::Constructor @ 0x005FC380` -> `ObjectClass::Unlimbo @ 0x005F4EC0` -> vtable `+0x124` | An admitted row whose object allocation succeeds dispatches `OverlayClass::Mark(1) @ 0x005FC570` synchronously. Vtable `0x007EF3D4` and COL/type descriptor `.?AVOverlayClass@@` bind the slot. |
| 9 | **Conditional** | `OverlayClass::Mark`, high comparisons/calls `0x005FC5F8..62C`, low comparisons `0x005FC790..A2` and `0x005FCBB9..CB` | Dispatches high, low-procedural, or ordinary work at the current packed coordinate; no later bridge dispatcher exists. |
| 10 | **Conditional** | reader high checks/store `0x005FD4DB..0x005FD502` | Only IDs `0x18`, `0x19`, `0xED`, `0xEE` restore the anchor's pre-construction `Cell+0x11E` after the entire Mark call returns. |
| 11 | **Conditional** | independent `OverlayDataPack` key `0x00833474`, loops `0x005FD5F7..0x005FD656` | If its length is positive, visits the same y-outer/x-inner grid and writes decoded byte to `Cell+0x11E` after only `Cell_in_bounds_check`'s radar-diamond predicate (the same predicate used by native cell allocation). It runs even when `OverlayPack` was absent/empty. |
| 12 | **Yes** | reader epilogue `0x005FD692` -> `DrainDeferredFinalizationQueue` | Drains deferred object finalization after the gated pack work or after a gate skip. No bridge Recalc is done here after data. |
| 13 | **Yes** | `Full_Init 0x00687A3E..6B`, `CellIterator_Init 0x00578350`, `CellIterator_Next 0x00578290`, Recalc call `0x00687A5A` | After both packs, visits exactly `H*(2W-1)` live cells once each in anti-diagonal order and calls the full `RecalcAttributes(-1)` body, independent of whether either pack ran. |
| 14 | **Yes; non-bridge exclusion** | `FUN_005FDDF0 @ 0x00687A6D` | Performs only overlay `0x7E` vein fixups; it is not a bridge post-pass. |
| 15 | **Yes** | `Full_Init`: Terrain `0x00687A74`; Units `0x00687AA7`; Aircraft `0x00687ABF`; Infantry `0x00687ACB`; Structures `0x00687AEA`; Smudge `0x00687B0E`; `InitCellAttributes(0) 0x00687B92` | The post-data Recalc precedes Terrain and Technos, so low-Mark words precede the first authored Techno word. After all listed object sections, InitCellAttributes deletes the earlier terrain-attached anims, clears latches, and performs the surviving post-object Recalc sweep. |

### Exact whole-map iterator count and order

`[ACTIVE-YR: YES]` On a successfully completed map Resize/Full_Init,
`MapClass::CellIterator_Init @ 0x00578350` seeds
`(x,y)=(1,W)`, run remaining `W-1`, and pointer
`cell_array[W*0x200 + 1]`. `CellIterator_Next @ 0x00578290` returns the current
pointer before advancing. Within a diagonal it moves `(x+1,y-1)`. At a diagonal
end it alternates the next start so that:

- visited sums are `x+y = W+1, W+2, ..., W+2H`;
- odd-numbered diagonals in that sequence contain `W` cells and even-numbered
  diagonals contain `W-1` cells;
- every cell satisfying `W < x+y`, `x-y < W`, `y-x < W`, and
  `x+y <= W+2H` appears exactly once, with no repeat;
- the next call returns the first null pointer on sum `W+2H+1` and terminates;
- therefore the loop makes exactly `H*(2W-1)` Recalc calls plus one terminating
  `Next` call. It is neither decoded-pack y/x order nor a rectangular array walk.

`[ACTIVE-YR: YES]` `MapClass::Resize @ 0x00565C10`, decompile plus allocation-loop
disassembly `0x00566368..0x0056642C`, allocates/publishes cells under those exact
four inequalities. Thus the iterator extent is data-independent after a successful
Resize. A Cell allocation failure is not a shorter valid sweep: Resize immediately
dereferences the published result to store level, so that load cannot complete.

`[ACTIVE-YR: YES]` The later `InitCellAttributes(0)` main sweep initializes the
same iterator state inline and uses the same `Next` function. Its Recalc count and
order are identical. A preceding flag-only sweep in that function also visits the
same count/order but does not call Recalc.

## Native row admission and rejection

The table distinguishes reader admission from Mark success. A row can pass the
reader and still fail Mark's steep-slope gate.

| Gate, in native order | Active in YR | Verified rule and consequence |
|---|---|---|
| Format | **Conditional** | `NewINIFormat > 1` gates both sections. Missing key defaults to `0`; values `<=1` skip them. |
| Section length | **Conditional** | Each section independently requires a signed-positive return length. Data is not nested under identity success. |
| Decoded identity | **Conditional** | The one-byte identity is initialized as `0xFFFFFFFF`; `0xFF` or a failed byte read leaves the sentinel and skips the row. Other bytes become unsigned `0..254`. |
| Type usability | **Conditional** | `OverlayType` virtual `+0x9C` must return an image, or type field `+0x29C` CellAnim must be non-null. |
| Multiplayer crate | **Conditional** | Row is accepted when `g_GameMode @ 0x00A8B238 == 0` or type `+0x2AA Crate == false`; a crate is rejected in nonzero game mode. |
| Native radar geometry | **Conditional** | `Cell_in_bounds_check @ 0x00568300` requires, for `W=Map+0xF4`, `H=Map+0xF8`: `W < x+y`, `x-y < W`, `y-x < W`, and `x+y <= W+2H`. This is a diamond/radar test, not an independent x/y rectangle clamp. |
| Overlay registry bounds | **Yes, negative fact** | Native performs no count, index-range, or null-entry guard before `g_OverlayTypeClass_Array[id]`. Malformed IDs can dereference invalid state; “unknown IDs are silently rejected” is not native behavior. |
| Allocation | **Conditional** | `operator_new(0xB0)` must return non-null for constructor/Mark. The high-only save/restore check still executes after allocation failure, harmlessly restoring the saved byte. |
| Constructor ground-list blocker | **No on this load path** | Constructor queries the Terrain ground list before Unlimbo, but `[Terrain]` is read later at `0x00687A74`; no authored Terrain object is live here. |
| Mark base call | **Yes** | Fresh-object `ObjectClass::Mark(1)` establishes the object mark/redraw state before derived dispatch. |
| Steep slope | **Conditional** | `OverlayClass::Mark` rejects `Cell+0x11C > 4` for every ID except unrelated `0xB2`. All high and low bridge IDs are subject to this gate. It occurs after base Mark/redraw but before high/low/ordinary writes. |
| Ordinary passability / Overrides | **No on this load path** | The nonzero Full_Init suppression counter bypasses the ordinary `CheckCellPassability` and prior-overlay `Overrides` pair. Low procedural dispatch occurs before that ordinary branch in any case. |

`[ACTIVE-YR: YES]` The accepted-row test order matters. Native asks the type
array for image/CellAnim before checking mode, coordinate, allocation, or slope.
A safe Rust load error for malformed type IDs can be justified as robustness, but
it must not be documented as a native rejection filter.

## Exact high/low/ordinary interleaving

### Four high anchors

| ID | Retail identity | Active in YR | Mark call | Temporary anchor data before owner restore |
|---:|---|---|---|---:|
| `0x18` | `BRIDGE1` | **Yes** | `CellClass::SetBridgeDirection_NESW(0,1) @ 0x0047E040` | `0` |
| `0x19` | `BRIDGE2` | **Yes** | `CellClass::SetBridgeDirection_NESW(6,1)` | `9` |
| `0xED` | `BRIDGEB1` | **Yes** | `CellClass::SetBridgeDirection_NWSE(0,1) @ 0x0047E470` | `0` |
| `0xEE` | `BRIDGEB2` | **Yes** | `CellClass::SetBridgeDirection_NWSE(6,1)` | `9` |

`[ACTIVE-YR: YES]` For every reader-admitted high row, the owner first saves
the anchor byte `Cell+0x11E`. When object allocation and Mark's slope gate pass,
Mark invokes the setter,
continues into the ordinary branch, stores the high identity, deliberately does
not zero the high state byte, and runs the common anchor Recalc while the setter's
temporary `0`/`9` is still present. Only after Mark returns does the owner re-fetch
the same anchor and restore the saved byte. The owner restores no neighbor byte and
no structural field. Therefore:

- a prior low/ordinary/high-neighbor write at the later high anchor is the byte
  saved and restored;
- setter structural bits and neighbor state writes remain unless a later packed
  coordinate overwrites them;
- Recalc at the high Mark sees the setter byte, not the restored byte;
- later `OverlayDataPack` wins at the anchor and neighbors; the post-data
  Recalc is the first Recalc guaranteed to see that data-pack result.

The exact Mark writes are visible in disassembly at `0x005FD09F..0x005FD10C`:

| Mark class | Active in YR | `Cell+0x44` identity write | `Cell+0x11E` state before common Recalc |
|---|---|---|---|
| Four high anchors | **Conditional** | Always writes the current type ID after the setter. | Skips the ordinary zero store; retains setter-derived `0`/`9` until the owner restores the pre-Mark anchor byte after common Recalc. |
| Ordinary non-high | **Conditional** | Writes the current type ID. | Writes `0`; if type Land code is `5`, rewrites `1` and calls `SpreadCellGerminate(0)`; if Crate is true, rewrites `0xFF` last. |
| Retail low body bands | **Conditional** | They use ordinary non-high identity write. | Retail Road/non-crate rows therefore enter common Recalc with `0`. |
| Low procedural triggers | **Conditional** | Do not pass through the general ordinary identity store. | Their settled direct fixed/body writes own generated identities/data; the original packed anchor still receives the common Recalc/cleanup if Mark reaches the tail. |

`[ACTIVE-YR: YES]` The Full_Init suppression counter makes the ordinary placement
predicate true, so the table's ordinary identity/state writes execute for every
reader-admitted, object-allocated, slope-accepted ordinary row. The four high exceptions
are keyed from the original Mark type ID, not the cell's prior identity.

### Low procedural and ordinary rows

`[ACTIVE-YR: CONDITIONAL]` Mark selects only `0x7A..0x7D` and
`0xE9..0xEC` as low procedural triggers. The already-settled body bands
`0x4A..0x65` and `0xCD..0xE8` do not enter procedural expansion; they traverse the
ordinary placement branch. The trigger's fixed writes, opposing-end scan, and any
body materialization complete inside that one Mark call. Every generated fixed or
body cell calls `RecalcAttributes(-1)` immediately; the body transaction consumes
the settled `3*L` raw Scenario words in longitudinal-outer/cross-row-inner order.

`[ACTIVE-YR: YES]` After either low family finishes, including its no-op/failure
arms that reach the tail, Mark calls `RecalcAttributes(-1)` on the original packed
anchor, clears object `+0x74 IsOnMap`, sets `+0x81 InLimbo`, invokes virtual
`+0xF8 UnInit`, and returns true. A steep-slope rejection returns earlier and does
not execute this tail.

`[ACTIVE-YR: YES]` The transaction-level tactical-dirty side effect is earlier
than that tail. `ObjectClass::Mark` marks the fresh object down and dispatches
vtable `+0x134` to `ObjectClass::MarkNeedsRedraw @ 0x005F4D10`. If redraw was not
already set, that function sets object `+0x80`, calls `FUN_004F42F0(0)`, and—when
`g_Tactical` is non-null—sets `Tactical+0xD7D=1`. The zero argument does not take
the optional bridge-counter path. Procedurally generated fixed/body cells call
Recalc directly and do not each create an object or repeat this dirty call. Thus
“the low common tail dirties every generated cell” is false; one accepted trigger
object dirties before dispatch, while the common tail contains Recalc and cleanup.

### Consequence of synchronous order

`[ACTIVE-YR: YES]` Suppose an early packed low trigger materializes identity and
data at a coordinate whose own non-`0xFF` packed row occurs later in y/x order.
The later row sees the already-mutated cell and performs its own Mark at that time;
if it succeeds, its ordinary/high/low writes can replace or build on the earlier
state. A coordinate already passed is not revisited. After all identities, the
data pass overwrites state bytes without changing identities. This is the required
player scenario: low expansion and its Scenario draws occur before later packed
rows, later data bytes, and the first authored Techno constructor.

## OverlayData and Recalc ownership

- `[ACTIVE-YR: CONDITIONAL]` `OverlayDataPack` is attempted whenever
  `NewINIFormat > 1`, even if `OverlayPack` is missing or decodes to zero length.
- `[ACTIVE-YR: CONDITIONAL]` For a positive data length, each of 512×512 decoded
  positions is initialized to data byte `0`, read once, checked only by
  `Cell_in_bounds_check`, then written to `Cell+0x11E`. There is no identity,
  SHP/CellAnim, crate, type, slope, high, low, or existing-overlay gate.
- `[ACTIVE-YR: YES]` Identity-empty and identity-pass-rejected cells can therefore
  retain nonzero packed data. The data pass does not repair or create identity.
- `[ACTIVE-YR: YES]` There is no per-cell or whole-map Recalc between the data
  store and the reader return. `Full_Init` owns the subsequent anti-diagonal
  `H*(2W-1)`-call sweep.
- `[ACTIVE-YR: YES]` `RecalcAttributes(-1)` preserves `Cell+0x11B Level`, returns
  immediately for the shared dummy, reads the current identity/data, and executes
  the complete attribute body. It does not recreate the high setter's structural
  stamp; those fields must already survive the inline high call.

### LAT and slope work is repeated at Recalc call sites

`[ACTIVE-YR: YES]` Both each Mark/common-tail Recalc and each post-data sweep
Recalc call `CellClass__ApplyLAT_and_SlopeFixup @ 0x0047CA80` on their applicable
valid-tile paths (`0x0047D54A` early-overlay call; `0x0047D813` normal call).
This is not the ordinary random tile-variant selector. It deterministically:

1. applies Rough, Sand, Green, then Pave LAT masks from the **live** N/E/S/W
   neighbor tile IDs;
2. applies the exact ramp/smooth-ramp fixup from the cell and neighbor slope bytes;
3. writes `Cell+0x38 IsoTileTypeIndex` while preserving the subtile.

Per-overlay Recalc can therefore mutate a tile ID before later decoded coordinates
run, and the post-data anti-diagonal sweep can mutate it again using the then-live
neighbor state. A Rust implementation that computes LAT only once in Fill and
then treats overlay Recalc as a land/zone patch is not native-equivalent.

### CliffBack work is repeated at Recalc call sites

`[ACTIVE-YR: YES]` Retail `rulesmd.ini` sets
`CliffBackImpassability=2`. Every non-dummy Recalc reaches one of three verified
CliffBack sites when its path permits: early overlay-claimed, sparse/unusable
subtile, or normal/sentinel. Each examines the same six neighbor coordinates and
the signed level boundary `neighbor_level >= cell_level + 4`; mode `2` can rewrite
`LandType` to native code `3` before `RecalcZoneType` and zone-cache stores.
The early overlay-claimed branch is not limited by the normal branch's land filter.
Thus per-overlay and both whole-map Recalc sweeps execute CliffBack, not just Fill.

### Terrain-attached animation latch work is call-order sensitive

`[ACTIVE-YR: CONDITIONAL]` On the valid-tile normal path, Recalc captures the
registered pristine tile receiver **before** LAT, then checks:

- `Cell Flags+0x140 & 0x20000 == 0`;
- pristine tile `+0x2C8` AnimType index is not `-1`;
- pristine tile `+0x2D4 AttachesTo` equals `Cell+0x11A` subtile.

If all pass, `0x0047D98E..0x0047DA88` constructs the terrain-attached `AnimClass`,
sets `Anim+0x196=1`, `Anim+0x100=tile+0x2D8`, `Anim+0x197=1`, and latches cell bit
`0x20000`. An ordinary/high per-Mark Recalc can therefore create and latch the
animation before the post-data sweep; that sweep sees the bit and does not create
a duplicate. A later unlatched cell can first create during the anti-diagonal
post-data sweep.

`[ACTIVE-YR: YES, RETAIL-DATA]` The low trigger rows and most low body rows have
`NoUseTileLandType=true`, so Recalc takes the early overlay-claimed path: it still
runs CliffBack, LAT, zone, and cache work, but returns before the tile-animation
latch block. The four high anchors have `NoUseTileLandType=false` and can take the
normal latch path. Retail `LOBRDG27/28` and `LOBRDB27/28` also set
`NoUseTileLandType=false`, so they must not be generalized with the earlier body
rows.

`[ACTIVE-YR: YES]` These early tile animations are transient. After Terrain,
Units, Aircraft, Infantry, Structures, and Smudge, `Full_Init @ 0x00687B92` calls
`MapClass::InitCellAttributes(0)`. That function first destroys every
`Anim+0x197` instance, then its main anti-diagonal sweep clears cell latch
`0x20000` immediately before calling Recalc. The resulting animations are the
surviving post-object set and are constructed in that sweep's anti-diagonal order.
The native load therefore has distinct Fill/materialization, inline overlay
Recalc, post-data Recalc, and post-object Recalc boundaries; only the ordinary
random variant choice belongs solely to the earlier materialization owner.

## Active retail data and reachability

`[RETAIL-DATA]` Dense `[OverlayTypes]` enumeration in retail `rulesmd.ini` gives:

- `0x18 BRIDGE1`, `0x19 BRIDGE2`;
- `0x7A..0x7D LOBRDGE1..4`;
- `0xE9..0xEC LOBRDGB1..4`;
- `0xED BRIDGEB1`, `0xEE BRIDGEB2`.

`[RETAIL-DATA]` All eight low triggers have an `Image`, `Land=Road`, and
`NoUseTileLandType=true`. The four high types have images and `Overrides=yes`;
`artmd.ini` provides theater-specific `[BRIDGE]` and `[BRIDGB]` art. These rows
satisfy the reader's image side of the admission test when their theater art is
resolved.

`[RETAIL-DATA]` `Lostlake.mmx`
(`39AE274E92A64CA1D5534876DE81DFFAF7153A696900B22A288D3EDB52C81143`),
`Killer.mmx`
(`423C7A997D80F964B4490910DA17124EFB3B42D49E14B191BBB281D3AE565845`), and
`Shrapnel.mmx`
(`3D0955DAA3CC146688D88555C6D0938A2E58648DEB25AFF51592EB5D8DAC77E0`)
all declare `NewINIFormat=4` and contain both pack sections. They contain ordinary
low body/destroyed rows but no procedural triggers. The larger recent census also
found zero triggers in 385 shipped payloads; that is a content census, not an
executable-liveness negative.

`[ACTIVE-YR: CONDITIONAL]` The sole reader caller is active `Full_Init`. Ordinary
authored format-4 loads enter the pack transaction. The generated `.SED` synthetic
Full_Init also reaches the reader call but omits `NewINIFormat`, so the default-0
gate makes both packs inert before the generator directly materializes its bridge.
The exhaustive cross-context matrix is deliberately left to its assigned slot;
no restore-path conclusion is inferred here.

## OpenTS correspondences and YR exclusions

- `[LEAD CONFIRMED IN YR]` OpenTS `OverlayClass::Read_INI` has the same broad
  `NewINIFormat`, y/x, constructor, high save/restore, data-pass skeleton. Active
  YR decompile/disassembly independently confirms each used correspondence.
- `[TS-ONLY NAME; ACTIVE-YR: NO]` OpenTS calls the second high pair
  `OVERLAY_RAIL_BRIDGE1/2` and dispatches `Set_Under_Rail_Bridge`. Active YR retail
  IDs `0xED/0xEE` are `BRIDGEB1/2` and take the high NWSE setter. Do not import rail
  or TrainBridge semantics from the OpenTS names.
- `[ACTIVE-YR: NO]` Low trigger/end/body placement does not create a `TubeClass`.
  `Full_Init` reads explicit `[Tubes]` through a separate owner before overlays.
  Low `Land=Road` / `NoUseTileLandType` is overlay/Recalc metadata, not implicit
  Tube topology.
- `[ACTIVE-YR: NO]` `FUN_005FDDF0` after the post-data Recalc is a vein-`0x7E` cleanup,
  not a hidden bridge completion phase.

## Current Rust ownership mismatch

- `[RUST]` `src/map/map_file.rs:270` always parses overlay packs after Basic;
  `basic.new_ini_format` is stored but does not gate authored execution.
- `[RUST]` `src/map/resolved_terrain.rs:2438..2481` performs a post-build loop over
  decoded entries, writes identities directly, and only calls the high stamp
  helper. It has no native admission/Mark transaction, no inline low dispatch,
  and no shared Scenario cursor.
- `[RUST]` Before that loop, the rectangular cell build at
  `src/map/resolved_terrain.rs:2088..2288` classifies raw OverlayPack entries while
  choosing LAT-derived metadata, CliffBack eligibility, and tile-animation rows;
  CliffBack is then batch-applied at `:2584..2697`, and animations are merely
  sorted to anti-diagonal order at `:2421`. Native does not finalize these effects
  from raw rows before admission: every successful Mark invokes full Recalc at
  that decoded coordinate, the post-data sweep invokes it again in iterator order,
  and the post-object sweep resets/recreates the animation latch.
- `[RUST]` `src/app/loading/init.rs:1879..1883` selects authored versus generated
  overlay behavior from `generated_construction_trace.is_some()`, an incidental
  proxy rather than the explicit load-source provenance.
- `[RUST]` `src/sim/overlay_grid.rs:206..257` reconstructs the overlay grid again
  from raw packs, applies a subset of admission/slope rules, recalculates entries,
  and then writes data. Production invokes this second owner at
  `src/app/loading/init.rs:1117` and `:1987`. It neither performs high/low Mark nor
  consumes a finalized map payload.
- `[RUST]` Consequently current Rust cannot reproduce early-low -> later-row ->
  data overwrite ordering or its Scenario-word effect before authored Technos,
  can let rejected/raw rows influence precomputed LAT/CliffBack/animation state,
  and can project resolved terrain from bytes different from the runtime
  OverlayGrid bytes. Sorting a prebuilt animation vector does not reproduce the
  native create/latch -> no-duplicate post-data sweep -> destroy/clear/recreate
  post-object lifecycle.

## Adversarial corner cases

| Case | Active in YR | Verified answer / required acceptance property |
|---:|---|---|
| 1 | **Yes** | A map with both sections but `NewINIFormat=1` executes neither pack body; the reader still drains deferred finalization and `Full_Init` still runs the anti-diagonal post-data Recalc sweep. |
| 2 | **Yes** | Format 4 with absent/empty `OverlayPack` but positive `OverlayDataPack` writes data to every native in-radar cell, including identity-empty cells, then gets the anti-diagonal post-data sweep. |
| 3 | **Conditional** | An early valid low trigger that writes a future packed coordinate completes and consumes its words first; the later accepted packed row can overwrite it; data pack overwrites state last. |
| 4 | **Conditional** | A high anchor whose cell already carries state from an earlier procedural/neighbor write saves that byte, recalcs with setter data `0/9`, restores the saved byte, then yields to any data-pack byte and the post-data Recalc. |
| 5 | **Conditional** | A high or low bridge row on slope `>4` passes reader admission but derived Mark rejects before stamping/expansion/common tail; base object redraw/tactical dirty has already occurred. |
| 6 | **Conditional** | A crate with valid art and coordinate in nonzero game mode is rejected before allocation, so it produces no object Mark/tactical dirty. |
| 7 | **Yes** | Radar-edge equality is asymmetric: `x+y == W` is rejected, while `x+y == W+2H` is accepted if both diagonal inequalities are strict. Rust must use the native mask/geometry, not only width/height. |
| 8 | **Yes, malformed-data negative** | A decoded non-`0xFF` ID outside the live type registry has no native bounds/null guard. A Rust safety error is acceptable policy only if documented as deliberate crash avoidance, not parity filtering. |
| 9 | **Conditional** | Allocation failure performs no constructor/Mark; a high ID still reaches the owner restore check and writes back the unchanged saved byte. |
| 10 | **Conditional; settled body logic** | A valid trigger with clear fixed row but no opposing end retains its synchronous fixed writes and Recalcs, consumes no body words, then runs original-anchor common cleanup. |
| 11 | **Yes** | Two neighboring LAT cells touched by decoded rows are recalculated in pack y/x order first, then every live cell is recalculated in anti-diagonal order. A one-time rectangular Fill batch cannot substitute for those live-neighbor passes. |
| 12 | **Conditional** | A high/ordinary Mark on a tile-animation attachment can create and latch the transient Anim before the post-data sweep; that sweep must not duplicate it. Post-object InitCellAttributes must then delete it, clear the latch, and recreate the surviving instance in anti-diagonal order. |

## Coverage ledger

| Target | Evidence used | Active in YR | Result |
|---|---|---|---|
| `ReadMapOverlayPacks 0x005FD2E0` | full decompile, full disassembly, callees, sole caller | **Yes/Conditional gate** | Closed: sections, filters, loops, high save/restore, data pass, drain. |
| `ScenarioClass::Read_INI_Basic` key path | string xref and store disassembly | **Yes** | Closed: default `0`, global, exact `>1` predicate. |
| `Cell_in_bounds_check 0x00568300` | full decompile + disassembly | **Yes** | Closed: four diamond inequalities used in both passes. |
| Overlay ctor / Unlimbo / vtable binding | full decompile + disassembly + vtable/COL memory | **Conditional admitted row** | Closed: synchronous `Mark(1)` before caller advances. |
| `OverlayClass::Mark 0x005FC570` | full decompile + disassembly + callee list | **Conditional** | Closed at high/low/ordinary dispatch and tail boundaries. |
| Low internal tables/search/body | three 2026-08-30 reports + entry/RNG/write cold checks | **Conditional** | Settled prior evidence retained; no contradiction found, no duplicate investigation. |
| `SetBridgeDirection_NESW/NWSE` | full decompile + disassembly at `0x0047E040/0x0047E470` | **Conditional high IDs** | Closed: four calls, direction/state args, temporary data, structural persistence. |
| Base redraw/tactical dirty | `ObjectClass::Mark`, vtable slot, `0x005F4D10`, `0x004F42F0`, all decompile + disassembly | **Conditional object Mark** | Closed: one pre-dispatch dirty, not generated-cell/common-tail dirty. |
| `CellClass::RecalcAttributes 0x0047D2B0` | full decompile + full disassembly + caller order | **Yes** | Closed for current identity/data, ordinary/high inputs, LAT, CliffBack, tile latch, zone/cache, and dummy return; unrelated Tube/shadow-caster branches excluded. |
| `CellClass::ApplyLAT_and_SlopeFixup 0x0047CA80` | full decompile + disassembly + Recalc callsites | **Yes on applicable paths** | Closed: live-neighbor LAT order, slope fixup, tile-index write, no ordinary variant selector. |
| `CellIterator_Init/Next 0x00578350/0x00578290` | full decompile + disassembly + Full_Init loop | **Yes** | Closed: `(1,W)` start, anti-diagonals, exact `H*(2W-1)` Recalc count, terminating null. |
| `MapClass::Resize 0x00565C10` | allocation-loop decompile + disassembly `0x00566368..0x0056642C` | **Yes** | Closed: exactly the same diamond predicate publishes all iterator cells on a successful load. |
| `MapClass::InitCellAttributes 0x00568BB0` | full decompile + disassembly + Full_Init callsite | **Yes** | Closed: delete terrain-attached Anim, equal-count flag sweeps, latch clear, post-object Recalc/recreation. |
| `ScenarioClass::Full_Init 0x00686B20` | full decompile + targeted disassembly | **Yes** | Closed: suppression, Tubes, reader, post-data sweep, vein exclusion, Terrain/Techno order, post-object InitCellAttributes. |
| Retail rules/art/maps | direct file reads and hashes | **Content-conditional** | Closed: IDs, key flags, format-4 fixtures, stock-trigger absence. |
| OpenTS correspondence | direct `overlay.cpp` read | **Not authority** | Closed: useful skeleton; rail/Tube/TS semantics excluded. |
| Current Rust | direct source read | n/a | Closed: split owners, missing gate/low/RNG/finalized payload identified. |
| Shared-dummy field census | assigned slot 2 | n/a | Explicitly excluded/delegated. |
| Full load-context matrix | assigned slot 3 | n/a | Explicitly excluded/delegated. |

## Open Questions Log — final drain

| ID | Question | Disposition |
|---:|---|---|
| Q01 | Is the open program the active retail binary? | **Resolved**: executable path, image base, architecture, and SHA above. |
| Q02 | What active function calls the reader? | **Resolved**: sole direct call is `Full_Init+0xF14`. |
| Q03 | Where is `NewINIFormat` sourced and what is its default? | **Resolved**: Basic int read, default `0`, global `0x00A8ED7C`. |
| Q04 | Is the predicate `>=1`, `>1`, or section-presence only? | **Resolved**: signed `>1`. |
| Q05 | Are identity and data mutually dependent? | **Resolved**: independent positive-length checks inside the format gate. |
| Q06 | What is the exact coordinate order? | **Resolved**: y outer, x inner, each `0..511`. |
| Q07 | What decoded values skip identity? | **Resolved**: `0xFF`/unchanged `-1`; remaining byte is unsigned. |
| Q08 | What type-art filter runs? | **Resolved**: SHP virtual non-null OR CellAnim non-null. |
| Q09 | What crate filter runs? | **Resolved**: reject Crate only when game mode is nonzero. |
| Q10 | What is native in-radar geometry? | **Resolved**: four exact diamond inequalities. |
| Q11 | Does native guard the registry index? | **Resolved**: no bounds/null guard. |
| Q12 | Does constructor really dispatch this Mark? | **Resolved**: ctor -> Unlimbo -> Overlay vtable `+0x124`. |
| Q13 | Can authored Terrain block construction here? | **Resolved**: no; Terrain section is later. |
| Q14 | What Mark gate remains active during load suppression? | **Resolved**: universal slope `>4` rejection except `0xB2`. |
| Q15 | Which four high calls and arguments execute? | **Resolved**: 18/19 -> NESW 0/6; ED/EE -> NWSE 0/6; state 1. |
| Q16 | What is the exact high prior-state window? | **Resolved**: save before alloc/ctor, restore after complete Mark, anchor byte only. |
| Q17 | What survives high restore? | **Resolved**: structural/neighbor writes survive; only anchor `+0x11E` restored. |
| Q18 | Are high, low, and ordinary rows phase-separated? | **Resolved**: no; one synchronous decoded traversal. |
| Q19 | What are the low trigger ranges? | **Resolved**: `7A..7D`, `E9..EC`. |
| Q20 | Do body bands invoke procedural expansion? | **Resolved/settled**: no; `4A..65`, `CD..E8` are ordinary rows. |
| Q21 | Where and when do low body RNG words occur? | **Resolved/settled**: inline `3*L` raw Scenario `Next & 3`, before later rows/Technos. |
| Q22 | Where is tactical dirty set? | **Resolved**: base Object Mark -> MarkNeedsRedraw -> `0x004F42F0(0)`, before low dispatch. |
| Q23 | What exactly is in the low/common success tail? | **Resolved**: original-cell Recalc, IsOnMap clear, InLimbo set, UnInit. |
| Q24 | Can data overwrite rejected or identity-empty cells? | **Resolved**: yes, after only radar check. |
| Q25 | Can data overwrite high restore and low-generated data? | **Resolved**: yes; it is the complete second pass. |
| Q26 | Where is the first guaranteed Recalc after data? | **Resolved**: Full_Init global sweep at `0x00687A5A`. |
| Q27 | Is there a hidden bridge post-step after the sweep? | **Resolved**: no; `0x005FDDF0` is vein-only. |
| Q28 | Is authored low execution active YR or TS-only? | **Resolved**: active content-conditional YR; retail rules declare it and active Mark dispatches it. |
| Q29 | Which additional dummy fields alias across low writes? | **Delegated/out of scope**: slot 2; no assumption imported. |
| Q30 | Which complete fresh/restore contexts can reach Full_Init/Mark? | **Delegated/out of scope**: slot 3; exact owner and authored-format reachability here are closed. |
| Q31 | What exact count/order does the post-data whole-map Recalc use? | **Resolved after inventory reopen**: anti-diagonal, exactly `H*(2W-1)` live-cell calls. |
| Q32 | Does per-Mark Recalc rerun LAT/slope work or only land/zone? | **Resolved after inventory reopen**: full `ApplyLAT_and_SlopeFixup` on applicable paths, using live neighbors. |
| Q33 | Do per-Mark and post-data Recalc execute CliffBack? | **Resolved after inventory reopen**: yes at their branch-specific sites; retail mode is `2`. |
| Q34 | Can per-Mark Recalc create/latch a terrain-tile animation? | **Resolved after inventory reopen**: yes on the normal valid-tile path; low `NoUseTileLandType=true` early branch skips it. |
| Q35 | Is the post-data sweep the last Recalc/animation owner in Full_Init? | **Resolved after inventory reopen**: no; post-object `InitCellAttributes(0)` deletes, unlatches, and recreates through another equal-count sweep. |
| Q36 | What exactly do ordinary and high Mark write before common Recalc? | **Resolved after inventory reopen**: identity always; ordinary state `0` then Land-5 `1` then Crate `FF`; four highs retain setter state until owner restore. |

## Zero-add and cold spot-check pass

After the first Q01–Q30 drain, the owner/Mark zero-add pass found no hidden bridge
dispatcher, Tube call, or reader-owned data Recalc. The parent then correctly
reopened the living inventory on the load-bearing Recalc boundary; Q31–Q36 were
seeded and resolved rather than treating “Recalc” as an opaque projection. A
second fresh decompile/disassembly/caller pass over Recalc, LAT, both iterator
functions, InitCellAttributes, Mark writes, and Full_Init added **zero further
questions**. Non-bridge Mark/Recalc callees were classified outside this corridor.

1. **Cold spot-check A:** fresh owner disassembly reconfirmed `CMP NewINIFormat,1 /
   JLE`, filter call order, save at `0x005FD4A4`, ctor at `0x005FD4D2`, exact
   four-ID restore at `0x005FD4DB..502`, inner-before-outer increments, and the
   data store at `0x005FD640` with its separate nested loops.
2. **Cold spot-check B:** fresh Full_Init disassembly reconfirmed Tubes
   `0x00687A0B`, sole reader call `0x00687A34`, iterator Recalc call
   `0x00687A5A`, vein-only call `0x00687A6D`, and Terrain `0x00687A74` in that
   exact order.
3. **Cold spot-check C:** fresh vtable/caller inspection reconfirmed
   `OverlayClass` slot binding, Mark's two high setters, and
   `ObjectClass::MarkNeedsRedraw -> 0x004F42F0(0)`; no generated low cell repeats
   that object-level dirty dispatch.
4. **Cold spot-check D:** fresh iterator decompile/disassembly independently
   reconfirmed the `(1,W)` start, up-right diagonal step, alternating `W`/`W-1`
   runs, first-null stop, and `H*(2W-1)` live-cell count for both whole-map Recalc
   sweeps; fresh Resize allocation-loop decompile/disassembly reconfirmed that the
   same four inequalities publish the complete successful-load iterator extent.
5. **Cold spot-check E:** fresh Recalc/LAT/InitCellAttributes disassembly
   reconfirmed both LAT callsites (`0x0047D54A`, `0x0047D813`), animation latch
   test/write (`0x0047D98E..0x0047DA88`), retail CliffBack sites, post-object
   latch clear (`0x00568CC7`), and Recalc call (`0x00568DF4`).

## Implementation handoff

These are three acceptance facets of one dependency-coherent authored-overlay
transaction, not three independent reconstruction owners.

| # | Verified behavior -> Rust delta -> surface | Acceptance -> proposed Rust test | Risk if wrong |
|---:|---|---|---|
| 1 | `[ACTIVE-YR]` `NewINIFormat>1`, native row filters, exact ordinary/high identity-state writes, and one y/x synchronous high/low/ordinary Mark traversal -> replace the post-build high-only loop with one map-owned authored inline transaction driven by explicit authored provenance and the native radar mask -> `src/map/map_file.rs`, `src/map/resolved_terrain.rs`, narrow map overlay owner, `src/app/loading/init.rs` | Mixed fixture has an early low trigger, later packed high/body/ordinary rows, prior high-anchor data, a format-1 control, and crate/art/slope/radar-edge rejects; it proves each row's pre-Recalc bytes and high restore -> `gsi_04_12_13_authored_overlaypack_executes_one_inline_yx_transaction` | Frequent authored maps can acquire wrong identity, high topology, attribute input, or deterministic order; custom trigger maps shift RNG. |
| 2 | `[ACTIVE-YR]` data blindly wins, then exactly `H*(2W-1)` full Recalcs run anti-diagonally, repeating live-neighbor LAT, branch-specific CliffBack, zone/cache and conditional tile-animation latch; a later post-object equal-count sweep deletes/unlatches/recreates the surviving animations -> split Fill's ordinary variant materialization from these two native Recalc boundaries, then retain one consumed-once finalized overlay/cell payload for runtime with no raw-pack rebuild -> `src/map/resolved_terrain.rs`, tile-animation load owner, `src/sim/overlay_grid.rs`, both app loading call sites | Interleaved high/low/data fixture asserts tile IDs, CliffBack land, zone/cache, exact Recalc coordinate trace, no duplicate early tile Anim, and post-object recreation order -> `gsi_04_12_overlay_finalization_recalc_runs_lat_cliffback_and_tile_latch_in_native_order` | A visually plausible map can have wrong LAT tiles, cliff passability, animation IDs/timing, zones, or runtime bytes even when bridge identities match. |
| 3 | `[ACTIVE-YR]` one trigger object dirties tactically before low dispatch; successful bodies consume settled `3*L` Scenario words before later pack rows and first authored Techno, while generated cells do not repeat object dirty -> borrow the single bootstrap raw RNG adapter into map Mark, emit one dirty intent per constructed row as required, return the advanced cursor and finalized payload before Techno construction -> low-Mark map owner, scenario bootstrap/app load boundary, authored spawn ordering | Early trigger expands, later packed row and data overwrite it, then Unit/Aircraft/Infantry/Structure constructors receive the exact subsequent native words; no-op/slope/blocked arms and generated cells prove zero extra draws/dirty calls -> `gsi_04_13_low_trigger_precedes_later_pack_overlaydata_and_first_techno_word` | First custom trigger shifts all later constructor randomness, or batching/double-dirty changes load side effects. |

## Negative facts / do not do

1. `[ACTIVE-YR: NO]` Do not classify `0xED/0xEE` as low or rail; they are active
   high `BRIDGEB1/2` anchors dispatched through `SetBridgeDirection_NWSE`.
2. `[ACTIVE-YR: NO]` Do not run high-before-low, component, endpoint, or post-data
   Mark phases. The only authored identity transaction is decoded y/x order.
3. `[ACTIVE-YR: NO]` Do not synthesize Tube topology from low triggers or Road
   land. Explicit `[Tubes]` has a separate earlier reader.
4. `[ACTIVE-YR: NO]` Do not precompute LAT/CliffBack/tile-animation state once in
   rectangular Fill and then rebuild runtime overlays from raw packs. Native runs
   full per-Mark Recalc, an anti-diagonal post-data sweep, and a post-object
   delete/unlatch/Recalc sweep before runtime consumes the resulting state.
5. `[ACTIVE-YR: NO]` Do not call the low common tail a per-generated-cell tactical
   dirty operation; base Object Mark dirties once before dispatch, while generated
   cells only receive direct writes/Recalc.

## Stale-document wording to replace

- In `ASSET_PARSING_BRIDGES_GHIDRA_REPORT.md:932..933`, replace “Low bridge
  variant” with: **“Active-YR high `BRIDGEB1/2` anchor; Mark dispatches
  `SetBridgeDirection_NWSE(direction=0/6,state=1)`. Low procedural IDs end at
  `0xEC`.”**
- Replace any “reader returns immediately for `NewINIFormat<=1`” wording with:
  **“The pack bodies are skipped for `NewINIFormat<=1`; the common reader epilogue
  still calls `DrainDeferredFinalizationQueue`, and Full_Init's post-data sweep remains
  outside the gate.”**
- Replace any “low common tail dirties generated cells” wording with:
  **“The accepted ephemeral object dirties tactical state once through base
  `ObjectClass::Mark` before low dispatch. Generated fixed/body cells call Recalc
  directly; the common tail recalculates and uninits the original object.”**
- Replace any phase-split algorithm with: **“One y-outer/x-inner traversal; each
  coordinate completes constructor/Mark/high restore before the next coordinate;
  OverlayData is the only later pack pass.”**
- Replace any endpoint-Mark/Tube or OpenTS rail inference with: **“Low bridge
  overlay Mark and explicit `[Tubes]` are independent active-YR owners; OpenTS
  `RAIL_BRIDGE` nomenclature is not YR authority.”**
- Avoid saying the post-data Recalc “reconstructs the high stamp.” Use:
  **“The high setter creates structural facts inline; the post-data Recalc projects the
  post-OverlayData identity/state while preserving those structural fields.”**
- Replace “whole-map/final Recalc runs over all cells” without order/count with:
  **“The post-data sweep makes exactly `H*(2W-1)` full Recalc calls in playable-
  diamond anti-diagonal order from `(1,W)`; it reruns LAT, CliffBack, zone/cache,
  and conditional animation latching. A later post-object InitCellAttributes sweep
  deletes terrain-attached anims, clears latches, and makes the same ordered Recalc
  calls to create the surviving set.”**

## Remaining uncertainty

- **None inside the bounded owner/callee corridor.** Gate, row filters, exact
  traversal, synchronous interleaving, four-high restore window, low boundary,
  tactical-dirty origin, data overwrite, both Recalc sweeps, and active-YR reachability
  are closed with binary plus retail evidence.
- `[OUT OF SCOPE]` The exact shared-dummy field census is slot 2. This report only
  uses the verified fact that dummy Recalc returns and does not prescribe its
  additional aliased fields.
- `[OUT OF SCOPE]` The exhaustive fresh-load/restore context matrix is slot 3.
  This report proves the sole owner call and the authored `NewINIFormat>1` arm,
  not every app provenance constructor.
- `[NON-LOAD-BEARING]` Native behavior for truncated LCW output and malformed
  out-of-range type IDs was not elevated into a crash-fidelity requirement.
  Native's absence of a registry guard is verified; Rust's safe-error policy must
  be explicit.

## Ghidra annotation candidates (not applied)

- `ReadMapOverlayPacks @ 0x005FD2E0`: plate comment candidate —
  `NewINIFormat>1; one y/x constructor-Mark pass; high-only anchor-data restore;
  independent data pass; no post-data Recalc; always drains deferred finalization`.
- `ScenarioClass::Full_Init @ 0x00686B20`: disassembly comment candidate at
  `0x00687A34` — `overlay identity/Mark + data owner; immediately followed by
  post-data anti-diagonal Recalc before vein-only cleanup, Terrain, and Technos`.
- `Cell_in_bounds_check @ 0x00568300`: plate comment candidate — exact four
  radar-diamond inequalities used by both OverlayPack passes.
- `MapClass::CellIterator_Init/Next @ 0x00578350/0x00578290`: plate comment
  candidate — `(1,W)` seed; up-right anti-diagonals; `H*(2W-1)` live cells;
  first null on sum `W+2H+1` terminates.
- `CellClass::RecalcAttributes @ 0x0047D2B0`: plate comment candidate — per-Mark,
  post-data, and post-object callers execute branch-specific LAT/CliffBack and
  conditional terrain-animation latch; not a land/zone-only projection.
- `MapClass::InitCellAttributes @ 0x00568BB0`: plate comment candidate — deletes
  `Anim+0x197`, clears cell latch `0x20000`, then recreates through the main
  anti-diagonal Recalc sweep after authored object sections.
- `FUN_004F42F0 @ 0x004F42F0`: rename candidate
  `MapClass__SetTacticalDirtyAndOptionalBridgeState`; comment that argument `0`
  from `ObjectClass::MarkNeedsRedraw` only sets tactical dirty and does not take
  the optional bridge-counter path.
- `FUN_0041B110 @ 0x0041B110`: rename candidate
  `AircraftClass__ReadFromINI`; string and constructor call corroborate the
  Full_Init category-order edge.
- `FUN_0051FB00 @ 0x0051FB00`: rename candidate
  `InfantryClass__ReadFromINI`; string and constructor call corroborate the same
  edge.

No annotations were applied.
