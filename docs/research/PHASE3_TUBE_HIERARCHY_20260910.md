# Phase 3 — Tube hierarchy publication (2026-09-10)

Status: **OPEN, candidate under validation.** This increment implements the active
high/Tube hierarchy record helper and proven production prerequisites. It does not
close GSI-04.01, GSI-04.06, GSI-04.12 or Phase 3. Accepted raw path tokens can address
unmodeled process data; those cases remain an explicit residual below.

## Native identity and active boundary

Original active-retail `gamemd.exe` SHA256:
`1cdd1180e49024fbda8ad568caac2e86e856063ff67ab38f62b7d2c7bb84298c`,
x86 image base `00400000`. Read-only Ghidra body/assembly checks were against
`testProsjekt/gamemd.exe`. The executable is not distributed.

`582D70` is called by full hierarchy build `581F90` and incremental `584550`.
Full build is reached from InitZoneMap `56720D`, rebuild-all `581F6A`, and the
incremental fallback `584D79`. Full records run forward, active-byte gated;
incremental records run backward and require either signed endpoint inside the
current block. Both drain temporary buckets in ascending bucket/insertion order,
adding both edge directions without zero/equality rejection. The unused wrapper
`582D30` is not liveness evidence.

The helper classifies the center cell by IsBridge `486750` OR IsWoodBridge `486770`.
The high branch uses the retail tile-offset table and packed-word side additions.
Otherwise GetTube `484F20` validates the raw signed Cell+116 index against the
registry; it does not require Tunnel land, explicit source, a positive path length,
a group or a bridge deck. Both side-cell lookups occur before their null tests.
Either missing Tube suppresses all three pairs. Successful lookup registers the
center pair, first side pair and opposite side pair in that order.

Side paths start at the side **query coordinate**, not Tube.entry or declared exit
(assembly `582EEC`, `582F0A`). Original walker `429780` adds packed signed-word
offsets. Exact token8 performs a current-cell lookup: index-1 produces coordinate0;
otherwise it reads that Tube's declared exit. Ordinary tokens index `89F688+4*token`
with 32-bit address wrapping, with no direction mask. The parser `7283C0` accepts
CRT-atoi signed tokens until exact-1 or its bounded path buffer limit.

Temporary pairs preserve orientation and the first exact key's flags and position.
The bucket comes from the original zone IDs' low nibbles. Native sign-extends both
zone words before OR at `582FFD..583012`; a low word at or above8000 contaminates
the packed high word. The oracle records this behavior; it does not certify native
allocation safety at that zone cardinality or resolve the intentional 20k/30 scale
exception for this domain.

Node lookup is signed/wrapping linear indexing with stride
`Size.width+Size.height+1`, clamped to the native square allocation. The previous
[bridge record evidence](PHASE3_CELL_ITERATION_BRIDGE_RECORDS_20260910.md) proves
normal initialized padding retains zone0. The derived source Size now reaches
full and local hierarchy projection, and the raw GetZoneID row projection.

## Shared dummy and restore prerequisite

ReadTubesINI `72850A..728520` calls GetCell `5657A0` and writes the source ordinal's
low word through Cell+116 even when lookup returns the retained shared dummy.
The Rust raw receipt binder now publishes that field and the hierarchy/path readers
consume it. Nondefault process Tube state enters the deterministic world hash;
constructor-default-1 retains prior hashes. It remains process-owned and is not a
new serialized Scenario authority.

Native MouseLoad body `5BDF70` dispatches at `5BE150` vtable `7E1964+70` to `653F50`, through Resize
`565C10` and its constructor call at `5670F2`. Constructor `47BBF0` stores Tube-1
at `47BC48`. LoadContent `67E730` subsequently loads objects, calls `67F9C0`, then
rebuild-all hierarchy `581F50` at `67E8CD`. This corridor does not replay ReadTubesINI;
Tube OLE construction with zero entry does not stamp the dummy and CellLoad
`4839F0` reinstalls real cells. The fresh post-Resize field therefore precedes
hierarchy reads on this ordinary restore route.

App preparation previously rebound the live dummy before fallible cache and map
restoration, then reset it only at successful commit. Once hierarchy reads Tube,
that order both used stale state and exposed live writes on a rejected candidate.
Preparation now owns a detached post-Resize dummy. Successful commit copies its
terminal fields (including subsequent lookup coordinates) onto the retained live
identity and rebinds the prepared terrain. It does not reset the rebuilt graph's
inputs again. Base-reservation dummy reset remains part of preparation.

## Production route prerequisite

The existing explicit-Tube test in both flat and layered path wrappers disabled
hierarchy whenever any positive explicit path existed. Fresh original `42C900`
shows no Tube-specific allowHS term; removing that deferral is necessary to deliver
these graph edges. Original `42CB22..42CB3F` also rejects unequal native base-zone
labels before calling precheck. Raw equality preserves labels1/FFFF rather than
conflating them through the flattened invalid0 presentation. Same-zone failed
precheck retains ordinary A* fallback. Compatibility-only grids without raw rows
still use their available flattened equality.

The route gate now owns a separate live DWORD `GetZoneID` query. Original56D230
returnsFFFFFFFF when current Flags100 has no matching high record; raw rowFFFF
remains distinct. The prior cached16-bit getter conflated this case with the ground
row, which could newly admit an order after enabling the native hierarchy gate.
Activation is established by the earlier native producer `high_no_far` case:
Size8,8, tile106/subtile4 at(4,7), structural middle(5,7), no far tile, records[].
The Rust production record factory and actual flat/layered route regression retain
that admission witness rather than supplying a fabricated record failure.

Original56DA10 compares signed coordinate words, inclusive span bounds and signed
axis-distance tolerance, with first high-kind record and no activity filter. The
shared record matcher now preserves these signed comparisons. Original56D230's
inactive branch reloads the fixed cell and481810 steps from the **returned cell's**
stored coordinates, with packed word addition. The live route query follows that
walk and publishes dummy miss coordinates; its pure cache counterpart remains
side-effect free. A real nonstructural high/wood exit with land other than Rock
selects B, otherwise A. Constructor dummy tile65535 is outside the hash-bound
retail high sets. The query oracle covers signed fixed aliases, inactive east/south
exits, wood, Rock, ordinary exits, missing cells and retained dummy writes.

This does not certify older cached16-bit `get_zone_id_native` consumers in
base-defense and threat selection: missing-record DWORD and live walk/state
semantics there remain unresolved Phase3 query authority. Other allowHS predicates, hierarchy retry/edge updates and whole cell
A* remain outside this bounded helper delivery and must stay open where unproved.

## Reproducible comparisons and Rust checks

[Harness](../../tools/spatial_oracle/tube_hierarchy.py),
[vectors](../../tools/spatial_oracle/tube_hierarchy.json), and
[provenance](../../tools/spatial_oracle/tube_hierarchy.meta.json) execute original
582D70 through return, including original direction initializers,429780,589E20 and
58AF80. Fixtures supply cells, Tube registry, hierarchy words and adequate allocated
bucket storage; there are no code patches or substituted helper returns. They do
not execute complete flood fills, final edge allocation or whole path search.

Current corpus:65 helper cases, covering all center directions, all high tile
branches and wood, all hierarchy levels, null sides in both orders, retained dummy
Tube reads, zero/bent/marker8 paths, source/query/declared-exit distinctions, signed
aliases and packed wrap, zero/equal pairs, orientation, genuine bucket collision,
first flags and signed zone packing. A separate five-write original ReadTubes tail
transcript covers miss-to-real-to-alias-to-miss and ordinal truncation. This tail
comparison excludes parsing/allocation, whose successful receipt is supplied.
Six original42CB22 gate cases additionally compare unequal/equal raw labels with
allowHS on/off, stopping before reject return, precheck call setup or ordinary
continuation. They do not substitute a precheck result. Five standalone original429780 walks
compare exact endpoints for the new constant-slot domain, before zone clamping
can hide a wrong offset. Seventeen whole original56D230 query cases compare
DWORD results and terminal dummy coordinates, using supplied raw movement rows
and cell/record inputs; they do not certify flood-fill label construction.

Rust checks use the actual production helper, raw receipt binder, record factory,
full/local builders and precheck. App tests exercise detached candidate preparation,
a late MoveSound restoration rejection after map preparation, and successful
publication onto the retained identity. Route tests distinguish hierarchy corridor
selection from unrestricted A* with an irrelevant explicit Tube present.

Validation receipts (intermediate):

- `python -B -m tools.spatial_oracle.tube_hierarchy --write`: exit0,58 helper cases
  and five-write native tail regenerated (reviewed original image identity).
- `cargo test -p vera20k --lib native_tube_hierarchy_pairs_match_original_executable`:
  exit0,1 passed,8709 filtered,0.01s; this tested the prior57-case/source candidate.
- Expanded focusedv4:5 passed,1 failed,1 ignored,8707 filtered. Native58,
  native binding, generated full/local precheck and actual route, explicit-Tube
  flat/layered corridor and compatibility inequality checks passed. The restore
  fixture failed its pre-restoration record-count premise; correction now supplies
  real east/west Tunnel neighbors with diagonal hierarchy sides. This is a fixture
  correction, not evidence of a restored-state implementation failure.
- Focusedv5: exit0,7 passed,0 failed,1 ignored,8707 filtered,0.02s. The corrected
  direction1 restore fixture and raw invalid-label/goal bridge flag tests pass.
- Zone-searchv5: exit101,25 passed,3 failed. Three historical caller fixtures
  previously demanded hierarchy override unequal coarse labels. After making
  labels consistent, their newly added zero-count counterfactual revealed bridge
  deck exemption; the revised shared fixture uses a mandatory unmarked **ground**
  approach to a bridge detour, with a different ordinary ground shortcut. It checks
  no-count shortcut, zero-count rejection and actual-count bridge routing.
- Extended native63 generation: exit0, original initialized constant bytes agree
  with the saved process startup capture at FPCW0xE7F.
- Focusedv6: exit0,7 passed,1 ignored. Zone-searchv6: exit101,27 passed,1 failed;
  capture approached the changed bridge fixture from(5,1), adjacent to(5,0), rather
  than the old(4,0). The exact endpoint expectation was corrected while retaining
  the bridge-layer and adjacency assertions; rally/dock counterfactuals passed.
- Focusedv7: exit0,10 passed,1 ignored,8707 filtered,0.02s, including the prior
  ten-query corpus. Queued zone-searchv7 embedded the expanded seventeen-query
  corpus and exited101 (29 passed,1 failed). Its new inactive fixture inputs had
  not yet been supplied by the Rust adapter; this is a fixture-version mismatch,
  not a valid executable mismatch receipt. The independently established inactive
  walk defect is repaired and the final adapter supplies every exit cell and row.
- Native v8 `--check`: exit0,63 helper +5 binding +6 gate +5 constant walker +17
  DWORD query cases, stable before Rust validation. An initial invocation lacked
  the EXE environment and failed configuration; the corrected explicit-image run
  produced this receipt.
- V8 focused exit0:10 passed,1 ignored,8707 filtered,0.03s, including all17
  DWORD queries. Zone-search exit0:30 passed,8688 filtered,0.02s. Zone-build
  exit101:31 passed,1 failed,1 ignored. The old boundary fixture passed no source
  Size and expected rectangular clamp-to-last-cell; native uses a padded square.
  Corrected fixture supplies Size2,2: NW negative clamp selects node0, while
  SE(4,4) selects native padding0. Two matching original582D70 cases were added;
  v9 native generation exit0 (65 helper cases). This repair changes tests only.
- V9 focused exit0:10 passed,1 ignored,8707 filtered,0.03s; zone-build exit0:
  32 passed,1 ignored,8685 filtered. Final native --check exit0 reproduces all
  65+5+6+5+17 cases. The leaf candidate is checkpointed with these receipts.
- Independent caller-order review then confirmed42C900 performs initial source/
  destination cell lookups, raw queries, projections and finally conditional
  playfield probes. Existing wrappers perform live queries after projection and
  skip them when hierarchy is disabled. This newly material live-write ordering
  obligation is under repair; these leaf passes do not certify that corridor.
- Expanded focused, full library and Clippy validation are pending. No readiness
  claim is made from the earlier candidate's result.

## Explicit unresolved domain

Known ordinary slots0..7 match the direction table. Original49F2D0 and49F280
initialize adjacent slots8..12 to0; slot13 is zero BSS with no direct xrefs in the
examined image. Native comparisons cover9..13 and positive/negative address-wrap
aliases, including0x40000008 (ordinary zero read, not token8 teleport).

The [original-process startup capture tool](../../tools/spatial_oracle/tube_startup_capture.py)
and [captured evidence](../../tools/spatial_oracle/tube_startup_capture.json) bind
FPCW0xE7F at49F0E0/49F190 to initialized constant bytes. Hardware breakpoints stop
before application startup, and all4,063,232 executable-section bytes match the
original image. The helper oracle executes those original initializers and compares
known ranges to the process capture. Slots14/15 and effective slots-2/-1 (the
latter reached by non--1 address aliases) now use those exact signed-word offsets.
Direct xrefs identify initializer-only writes at49F0FE/49F19C; runtime indirect
mutation and arbitrary neighboring addresses have not been exhaustively excluded.

Other accepted tokens can read unmodeled mutable process data. The Rust walker returns an explicit unresolved-read diagnostic and contributes no
pairs for that record, preserving the prior absent-Tube-pair outcome without a new
panic or invented direction mask. This is **not parity** for that raw domain.
Likewise token8 with a non--1 invalid raw registry index remains explicit unresolved
process-memory behavior. Trigger: authored accepted path data reaching these reads.
Effect: missing hierarchy pairs may reject or redirect orders. Stock occurrence is
not established; earlier retail census found no explicit Tubes in385 payloads,
which does not make this live loader domain out of scope. These obligations keep
this mechanism and the phase rows open.

A retained structural dummy can make native's inactive query walk nonterminating.
Rust detects repeated actual coordinate/identity-kind states, emits a diagnostic
and returns unavailable; the current route caller then uses compatibility equality.
This can admit a route where native does not return. The explicit cycle behavior
is **unresolved**, not an endpoint or termination parity claim. No stock incidence
is established. The row stays open for this and the older cached query consumers.
