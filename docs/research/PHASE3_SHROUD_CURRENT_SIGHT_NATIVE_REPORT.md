# Ordinary sight, hostile gaps and persisted knowledge

2026-09-10. Phase3 row50 remains OPEN. Original executable SHA256:
`1cdd1180e49024fbda8ad568caac2e86e856063ff67ab38f62b7d2c7bb84298c`.

## Finding and candidate delivery

Baseline `c7432d8c` erased knowledge under hostile gaps even while an admitted
Techno supplied sight. Reusing VISIBLE as immunity would wrongly protect fire and
Psychic Reveal. The coherent correction requires real source/generator events,
pending conceal, direct allied publication, persistence and native frame timing.

The candidate has one knowledge plane per viewer. Selected ordinary-cell state
owns signed shroud/gap counters, coupled open state, pending conceal and counted
local/allied sources. Legacy CellVisibilityRuntime remains a separate, explicitly
inexact edge-cache projection and does not drive these transitions. Admissions
keyed by `(stable_id, viewer)` retain their own source footprint. Changed/missing
sources release the stored footprint; unchanged refresh/House calls emit no new
reveal event. Writers immediately publish consistent knowledge/gap-covered output.

Direct source-to-viewer admission precedes writes. Display copies the selected
viewer; it never re-exports derived knowledge through A-B-C alliances. Fire keeps
its separately admitted local writer. Psychic performs mode0 then mode1 for the
source/direct allies. Stable generator receipts distinguish new writes from House
materialization; removal reads actual House MapIsClear and preserves pending.

The early master-frame rung consumes pending at signed `binary_frame %120`, after
triggers and before ore, Teams and objects. The high-flying FootAI refresh runs
before locomotor Process. Each viewer retains an independent entity-lifetime
65C/664 clock initialized from the constructor frame; nonallied viewers never
borrow another viewer's timer. A due event releases/re-admits only its viewer and
reloads15 even if the reveal leaves reject membership. Losing an admission does
not reset this clock. Snapshot142 persists state/footprints/gap receipts/clocks;
current hashes include them, historical probes omit them and the four new bits.
**The Rust revision is not yet validated.**

## Original bodies and active retail evidence

| Native owner | Established selected behavior |
| --- | --- |
| Constructor47BBF0 | Cell130=1,134=0,120/121=-2 |
| MapCell4A9CA0, reduce487630/increase487690 | Mode0 adds a contribution, mode1 releases it. Opening missing alt18 retains pending20; only an already-open reveal cancels pending |
| Fire4876F0 from5673A0 | Opens without decrementing; counter130>0 schedules pending even without a gap |
| Psychic6CD773/6CD79C through5678E0 | Final0 then final1; the older identical-arguments report was incorrect |
| Gap6FB170/removal6FB470 | Selected blocks6FB2F7..6FB3C1 and6FB5E1..6FB69E update counter/gap/open; ordinary removal preserves pending, MapIsClear is distinct |
| Logic55B29A..55B2C4 through578100 | Signed modulo120; first iterator pass consumes pending20 and clears alt18/flags23, second updates edges |
| Techno70AF50/70B1D0 | Latch250 suppresses unchanged ordinary calls; stores254XYZ/260radius. Release clears latch before radius check/call and retains stored geometry |
| Drive/UnitPerCellProcess739EC0 | Object9C coordinate write precedes release73AC5F then admit73AC74; older +504 attribution was wrong |
| FootAI4DA692..4DA7AA | Live slot80 moving, +54 IsHighFlying, direct local alliance, then timer due; release4DA6F7/admit4DA706 and unconditional due-branch reload15 |
| Foot constructor4D31E0 |664=0 at4D3349,65C=current frame at4D334F; no assertion about auxiliary660 |
| IsHighFlying5F6B90 | On-map74 and virtual height >=2*104, not a ground-unit or generic healthy gate |
| Fly4CCAC0/Jumpjet54D0D0/Rocket661F90 | Current speed ordered nonzero; state!=0,2; signed state3..5 respectively |

FootAI's event is necessary: reveal,gap,leave,gap,reveal,119,120 conceals despite
counter-1; an admitted due release/admit before120 cancels pending and stays open
with identical geometry. Ordinary global refresh must not manufacture that event.
Fly/Jumpjet/Rocket interface vtables are7E89F4/7ECD68/7F0B1C, slot80. Fly reads
current speed, not commanded speed. Rocket altitude is owned by rocket_state.
AircraftAI414BB0 reaches FootAI at414DA3 on the ordinary surviving unwarped path;
its earlier warp branch is not a Rocket exclusion.

The sweep precedes ore55B4D7, Team55B558, objects55B608 and Houses55B698.
MainTick55DC9E calls Logic before frame increment55DE7E/81. Optional
4ACAC0/4ACDA0 ShroudGrow is not the ordinary departure owner.

Hash-bound retail rulesmd.ini (`expandmd01.mix`), SHA256
`3d341ef8a13a4b5ab24af2eef48ac94931ac2bb87d950fe3330a07e2d25672ef`, contains
GAGAP GapGenerator=yes,radius10,TechLevel7,Sight5,Powered=yes,AIBuildThis=yes;
AllyReveal=yes,ShroudGrow=no,MPFogOfWar=no. JUMPJET has Sight8/height500;
ORCA/BEAG Sight8/Fly, HORNET Sight2/Fly, V3ROCKET Sight1/Rocket. Retained extraction:
`.local/ground-height-retail/extract/rulesmd.ini`. SpecialFlags admits conditional
FogOfWar overrides; no TS-only or universal-unreachable claim is made.

## Reproducible native comparison

[`shroud_current_sight.py`](../../tools/spatial_oracle/shroud_current_sight.py),
JSON and metadata use the shared identity-checked image loader. No code patches,
substituted returns or unmapped-memory fallbacks are used.

```powershell
$env:VERA20K_GAMEMD_EXE='C:/Users/enok/Documents/Command and Conquer Red Alert II/gamemd.exe'
python -B -m tools.spatial_oracle.shroud_current_sight --write
python -B -m tools.spatial_oracle.shroud_current_sight --check
```

Both exited0. The read-only check reproduced **19 sequences and9 timer cases**.
Sequences cover never/current/past/overlapping sight, entry/removal/MapIsClear,
fire/Psychic, delayed departure, return, second-gap/return, first-fire versus
first-Psychic history and the due same-footprint contrast. Counter/gap/pending and
AL-only IsShrouded observations are retained. Original4DA6C8 timer cases stop at
4DA6EF due or4DA7B0 not-due, including paused and signed-wrap cases. They start
after supplied admission gates and stop before either reveal call.

Frame cases execute original modulo logic and complete578100 two-pass iteration
on a fully allocated Size12x12 diamond with real Cell vtables. Sparse allocation
would terminate early. Original49F2F0 initializes direction data. MapCell's real
cache/notification leaves execute with supplied empty lists/scenario state; gap
blocks start after owner/power/radius admission. This does not emulate full
FootAI/GapGenerator admission, every coordinate, or all edge-cache outputs.

## Regression owners and validation status

`vision/vision_tests.rs` replays saved operations through actual source/gap/fire/
sweep owners and compares selected counters, pending and knowledge. Additional
checks distinguish unchanged refresh/House calls from force events, snapshot
continuation after second-gap/return, immediate publication, fire/Psychic history,
direct allied knowledge, cache freshness and hash authority.

`world/gsi_04_18_tests.rs` exercises GAGAP collection, actual Psychic launch,
master-frame120/240, replacement and GameSnapshot continuation. Its high-flying
source enters the actual live object pass and contrasts dueA/not-dueB/nonalliedC,
then legitimate directC admission with independent persisted clocks.
`render/shroud_buffer.rs` checks tactical CPU bright/dark output and nontransitive
viewer knowledge. These Rust tests are written but not yet executed.

Working `cargo check -p vera20k --lib` passed: exit0, 1m20s,90 warnings, retained
in `.local/shroud-current-sight-check-v1.log`. Focused tests, historical
hash reconciliation, full `cargo test -p vera20k --lib`,
`cargo clippy -p vera20k --lib` and final independent review remain pending.

## Explicit remaining scope

Legacy cache projection, full radius/height traversal and all arrival scheduling,
alliance-history transfer, optional AllyReveal, fogged-object memory,
FogOfWar/ShroudGrow and GPU output remain outside this comparison. The complete
GapGenerator4555D0 operational predicate (EMP/engineer/mission/HasPower) and full
fire owner policy remain open. Existing represented Jumpjet state5/6 limitations
remain; this does not certify floating-point Fly evolution, warp dispatch or
rocket flight. Selected ordinary counter algebra is compared, not arbitrary
partial alt flags. No row or phase closes from these bounded receipts.
