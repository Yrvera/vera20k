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

This does not certify the existing GetZoneID bridge-redirection implementation:
missing structural records and signed aliases have separate unproved return/state
semantics. Other allowHS predicates, hierarchy retry/edge updates and whole cell
A* remain outside this bounded helper delivery and must stay open where unproved.

## Reproducible comparisons and Rust checks

[Harness](../../tools/spatial_oracle/tube_hierarchy.py),
[vectors](../../tools/spatial_oracle/tube_hierarchy.json), and
[provenance](../../tools/spatial_oracle/tube_hierarchy.meta.json) execute original
582D70 through return, including original direction initializers,429780,589E20 and
58AF80. Fixtures supply cells, Tube registry, hierarchy words and adequate allocated
bucket storage; there are no code patches or substituted helper returns. They do
not execute complete flood fills, final edge allocation or whole path search.

Current corpus:58 helper cases, covering all center directions, all high tile
branches and wood, all hierarchy levels, null sides in both orders, retained dummy
Tube reads, zero/bent/marker8 paths, source/query/declared-exit distinctions, signed
aliases and packed wrap, zero/equal pairs, orientation, genuine bucket collision,
first flags and signed zone packing. A separate five-write original ReadTubes tail
transcript covers miss-to-real-to-alias-to-miss and ordinal truncation. This tail
comparison excludes parsing/allocation, whose successful receipt is supplied.

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
- Expanded focused, full library and Clippy validation are pending. No readiness
  claim is made from the earlier candidate's result.

## Explicit unresolved domain

Known ordinary slots0..7 match the direction table. Original49F2D0 and49F280
initialize adjacent slots8..12 to0; slot13 is zero BSS with no direct xrefs in the
examined image. Native comparisons cover9..13 and positive/negative address-wrap
aliases, including0x40000008 (ordinary zero read, not token8 teleport).

Other accepted tokens can read neighboring constants or mutable process data;
slots14 and negative neighbors have demonstrated accessible original instruction
paths, but their startup floating-control/data provenance is not yet established.
The Rust walker returns an explicit unresolved-read diagnostic and contributes no
pairs for that record, preserving the prior absent-Tube-pair outcome without a new
panic or invented direction mask. This is **not parity** for that raw domain.
Likewise token8 with a non--1 invalid raw registry index remains explicit unresolved
process-memory behavior. Trigger: authored accepted path data reaching these reads.
Effect: missing hierarchy pairs may reject or redirect orders. Stock occurrence is
not established; earlier retail census found no explicit Tubes in385 payloads,
which does not make this live loader domain out of scope. These obligations keep
this mechanism and the phase rows open.
