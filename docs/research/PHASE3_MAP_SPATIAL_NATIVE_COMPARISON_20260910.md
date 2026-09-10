# Phase 3 map lookup and playfield native comparison

Date: 2026-09-10. Source baseline: `origin/main` `8f988ab2`.
Scope: bounded GSI-04.01 lookup and playfield evidence. No Phase 3 row is closed
by this comparison. The comparison exposed and corrects one production
LocalSize clipping operand-order mismatch for signed-overflow inputs.

## Native behavior established

The current Ghidra `testProsjekt` program `gamemd.exe` was read explicitly,
including the following bodies, assembly, and active callers. The original
retail executable has image base `0x00400000` and SHA-256
`1cdd1180e49024fbda8ad568caac2e86e856063ff67ab38f62b7d2c7bb84298c`.

| Native entry | Established behavior and active consumer |
| --- | --- |
| `0x005657A0` | Packed signed-i16 `y*512+x`; only the linear index is checked against `0x40000`. A nonnull slot returns its pointer without touching the dummy. Misses return `0x00ABDC50` and overwrite only its packed coordinate at `+0x24`. Ordinary map, movement and projectile callers use this shared lookup. |
| `0x00565730` | Signed world/lepton X/Y divide by 256 toward zero. Full i32 quotients feed wrapping linear-index arithmetic; only the miss path narrows to packed words. Its upper bound reads `MapClass+0x140`; normal retail table capacity is `0x40000`. Active calls include Map Logic at `0x004D245E`, Foot cell-action selection at `0x004DDE45`, Techno fire at `0x006FE1D0`, and Bullet detonation at `0x004695D6`. |
| `0x00578460` | Only the low byte of mode selects lookup. Mode zero performs no lookup or stamp. Nonzero mode reads the global table pointer `0x0087F924`, applies shared fallback, reads signed level `+0x11B` and unsigned slope `+0x11C`, and uses the strict upper-half slope correction and four signed wrapping inequalities. Active callers include `CellClass::RecalcZoneType` at `0x00483C90`, `CrateSlot::ValidateCellAndCreateOverlay` at `0x004A1915`, and aircraft landing at `0x00419B18`. |
| `0x005785F0` | Signed world division, then packed narrowing, then forced mode one at `0x0057862B`; Z is ignored. Active callers include aircraft Unlimbo at `0x00414352`, Building CanDock at `0x00457D31`, and Object Paradrop at `0x005F5952`. |
| `0x00567230` through `0x005672D3` | Calls the original ClipRect `0x00421B60`, writes normalized LocalSize, floors left/top at two and caps width/height against Size margins. The endpoint is **before** the redraw call; Techno membership updates are also outside this comparison. Radar map-bounds setup invokes the full owner at `0x0065449D`. |

The lookup code is shared active YR code. No TS-only activation is asserted.
Native constructors, input selection in every caller, allocation failure, and
the full Resize lifecycle are not executed here.

### Confirmed LocalSize correction

The pre-fix Rust test failed on `Size=0,0,80,80` and
`LocalSize=2147483647,2,2,20`: Rust produced
`[2147483647,2,-2147483569,20]`, while native execution produced `[2,2,0,0]`.
The other three comparison tests passed. Independent review identified the
operand-order error; a fresh builder disassembly read confirmed it.

At `0x00567233`, EDX receives raw LocalSize. The caller pushes Size at
`0x00567240..0x00567251`. ClipRect initializes its candidate from that stack
argument at `0x00421B6F..0x00421B7E`, while EDX supplies its clipping bounds.
Rust had reversed the rectangles. Ordinary positive intersections are
equivalent, but the intermediate signed wrapping branches are not. The
production helper now initializes its candidate as normalized Size and applies
the LocalSize bounds in the original order, retaining all native empty checks,
wrapping operations and the subsequent margins.

This input reaches active retail code: `ScenarioClass::Full_Init` calls map
loading at `0x006879FF`; `Read_Map_Section_And_IsoMapPacks @ 0x004ACE70`
reads LocalSize at `0x004AD77A` through `INIClass::ReadRect @ 0x00527CC0`,
stores its signed dwords, then invokes `RadarClass::ComputeRadarMapBounds`,
which calls normalization at `0x0065449D`. ReadRect copies into its 64-byte
buffer and scans `%d,%d,%d,%d`; INT_MAX is representable and this text fits.
No shipped-retail occurrence is established. The verified retail nearoref
header is unaffected. The trigger action integer-input boundary is not needed
to establish this physical-map-input path and is not newly certified here.

The original map supplied by default-active asset lookup is `nearoref.map`
from `multimd.mix`, entry `0x5E33E585`, 97,641 bytes, SHA-256
`d35a49d858e49bb31f63bbfe5d17e3cfcc2734fbfa4d4708c1b355ab6270d424`.
Its `[Map]` section has `Theater=TEMPERATE`, `Size=0,0,80,58`, and
`LocalSize=2,4,76,48`. A different `Size` in its Preview section is not the map
shape. The executable fixture uses these map dimensions in one case family;
the sparse cell allocation and level/slope cases are explicitly synthetic.

## Executable evidence and Rust production delivery

[The generator](../../tools/spatial_oracle/map_queries.py) executes original
instructions with the shared checked runner. It provides both table owners,
retail capacity, seven real slots, the full zeroed remainder of the pointer
table, and retained dummy bytes. Real pointers encode distinct fixture cells;
unexpected returned pointers fail generation. Input and dummy-height writes
are checked. No native call results or instruction bodies are substituted.

[The reference](../../tools/spatial_oracle/map_queries.json) and
[provenance sidecar](../../tools/spatial_oracle/map_queries.meta.json) preserve:

- 114 packed/world lookup cases: allocated/null/out-of-range slots, fixed-stride
  aliases, negative division boundaries, i16 narrowing and i32 index wrap.
- 15 LocalSize normalization prefixes, including the retail header, clipping,
  empty and small rectangles, signed limits and arithmetic wrap.
- 2,010 playfield cases: five final field sets; low-byte modes zero, one, 255
  and 256; signed level limits; slope zero/nonzero; both sides and equality at
  bounds and slope thresholds; packed aliases; 92 world-wrapper cases.
- Five sequential calls in one emulator retaining the first dummy pointer,
  showing later real hits preserve its coordinate and later misses mutate
  that same retained object without clearing its height/slope.

These are 2,144 bounded calls/prefixes, not an exhaustive proof of all signed
inputs, map lifecycle, or complete gameplay loops. Extreme signed field sets
exercise the arithmetic contract; they are not claimed as shipped map data.

[Rust comparison tests](../../src/sim/cell_rect_native_tests.rs) call the actual
owners in [cell_rect.rs](../../src/sim/cell_rect.rs) and
[playfield.rs](../../src/map/playfield.rs), using the same seven-slot allocation
and level/slope inputs. They compare native pointer-selected cell coordinates,
verdicts, dummy stamps, preserved fields, and retained shared identity.
The current production connections include:

- `ProjectileCollisionWorld::cell` uses the world lookup; its packed collision
  helpers use the packed lookup.
- `find_nearby_cell` and Simulation playfield membership use the same
  height-aware predicate.
- `Simulation::install_playfield_from_map_header` uses the same normalization
  owner for loaded map headers. Trigger LocalSize changes share that owner.

The normalization test also parses each physical signed header through
`MapFile::from_bytes` and installs it through the production Simulation seam,
comparing the resulting authority to the same native reference.

The tests do not substitute a second Rust algorithm. They demonstrate only the
bounded mechanism exposed by these production owners,
not equivalence of every caller's input or scheduling.

## Validation

Run from the repository root with the original retail executable configured:

```powershell
$env:VERA20K_GAMEMD_EXE = 'C:/your-retail-install/gamemd.exe'
python -m tools.spatial_oracle.map_queries --check
cargo test -p vera20k --lib sim::cell_rect::tests::map_
cargo test -p vera20k --lib
cargo clippy -p vera20k --lib
```

Generation and a separate read-only `--check` passed under Unicorn 2.1.4:
`native outputs match; no files written`. The sidecar records native identity,
binding/core versions, entry points and fixture assumptions. Pre-fix focused
Rust result: **3 passed, 1 failed**, the exact LocalSize drift above.
Post-fix focused result: **4 passed, 0 failed, 8,702 filtered out**, including
the physical-map-input production seam (0.04 seconds; initial recompile
4 minutes 15 seconds). The library emitted 58 existing warnings. Native
reference values were retained unchanged for the fix. A fresh independent
read-only critic passed the bounded code/evidence revision without findings.
Final-candidate full-library validation exited **0**: **8,587 passed, 0 failed,
119 ignored**, in 26.29 seconds. Clippy exited **0** in 3 minutes 20 seconds
with **1,144 warnings**; this increment does not resolve the repository warning
backlog. No Rust changes followed the validated implementation commit
`258a59ce`. Full local receipts are preserved in
`.local/spatial-full-lib.log` and `.local/spatial-clippy.log`; focused and native
check receipts are `.local/spatial-focused-fixed.log` and
`.local/spatial-native-check.log`. These local logs are not tracked artifacts.

## Residual scope

GSI-04.01 remains open for the phase-wide audit of CellClass constructor/ID
observability, Resize caller multiplicity and surrounding lifecycle, all
remaining shared-dummy writer/reader chains, and generic CellIterator consumer
ordering. The older local GSI-04.01 census was used only as a search lead:
its reservation-writer unknowns predate current production Mark/Clear and
perimeter work, so they cannot be copied as current gaps. The live projectile
world owner already has Size-diamond bounds; the unused `coord_is_on_grid`
helper is not evidence of a production divergence.

Bridge-zone routing, shroud, terrain mutations, and other Phase 3 mechanisms
have separate closure obligations. This evidence increment does not certify
or exclude them.
