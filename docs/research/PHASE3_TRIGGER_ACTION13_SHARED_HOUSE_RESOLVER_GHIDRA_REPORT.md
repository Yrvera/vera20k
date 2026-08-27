# Phase 3 Trigger Action 13 Shared House Resolver — active-retail Ghidra report

Date: 2026-08-27

Binary: active retail Yuri's Revenge 1.001 `gamemd.exe`

Mode: bounded re-investigation, research only

Status: **VERIFIED GAP — not builder-ready in isolation**

## Verdict

Trigger action 13 (`Autocreate Begins`) has a small terminal mutation but not a
small input contract. Native resolves the already-materialized
`TActionClass+0x90` operand through the same shared House resolver used by
actions 3, 74, and 75. A null result returns failure without mutation; a
nonnull result writes only `HouseClass+0x1EF AutocreateAllowed = 1` and returns
success.

Rust does not yet carry two inputs needed to reproduce every native operand:

- the House context produced by selected trigger-event evaluations, used by
  special operand `0x2325`; and
- the dialog, sound, and theme registries used to materialize parameter types
  6, 7, and 8 into `TActionClass+0x90`.

The current `resolve_trigger_house` helper resolves a TriggerType owner for
actions 137/138. It is not the action-13 resolver. Reusing it, accepting only
the common numeric forms, or discarding action success would be an
approximation. The action remains open until the shared operand/context/result
contract is implemented by the trigger-runtime owner.

## Native action boundary

`TriggerAction__Execute @ 0x006DD8B0`, case 13 at
`0x006DEB18..0x006DEB54`:

1. read the signed operand at `TActionClass+0x90`;
2. call shared House resolver `FUN_006E45E0(trigger_context, operand)`;
3. if the resolver returns null, return false and write nothing;
4. otherwise write literal byte `1` to `HouseClass+0x1EF`;
5. return true.

The case does not write Production (`+0x1EE`), AITriggersActive (`+0x1F2`),
or AutoBaseBuilding (`+0x1F3`). The complete `+0x1EF` writer census contains
only the constructor clear, the House-update activation store, action 13, and
the House CRC read already documented in the House-latch reports.

Actions 3, 74, and 75 call the same resolver before writing their own single
House fields. This makes the resolver a shared trigger-runtime prerequisite,
not action-13-local policy.

## `TActionClass+0x90` materialization

`TActionClass__Read @ 0x006DD5B0` first clears `+0x90`. An action record is
read as:

`ActionID, ParamType, Param3, Param4, Param5, Param6, Param7, WaypointCode`.

The retained `+0x90` value is then produced as follows:

| ParamType | Native materialization |
|---:|---|
| 0 or 11 | `atoi(Param3)` |
| 5 or 9 | `atoi(WaypointCode)` when that token is present, overwriting the earlier value |
| 6 | dialog-name lookup through `FUN_00753250` |
| 7 | `VocClass__FindByName @ 0x007514D0` |
| 8 | `ThemeClass__From_Name @ 0x00721210` |
| other | retains the cleared zero |

`FUN_00753250` scans the dialog string registry and returns its index or `-1`.
The sound and theme helpers likewise scan their registered definitions. These
are not cosmetic parse details: their signed results become the exact operand
the shared House resolver consumes, even when the action kind normally expects
a House.

Rust `ActionEntry::params` preserves the seven tokens following the action ID,
but it materializes only the waypoint-oriented field required by actions
137/138. The simulation trigger layer does not receive the dialog, sound, or
theme registries, so it cannot yet reproduce all valid `+0x90` states.

## Shared House resolver

`FUN_006E45E0(context, operand)` has four ordered branches:

1. null `context` returns null;
2. operand `0x2325` returns `TriggerClass+0x2C` from the runtime context;
3. operand `-1` returns null;
4. operands `0x117B..0x1182` resolve the corresponding scenario start slot;
5. every other operand calls
   `HouseClass__Find_By_Country_Index @ 0x00502D30`.

The final lookup walks the native global House array forward and returns the
first House whose HouseType country/self index matches the signed operand.
This is not a lookup by TriggerType owner.

### Start-player tokens

`FUN_00510F60` recognizes exactly `0x117B..0x1182` (`<Player @ A>` through
`<Player @ H>`). `FUN_00510ED0` subtracts `0x117B`, calls `FUN_0068C030`, and
uses the returned House index with native array bounds. `FUN_0068C030` reads
`ScenarioClass+0x1180[index]` for indexes 0 through 15 and returns `-1`
outside that range.

`ScenarioSession::start_slot_houses` is the existing Rust authority for this
scenario table and is already serialized and hashed. It is a valid input to a
future shared resolver; it does not by itself close the other branches.

### Dynamic event House context

Special operand `0x2325` is not the trigger owner. `FUN_00726910(context)`
returns `TriggerClass+0x2C`. The TriggerClass constructor clears that field,
and `TriggerActionEntry__EvaluateConditions @ 0x007264C0` writes it from the
successful event condition's `TEventClass+0x54` result.

The currently supported Rust event subset (elapsed time, variables, and
TechType exists/not) has no modeled event-result House. Live checks of native
events 60/61 confirm those type-count branches do not produce this context.
Therefore `0x2325` resolves null for today's supported event set, but the
field and propagation seam must exist before broader trigger-event support can
claim the shared resolver is complete.

## Action result contract

Native action execution returns a boolean. Action 13 returns false on null
House resolution and true after the store. Rust `trigger_runtime::apply_action`
currently returns `()`, so even a correct field mutation would lose the native
failure/success boundary consumed by trigger action execution.

The shared prerequisite must therefore include result propagation for action
execution, not merely a House lookup helper.

## Retail activation and exclusions

The enumerated installed retail corpus contains zero action-13 chunks across
310 mounted map payloads. Action 13 is still an active compiled/custom-map
surface; its absence from shipped maps is an evidence-backed ordinary-retail
exclusion, not permission to assign approximate semantics.

The House-update path remains the stock-active writer of
AutocreateAllowed. That mechanism is separately implemented and critic-passed.

## Current Rust mismatch

- `trigger_runtime::resolve_trigger_house` resolves a TriggerType owner through
  Rules country data for actions 137/138; native action 13 resolves the action
  operand described above.
- `ActionEntry` has no exact generic `TActionClass+0x90` projection.
- trigger execution carries no dynamic event-produced House context.
- trigger action application does not propagate native boolean success.
- the sim trigger boundary has no dialog/sound/theme registry authority for
  parameter types 6/7/8.

## Implementation gate

Action 13 may be handed to a builder only after one coherent trigger-runtime
design supplies:

1. exact `+0x90` materialization for every parameter type, with the required
   registry inputs or an evidence-backed exclusion at the ingestion boundary;
2. the ordered shared House resolver, including start-slot and country-index
   lookup;
3. a runtime event-House context seam for `0x2325`;
4. boolean action-result propagation; and
5. focused tests proving null/failure and nonnull/single-field mutation for
   action 13 while rechecking actions 3, 74, and 75 against the same resolver.

Until all five exist, the Phase 3 ownership row remains open for this compiled
surface. No numeric-only compatibility shortcut is admissible under the
phase-wide zero-residual completion bar.
