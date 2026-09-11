# Phase 3: Building gap operational publication

The original updates a GAGAP's deposited gap at its Building turn when its
operational state changes. Rust at baseline `417dfe4d` instead reclassifies all
generators after the object pass and again during House reconciliation. These
orders can produce different shroud knowledge at the next 120-frame sweep.
The candidate moves publication to that Building owner and retains admission
across House reconciliation and restore. Production validation is in progress;
this is not row50 closure.

## Evidence and active caller

The original executable is `gamemd.exe`, SHA256
`1cdd1180e49024fbda8ad568caac2e86e856063ff67ab38f62b7d2c7bb84298c`.
[The retained comparison](../../tools/spatial_oracle/gap_admission.py) executes
18 complete `4555D0` predicate cases with original Mission `5B3040` and House
power-ratio `4FCE30` callees, and two ordered shroud sequences using the original
leaves and complete periodic sweep. [Outputs](../../tools/spatial_oracle/gap_admission.json)
and [provenance](../../tools/spatial_oracle/gap_admission.meta.json) are retained.
No instructions, return values, or callees are replaced.

Fresh retail `rulesmd.ini` SHA256
`3d341ef8a13a4b5ab24af2eef48ac94931ac2bb87d950fe3330a07e2d25672ef`
declares GAGAP `GapGenerator=yes`, `Powered=true`, `Power=-100`, radius10,
SuperGapRadius10, Strength600, Sight5, Capturable=false and AIBuildThis=yes.
This binds ordinary power-dependent incidence; optional predicate cases do not
establish retail producers for every raw field.

Building constructor `43B9FA` installs vtable `7E3EBC`. Slot+5C at `7E3F18`
is `43FB20`, reached by the live Logic object call at `55B610`. Its initial
operational result is compared with retained byte+6C8 at `43FB59/5F`.
Only a changed result calls `4549B0` at `43FBEA`, then stores the result at
`43FBEF`. Unchanged visits emit no gap write.

`4549B0` rechecks operational+350. Type+CD1 and inactive+269 call gap-add+414
at `454A59`; rejection and active+269 call gap-remove+418 at `454BA9`.
The other direct caller, `6E0B60` via `6E0C7A`, processes an ownership action;
the older report's generic power-plant destruction interpretation is unsupported.
House `4F8440` calls aggregate power assessment `508C30`; its Building power
notification `454CE0` is an empty return. This does not publish a gap removal
before the next Building visit.

## Operational predicate

`4555D0` reads Type at Building+520 and House at+21C. It rejects when:

- byte+660 is clear and signed+67C is below2;
- signed+504 is positive, or health+6C is exactly zero;
- Type+1573 is set, signed Type+EE4 is positive, House power ratio is below1,
  and signed+67C is below2;
- the Type+1574 House timer/+577B gate is not satisfied;
- Type+1552 is set without Building+6CC;
- effective Mission is Construction12 or Selling13.

The actual Mission virtual+184 is `5B3040`: current+AC, falling back to queued+B4
only when current is -1. A queued Selling mission does not reject an active
Guard current mission. `4FCE30` reads signed output+53A4/drain+53A8: output at
least drain or zero drain returns1; otherwise it returns the original ratio.
The comparison supplies normal stock power inputs and executes this callee.

## Observable ordering witness

The original sequence `reveal, gap, leave, gap, reveal, remove, frame120`
reaches one ordinary sight receipt, one hostile gap, counter-1, closed knowledge,
and no pending conceal. All state comes from actual original writes.
The source is parked so no intervening moving high-flight timer event refreshes
it. A due release/admit is admitted at frame239. Then:

| Event order before frame240 | Final counter/gap | Original IsShrouded AL |
|---|---|---|
| Source release/admit, then gap removal | -1 / 0 | 1 |
| Gap removal, then source release/admit | -1 / 0 | 0 |

The native witness composes supplied object order, not a full Logic/power-system
execution. The production regression damages the generator owner's power
plant before House assessment at frame238. The baseline House collector removes
the gap then; the original retains it until the Building turn at239. Source-first
registration consequently chooses different rows. Both registration orders and
snapshot continuation are exercised by the regression, including the actual
high-flight refresh producer and the CPU shroud-fill consumer.

## Retained lifecycle and integration

Techno constructor `6F2B40` initializes admission+269 and cached radius+26C to
zero; Building constructor initializes previous operational sample+6C8 to zero
and HasPower+660 to one. Add `6FB170` checks admission, rechecks `4555D0`, caches
the signed Type radius when needed, and sets admission before the cell loop.
Remove `6FB470` clears admission before its loop, retains radius and does not
check current power. Admission exists even for a friendly viewer with no hostile
cell receipts. The Rust entity therefore retains a shared operational sample and
per-viewer admission/radius, independently of those cell receipts.

Building save `454190` reaches AbstractSave `410320`; virtual size leaf
`459E70` returns `0x720`. Load `453E20` reaches `410380` for the same body.
Restore constructors `43B680 → 6F4300 → 65A7E0 → 5F3B50 → 4101C0` neither
reset these fields nor replay the gap. Snapshot schema143 preserves and hashes
the retained Rust state. House and cache reconciliation only project it.

Techno Limbo `6F6AC0` releases ordinary sight at `6F6B16`, removes the gap at
`6F6B6A`, then reaches Object Limbo. Ownership `448260` instead removes the
gap before releasing sight and changing owner; ordinary new-owner reveal
`701875` precedes the new gap admission. These existing Rust lifecycle
chokepoints now publish in that order. SpySat's selected viewer bracket uses
live gap candidates and rechecks current operational state, including candidates
with no old receipt; other viewers and the Building operational sample remain
unchanged. Repeated House materialization is not an admission event.

Complete add/remove leaves also clear local House+240 (`6FB43F` / `6FB71C`),
including friendly admissions with no hostile receipt. This is distinct from
House+577A's SpySat latch. The selected event owner invalidates the mapping
latch; successful post-bulk re-admissions clear it again after `577D90` sets it.
Rejected re-admissions leave the new mapping latch set. The production regression
uses the launch shroud option, a hostile gap, power loss and a new SpySat to
exercise this existing consumer, plus friendly re-admission.

Fresh Unlimbo is not an unconditional gap add: the discovery path reaches
`445F80 → 446AA3` behind its caller gates. Construction `449A50` invokes that
path before queuing Guard, so the operational recheck still rejects it. Current
Rust placement represents construction with `BuildingUp` without necessarily
publishing Mission12; the selected gate respects that existing owner until the
build completes, then the next Building visit admits the gap.

The ordinary death path is synchronous: raw Building+4EC resolves `4415F0`,
and ReceiveDamage's postlude `44266B..4426A7` calls UnInit when its ordinary
non-Selling, `Explodes=no` timer has positive remaining time. Stock GAGAP uses
that path. No eight-frame delayed-death behavior was introduced. Selling or
`Explodes=yes` deferred destruction remains outside this comparison.

## Delivery boundary and limits

The existing shroud counter and per-viewer gap receipt owner remain applicable;
see [the current-sight report](PHASE3_SHROUD_CURRENT_SIGHT_NATIVE_REPORT.md).
The selected delivery covers operational edges, ownership and Limbo ordering,
SpySat rechecks, and persistence. House/cache refresh must not substitute a new
power classification for a Building event.

General EMP/NeedsEngineer/PoweredSpecial/+67C producers, mobile gap geometry,
campaign/discovery gates and optional fog records remain separate unproved
domains. The stock static geometry comparison does not prove current-coordinate
removal for moving/warped gaps. Friendly fog's existing boolean projection does
not establish native Cell+13C counter equivalence. No entire Building policy or
power system equivalence is claimed.

## Validation receipts

- 2026-09-10: native `gap_admission --write`, actual exit0, 1.44s.
- Native `gap_admission --check`, actual exit0, 1.44s; 18 predicate cases and
  two ordered sequences matched; no files written.
- Evidence checkpoint `e4cbf9e1` contained no Rust behavior change. A subsequent
  native replay corrected the fixture's named Guard value to raw5; `--write`
  and `--check` both exited0 (2.69s combined), with the same bounded cases.
- Working candidate `cargo check -p vera20k`: exit0, 30.53s, 88 warnings;
  `.local/gap-admission-check-v1.log`. This predates the final construction and
  ownership-sight fixes and is not final validation.
- Focused-v1 exited101 on one test-only private-method access error. The placement
  fixture now drives public `advance_tick` without broadening production access.
- Focused-v2 compiled in3m32s and exited101: four tests passed and two fixtures
  failed before their intended observations. The snapshot fixture omitted the
  production restore coordinator needed to reconstruct occupancy; the SpySat
  fixture indexed an absent empty receipt set. Both were corrected without
  changing their behavioral expectations. Logs are retained as
  `.local/gap-admission-focused-v1.log` and `-v2.log`.

- Focused-v3 passed all seven cases (exit0), including actual placement,
  power/object ordering, owner/death, SpySat and snapshot continuation.
- The affected vision suite passed69/69 and owner-change checks passed4/4.
  The initial GSI run passed20 and failed3: two fixtures supplied alliances only
  to the fog projection instead of the House authority, and one asserted that
  House240 remained set after successful gap readmission. The fixtures now use
  the production alliance authority and native6FB43F latch expectation; all
  existing cell-visibility and native ordered-comparison assertions remain.
- 2026-09-11: a fresh independent critic confirmed those corrections, reviewed
  the bounded production mechanism and independently replayed all18+2 native
  cases successfully. Final main integration and full checks are still pending.
