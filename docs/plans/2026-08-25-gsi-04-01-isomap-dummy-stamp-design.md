# GSI-04.01 IsoMapPack dummy-coordinate stamping design

## Status

Approved for the narrow G2 implementation slice. GSI-04.01 remains open after
this slice through G3-G4, C1-C6, and the explicit cross-row/excluded residuals.

## Requirement and evidence

Active YR `IsoMapPack5 decoder @ 0x0056BAC0`, called unconditionally by the
ordinary map loader, consumes each non-sentinel 11-byte record in stream order.
It computes `cell_idx = (i32)(i16)X + (u32)Y * 512`. An index outside
`[0,0x3FFFF]` or an in-range null pointer writes the record's exact packed
coordinate header to the fixed dummy at `0x00ABDC50 + 0x24`, consumes the seven
payload bytes, and skips all tile/subtile/level/ice writes. A valid allocated
record mutates only its real CellClass. Therefore the last invalid/null record
is the decoder's final dummy-coordinate writer.

Rust consumes each complete record and already suppresses payload for records
that do not resolve to a materialized Size-diamond cell. It currently drops
out-of-fixed-range records during parsing and silently skips in-range null-slot
records during materialization, so neither class reaches `SharedCellDummy`.

## Player-experience and determinism ledger

- Trigger: malformed/editor/custom IsoMapPack5 containing an out-of-fixed-range
  record or an in-range coordinate whose fixed table slot is null for the
  current Size diamond.
- Required state change: stamp the exact raw packed `(X,Y)` coordinate for every
  such record in original stream order; the last miss remains live.
- Payload: all seven bytes remain consumed but do not change dummy tile,
  subtile, level, ice, slope, bridge, or reservation fields.
- Aliasing: signed X plus unsigned Y determines whether the fixed-table lookup
  is in range and its canonical slot. If an aliased slot is real, no dummy stamp
  occurs; if it is null, the raw request—not its canonical alias—is stamped.
- Ordering: Fill happens first, then IsoMap records, then later overlay/bridge
  load writers. The stamp must occur inside terrain materialization before
  those later writers. Interleaved valid records do not touch the dummy, so a
  miss-only trace replay preserves the exact final dummy state.
- Parsing transaction: `MapFile` parsing remains pure and does not borrow or
  mutate the process dummy. It only retains lookup evidence for the later
  native-shaped materializer.
- No RNG, tick, scheduler, identity, reservation, or real-cell behavior changes.

## Options considered

### 1. Retain a raw lookup trace and replay only misses during materialization — chosen

Have the IsoMap parser retain one compact lookup record per non-sentinel input:
raw signed X, raw unsigned Y, and the optional canonical fixed-table coordinate.
Keep `MapFile::cells` as the existing payload-bearing in-range records. During
production Size-diamond materialization, use the lookup trace plus the already
built allocation index to stamp raw coordinates for out-of-range or null-slot
records in stream order. Then apply payloads through the existing real-cell
path.

This preserves ordering and raw/canonical distinction without exposing the
process owner to parsing or perturbing established `MapCell` consumers.

### 2. Store only the last out-of-range coordinate

Rejected. The final native miss may instead be a later fixed-range null slot,
or vice versa. Two independent “last” values cannot recover cross-class stream
order.

### 3. Keep every raw record directly in `MapFile::cells`

Rejected for this slice. `MapCell` currently means a canonical in-range record
to many loaders, diagnostics, test builders, and RMG paths. Widening it to raw
invalid records would force unrelated consumers to rediscover decoder bounds
and could accidentally apply payload.

### 4. Stamp the dummy during `MapFile::from_bytes`

Rejected. Parsing has no process-global MapClass owner and is reused for
previews, scans, and fallible candidates. Mutating live game state there would
violate ownership and transaction boundaries.

## Implementation shape

1. Add a crate-private `IsoMapPackLookup` record and parsed-pack result holding
   `cells` plus ordered `lookups`.
2. Change IsoMapPack parsing to retain every non-sentinel lookup while keeping
   the existing payload cell list limited to fixed-range canonical entries.
3. Add the lookup trace to `MapFile`; synthetic/RMG/manual constructors set an
   empty trace.
4. During production `materialize_map_load_cells`, replay trace misses against
   the Size-diamond allocation index and stamp the supplied
   `SharedCellDummy`. For synthetic maps without a trace, preserve the existing
   manual-`MapCell` test semantics by treating an explicit cell absent from the
   allocation index as a canonical miss.
5. Keep payload application unchanged and real-only. Do not model IceGrowth or
   broaden tile-index fixup in this slice.

## Acceptance tests

- Parser test: prove raw signed-X/unsigned-Y headers and canonical fixed-table
  aliases are retained for out-of-range, aliased-in-range, and upper-bound
  records while payload cells remain filtered as before.
- Materializer test: interleave out-of-range, real, and fixed-range-null
  records; prove only the real payload applies, every miss stamps in stream
  order, the final coordinate is the last miss's raw request, and seeded
  level/slope survive.
- Valid-map regression: all allocated explicit records leave the dummy
  coordinate unchanged and retain current Fill-before-explicit behavior.
- Focused `--lib` validation only for this slice; do not run the phase-wide full
  suite.

## Adversarial self-review and approval

The easy but incorrect design is a single `last_rejected_coord` captured by the
parser. It cannot order parser-rejected records against later materializer-null
records. The chosen trace is the minimum data that resolves that ambiguity.

The trace duplicates coordinate metadata, but not payload authority. It is
crate-private, load-only, bounded by the existing record count, and keeps
`MapCell` stable. Replaying miss stamps before applying valid payloads is
state-equivalent because valid payloads never touch the dummy; the replay still
occurs after Fill and before later overlay/bridge load writers.

Approved because it preserves the native raw request, fixed-table aliasing,
null-slot test, and stream last-writer result without coupling parsing to
process state or expanding adjacent decoder gaps.
