# Ordinary sustained sight and hostile gap shroud

Date: 2026-09-10. Phase3 row50 remains OPEN. Native executable SHA256:
`1cdd1180e49024fbda8ad568caac2e86e856063ff67ab38f62b7d2c7bb84298c`.

## Finding and delivery boundary

Current production `apply_gap_generators` unconditionally erased a hostile viewer's
knowledge even when an admitted live Techno still supplied sight. Original
`6FB170` preserves that sight. Using the existing VISIBLE bit as immunity would
introduce a second error: firing and PsychicReveal also set that bitmap, but their
native paths do not retain a reveal-counter contribution. Departure under an
already-active gap is deferred until the early Logic 120-frame sweep, including
when the generator disappears before that sweep.

The candidate stores knowledge per viewer. Fresh Techno and Psychic writers apply
the source house's direct-alliance relation before publishing to recipient planes;
display caching never ORs another viewer's derived knowledge. Only the actual
Techno owner receives the local SOURCE bit; admitted allied viewers receive the
separate EFFECTIVE sight bit. Generic transient mapping receives neither.

Stable generator identity/owner/geometry admission receipts distinguish a fresh
native gap write from repeat refresh/House materialization and survive save/load.
Existing hostile coverage lets transient mapping schedule pending conceal without
inventing sustained sight. Empty generator sets publish removal without canceling
pending. `advance_master_frame` consumes prior pending state using signed
pre-increment `binary_frame % 120`, after trigger work and before ore, Teams and
live object work. Snapshot142 rejects earlier representations. Historical hash
probes mask the four new bits and omit admission receipts. Raw legacy counters and
edge caches are not asserted equal to native by these bitmap tests.

Rust validation is pending at this evidence checkpoint; this paragraph describes
the unvalidated working candidate, not a passed production receipt.

## Native bodies, callers and ordinary retail activation

* Constructor47BBF0 initializes Cell+130=1, +134=0, +120/+121=-2.
  `MapCell4A9CA0` maps the cell and calls reduce487630 for finalmode0, or
  increase487690 for mode1, then real cache/redraw/notify helpers.
* Techno70AF50→5678E0→653830→4A9CA0 contributes sight with mode0;
  70B1D0 removes that contribution with mode1. A current contribution keeps
  Cell+130 negative. Selected hostile-gap block6FB2F7..6FB3C1 increments gap134
  but only clears Cell+12C bits18 when the resulting shroud counter is positive.
  IsShrouded586360 returns its boolean in AL, not the whole EAX register.
* Fire sites6FDF13..30 and6FF6FE..71B call5673A0 final0, reaching4876F0:
  unshroud without a counter decrement. Psychic6CD773 passes final0, then
  6CD79C final1: add then remove, leaving no sustained contribution. Earlier
  source wording that both Psychic arguments were identical was incorrect.
* Leaving the last sight contribution under a gap makes counter1 and sets
  Cell+140 pending20 while retaining alt18. Removal6FB5E1..6FB69E decrements
  gap134; ordinary MapIsClear=false neither restores prior knowledge nor cancels
  pending20. Its MapIsClear=true branch can restore knowledge separately.
* Logic55B29A..55B2C4 uses signed frame modulo120 and calls578100 at55B2AD.
  The complete first iterator pass consumes pending20 and clears alt18 and
  flags23; the second recalculates edges. This runs after trigger/tag work and
  before ore55B4D7, Team55B558, live objects55B608 and Houses55B698.
  MainTick55DC9E invokes Logic before the late frame increment55DE7E/81.
  4ACAC0/4ACDA0 instead belongs to optional ShroudGrow, not this ordinary owner.
* Hash-bound retail rulesmd.ini (`expandmd01.mix` winner), SHA256
  `3d341ef8a13a4b5ab24af2eef48ac94931ac2bb87d950fe3330a07e2d25672ef`,
  has GAGAP GapGenerator=yes, GapRadiusInCells=10, TechLevel7, Sight5,
  Powered=true and AIBuildThis=yes. General AllyReveal=yes, ShroudGrow=no and
  MPFogOfWar=no. Local retained extraction is `.local/ground-height-retail/extract`.
  The actual SpecialFlags reader admits conditional FogOfWar overrides; no
  TS-only or universal-unreachable claim is made for that option.

## Reproducible original execution

[`shroud_current_sight.py`](../../tools/spatial_oracle/shroud_current_sight.py)
executes original instructions using the shared identity-checked loader, without
code patches, substituted returns or unmapped-memory fallback. The JSON and
metadata sidecar retain every selected observation and the supplied-state limits.

```
$env:RA2_DIR='C:/Users/enok/Documents/Command and Conquer Red Alert II'
python -B -m tools.spatial_oracle.shroud_current_sight --write
python -B -m tools.spatial_oracle.shroud_current_sight --check
```

Both commands exited0 on2026-09-10. The read-only check reproduced all15 sequences
without writing files. Cases cover never-seen/current/past/overlapping sight,
entering an existing gap, ordinary and MapIsClear removal, fire-only/Psychic-only
mapping, departure119→120, return before120, removal preserving pending, and a
post120 departure waiting until240, a second fresh gap consuming pending
knowledge, and fire/Psychic mapping under an existing gap. The frame cases execute the original modulo
gate and complete578100 two-pass loop on a fully allocated Size12x12 diamond with
real Cell vtables. Sparse allocation would terminate the native iterator early
and is not used for that claim. Original49F2F0 initializes direction data.

The owner/power/radius admission branch is supplied, not emulated end to end.
The selected cell is an ordinary flat allocated cell with empty object lists;
full MapCell callbacks execute with supplied empty scenario/tactical state.
This compares selected IsShrouded and pending observations, not all native edge
cache shapes, complete reveal traversal, every map coordinate, or GPU pixels.

## Coherent viewer authority after review

Independent review rejected both re-exporting allied sight as a local source and
using a permanent gap-only display override. Both could make A-B-C alliances
transitive or suppress a fresh legitimate ally reveal. The final candidate has one
viewer knowledge authority: direct source-aware writes update it; presentation
only copies that viewer. There is no retained gap-knowledge override plane.

| Writer/consumer | Final production boundary |
| --- | --- |
| Ordinary Techno recompute and SetLocalSize membership promotion | `reveal_entity_vision` computes the existing source geometry once and publishes it to the source/direct allied viewers; SOURCE only on the source, EFFECTIVE on recipients |
| Psychic launch | Both original-mode counterparts publish a fresh transient event to source/direct allies; derived recipient knowledge is never a new source |
| Fire `damage_consequences` | Existing admitted local `reveal_radius` writer; no unsupported5673A0 allied broadcast |
| Whole-map/allocated-cell reveal and reset | Explicit recipient viewer writes, including SpySat and trigger callers; fresh writes invalidate presentation cache |
| Display/cache bootstrap and owner switch | Copies the already-resolved selected viewer plane; no cross-viewer OR |
| Snapshot continuation | Viewer bytes and stable gap admissions persist; presentation cache is discarded and rebuilt |

Original reveal→gap→leave→second gap immediately clears alt18. Admission receipts
and a fresh-write mask preserve this, while repeated materialization and restored
admissions remain idempotent. Replacement at identical coordinates has an actual
owning-world regression. This does not certify every generator operational gate.
Original gap→fire4876F0→119 stays open with pending20, then120 closes; Psychic
add/remove has the same selected-cell result. These additional original cases are
retained alongside actual Psychic-under-gap/House regressions.

A-B-C tests cover sustained and Psychic sources, never-gapped cells, mixed gapped
and ungapped cells, existing gaps, removal, repeated House/cache publication and
snapshot continuation. C cannot see A through B; a fresh B event reaches both of
its direct allies. Tactical CPU tests check bright versus dark cell output.

## Rust regression owners and remaining gates

Candidate tests in `vision/vision_tests.rs` replay every saved operation through
actual entity sight production and gap application; compare the knowledge
consumer, not raw counters. Allied sight, serialization, source-vs-transient
future conceal, cache freshness and hash authority have separate witnesses.
`world/gsi_04_18_tests.rs` uses the actual GAGAP collector, entity departure,
master-frame rung and GameSnapshot continuation; a real Psychic launch remains
concealable. `render/shroud_buffer.rs` tests the actual CPU cell-fill output from
bright current sight through pending departure to dark boundary output.

Focused Rust, historical-hash reconciliation, full `cargo test -p vera20k --lib`,
`cargo clippy -p vera20k --lib`, and final independent candidate review are pending.

## Explicit residuals

The existing CellVisibilityRuntime defaults/map-visible/clear-visible transitions
and forced cache values are not a native counter/cache model. They remain a
separate row50 obligation; the new pending owner must not consume those fields.
Full reveal radius/height/traversal and event scheduling, alliance-change history
transfer, optional AllyReveal=false, fogged-object memory,
optional FogOfWar/ShroudGrow and full rendered pixels remain outside this bounded
comparison. The GapGenerator collector does not yet prove every4555D0 operational
term (EMP, engineer, mission, HasPower); fixtures use an admitted operational
producer. Full firing reveal owner policy and Psychic raw counter publication are
also not closed. These are explicit residuals, not grounds for claiming the whole
row or phase complete from15 samples.
