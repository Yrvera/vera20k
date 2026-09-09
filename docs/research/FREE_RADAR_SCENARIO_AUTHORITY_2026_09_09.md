# Map-owned FreeRadar

The captured retail maps set `[Basic] FreeRadar=yes`. VERA previously omitted
that field and queried only radar buildings and power, leaving those scenes'
minimap offline. This increment carries the map flag into persistent scenario
authority and the existing availability consumer.

## Native behavior established

Pinned gamemd.exe SHA256:
`1cdd1180e49024fbda8ad568caac2e86e856063ff67ab38f62b7d2c7bb84298c`.
Scenario reset `0068383C` writes zero to Scenario+34A4. Basic parsing
`0068A5E3..0068A61A` calls `005295F0` and stores the result there. Missing or
unrecognized values preserve the supplied prior byte; fresh scenario loading
supplies the reset false value. Original Scenario CRC includes it at
`0068BC16`.

House update reaches `00508DF0` at `004F8505` through its radar-dirty and
caller-global gates. The body clears House+5779, returns for a nonlocal House,
and checks the distinct spy-radar timer at House+2B0/+2B8 before FreeRadar.
When set, FreeRadar bypasses aggregate power and provider scanning. The result
is compared through `00656DE0`; changes reach `00656DF0`, which writes the
radar availability byte and activates the radar mode.

The [portable original runner](../../tools/free_radar_oracle/oracle.py) repeats
the [preserved fixture](../../tools/free_radar_oracle/fixtures/native-free-radar.json):
28 token/prior cases, reset, 70 empty-provider decisions and a complete
nonlocal early return. Fixture SHA256 is
`f434e555f94f9f68cbcd8247a000fdf93bf661df79611152d251045d8930ef4d`.
The local native evidence was independently replayed by the critic. The
portable runner also reproduced every result using the shared original-image
loader and checked stop boundaries. Prepared cached INI/House structures do
not establish a full INI file load, provider scan, UI transition or capture.

## Rust ownership and boundaries

`map/basic.rs` retains `Option<bool>` from the existing native first-character
boolean parser. Both app descriptors in `app/loading/init.rs` and the
headless descriptor in `headless_scenario.rs` resolve fresh missing values to
false. `ScenarioDescriptor` transfers the value to one `ScenarioSession` bool.
That field persists in snapshots and participates in the scenario identity
hash through a true-only tag, preserving historical false-map hash streams.
Snapshot schema 139 rejects older positional bincode records before decoding;
adding `serde(default)` alone would misread their following fields.

`sim/radar.rs::has_radar_for_owner` retains owner resolution and checks the
session flag before the existing power/provider helper. The presentation
consumer already queries the preferred local owner and drives radar animation
from the returned availability. No renderer flag or alternate UI authority
was added. Sidebar theme/geometry/animation changes belong to a separate
increment.

Native recording headers do not contain FreeRadar. The recording callback
must reconstruct map fields through ordinary loading. Current source has
only test callers of `NativeReplayStream::initialize` and
`ScenarioDescriptor::from_native_replay_header`; app native-recording loading
is not yet wired. The new regression exercises that callback API with actual
MapFile parsing and the shared GPU-free scenario-construction funnel. It does
not claim an existing production replay loader that the repository lacks.
The existing replay runner consumes an already-constructed Simulation.

The native spy-radar-blackout state and its infiltration writer are absent
from Rust. Existing `power_blackout_remaining` belongs to spy power/ForceShield
and must not be used as radar blackout. This increment covers ordinary
inactive radar-blackout states; an eventual spy radar outage remains DRIFT.
The existing ordinary provider aggregation also has unresolved native
first-provider/mission/lifecycle predicates. Nonlocal native early-return UI
semantics are recorded evidence, not a new arbitrary-owner query contract.

## Validation

The native portable replay passed all recorded cases. Rust checks cover the
28 optional parser/default cases, the 40 inactive-radar-blackout decisions,
low-power and power-blackout bypass, unknown owner behavior, true-to-missing
map reload, recording callback construction, snapshot persistence and version
rejection, and state-hash discrimination. The other 30 native blackout cases
are deliberately retained without a Rust equivalence claim.

The combined source includes approved sidebar commit
`ffc32972d7a011f31c2be5bb97781b0c03daf9ef` and its TREE prerequisite. Final
checks on that source passed:

- Focused FreeRadar: 6 passed, 0 failed.
- Full library: `test result: ok. 8571 passed; 0 failed; 111 ignored; 0 measured; 0 filtered out; finished in 24.94s`.
- Actual sidebar GPU: 3 passed; TREE snapshot/edit GPU: 6 passed; Ground
  replay integration GPU: 1 passed; shared depth GPU: 13 passed. Each group
  reported zero failures. These are correctness checks, without a timing claim.
- Retail sidebar source/history checks: 2 passed, 0 failed.
- `cargo clippy -p vera20k --lib`: exit 0, with 1,142 warnings.
- System Map: 0 errors across 3 files.

The first full run found a pre-existing version guard still asserting 138.
Updating its name, comment and expected value to the required schema 139 was
the only test change before the passing final full run; runtime and fixture
bytes remained unchanged. The independent critic cleared the combined source
authority and this correction. The final report update changes no Rust.

The combined sidebar/FreeRadar application capture must still demonstrate an
online minimap in the same retail scene; CPU availability tests do not prove
rendered minimap pixels or general radar equivalence. The inherited TREE
application overlap and dense-scene performance gates also remain open; this
increment does not supersede those boundaries.
