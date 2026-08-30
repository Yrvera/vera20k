# Phase 3 Unit Deploy House Flags — active-retail Ghidra report

Date: 2026-08-27

Binary: active retail Yuri's Revenge 1.001 `gamemd.exe` in live read-only Ghidra

Mode: `re-investigate`, exhaustive slice, research only

Status: **VERIFIED — implementation-ready**

## Scope and verdict

This report closes only the three open House byte writes after successful AI ConstructionYard anchoring in `UnitClass__Deploy @ 0x007393C0`:

- `HouseClass+0x1EE` — **Production** (AI production has begun)
- `HouseClass+0x1F2` — **AITriggersActive**
- `HouseClass+0x1F3` — **AutoBaseBuilding**

The offsets are semantic names corroborated by the YRpp `HouseClass` layout; all behavior below is proved independently from the active binary.

**Verdict:** these are **three independent persistent latches**, not one packed flag or one mechanism. Successful nonhuman, nonzero-mode ConstructionYard deployment deliberately co-writes all three to literal `1` as one ordered transaction, after base anchoring and before dispersal. Each latch has its own writers, readers, reset behavior, and checksum treatment. Rust currently implements none of the three. It must model them independently, while one deploy helper may perform the three native-ordered writes.

This report extends, and does not re-cover, `PHASE3_HOUSECLASS_ORDINARY_BASE_PLACEMENT_005060B0_GHIDRA_REPORT.md`. `FUN_0050C920` is bounded only as the immediately following dispersal call; its internals are intentionally outside scope.

## Evidence method

- Full instruction-operand census for displacements `0x1EE`, `0x1F2`, and `0x1F3` in active `gamemd.exe`, with constant/immediate false positives removed.
- The research-index brief was queried on `UnitClass__Deploy`, all three offsets, the base-placement parent report, and AITrigger context. It correctly identified this post-anchor gap but supplied no substitute for live-binary verification.
- Decompilation plus instruction listings for every surviving writer and reader, constructor, save/load, raw House CRC, deployment, takeover, House update, trigger action, team script, and relevant unit mission consumers.
- Call-order checks around `FUN_00505180`, `HouseClass__Computer_Paranoid`, and `FUN_0050C920`.
- Direct scans of retail `rules.ini`, `rulesmd.ini`, `ai.ini`, `aimd.ini`, loose maps, `MAPS01.MIX`, `MAPS02.MIX`, `mapsmd03.mix`, `expandmd01.mix`, `MULTI.MIX`, and `multimd.mix`.
- Direct Rust scan of House state, deploy, AI, trigger runtime, team-script VM, RuleSet, snapshot, and world hash.
- Existing research was used only as a lead. The binary, retail data, and current Rust tree are the authorities.

## Open Questions Log

| Question | Resolution |
|---|---|
| Are the three bytes one flag group? | **RESOLVED:** no. Three independent booleans with different readers/writers/reset and CRC lifecycles; deploy merely co-writes them. |
| Exact deploy values, order, and gates? | **RESOLVED:** `1EE=1`, then `1F2=1`, then `1F3=1`; nonhuman owner, deployed target is `ConstructionYard`, and `g_GameMode != 0`. |
| Does anchoring or `FUN_00505180` write them? | **RESOLVED:** no. All anchoring/Recalc work completes first; the three explicit stores follow. |
| Does `Computer_Paranoid` write them? | **RESOLVED:** no. It is a global diplomacy pass called inside `FUN_00505180` under its own gates. |
| Can `FUN_0050C920` undo or condition them? | **RESOLVED:** no. It is called after all stores, has no target-byte reads/writes in the full census, and returns too late to gate them. |
| Constructor/defaults? | **RESOLVED:** all three are initialized to zero. |
| Every active writer and reader? | **RESOLVED:** complete instruction census below, including stock-data activation classification. |
| Save/load/checksum behavior? | **RESOLVED:** all three raw-persist; `1EE` and `1F2` are directly House-CRC-fed, `1F3` is deliberately omitted from the direct House CRC sequence. |
| Are trigger action 30 and team opcode 29 stock-active? | **RESOLVED:** compiled/custom-map surfaces but zero hits in the enumerated installed retail corpus; excluded from the ordinary stock row, retained as compatibility evidence. |
| Current Rust equivalent under another name? | **RESOLVED:** none. `AiPlayerState::mcv_deployed` is not equivalent. |

No question needed an `UNKNOWN`, `UNCHECKED`, or approximate answer for this slice.

## Exact deployment transaction

The relevant active block is `UnitClass__Deploy 0x00739855..0x00739926`. It executes only after the unit has successfully become its deployed Building.

### Gates

All three stores share the same outer gate:

1. `HouseClass__IsControlledByHuman @ 0x0050B730` returns zero for the owner.
2. The spawned `BuildingTypeClass+0x16B9` `ConstructionYard` byte is nonzero.
3. global `g_GameMode @ 0x00A8B238` is nonzero.

Failure of any gate skips anchoring, all three stores, and the following dispersal call. The stores have no independent per-byte branch.

### Ordered body

1. Convert deployed Building leptons to the signed-adjusted `/256` cell.
2. Write the House primary base cell through the helper at `0x0050E000`.
3. Call `FUN_00505180 @ 0x00505180`.
4. Write BasePlan node zero's packed cell through `(*(House+0x5708)+4)`.
5. Write the distinct BasePlan center at `House+0x5750`.
6. `0x007398FF`: write byte `1` to `House+0x1EE`.
7. `0x0073990C`: write byte `1` to `House+0x1F2`.
8. `0x00739919`: write byte `1` to `House+0x1F3`.
9. `0x00739926`: call `FUN_0050C920`.

There is no conditional jump, call, or observable failure edge between the three byte stores. Repeated qualifying deployments rewrite literal `1` idempotently, but still execute the surrounding anchoring and post-store call.

## `FUN_00505180`, `Computer_Paranoid`, and post-store boundary

`FUN_00505180 @ 0x00505180` is a pre-flag setup helper:

- It computes the native player-control predicate: in campaign, `CurrentPlayer || PlayerControl`; in nonzero mode, `CurrentPlayer` only.
- For a noncontrolled House in a nonzero mode it calls static `HouseClass__Computer_Paranoid @ 0x00501640`.
- If `House+0x5714` says the BasePlan node vector is empty, it saves `g_MapEditorMode`, forces it to zero, calls `HouseClass__AI_RecalcBuildOptions @ 0x005054B0`, then restores the prior editor-mode value.
- It never reads or writes `+0x1EE`, `+0x1F2`, or `+0x1F3`.

`Computer_Paranoid` is not a House-latch helper. Its own gates are a network/session virtual or `Rules+0x14B5 AlliesAllowed`, plus clear `Scenario+0x11E0`; it scans active, nondefeated AI Houses, adjusts `+0x24A`, allies AI Houses, and enemies human Houses. It touches none of the three target bytes. Retail says `AlliesAllowed=no` and `Paranoid=yes`; the call from `FUN_00505180` itself does not directly test the `Paranoid` INI key.

`FUN_0050C920` is the immediate post-store dispersal boundary. It selects a nearby passable destination and moves eligible owned Techno objects away from the base center. Full target-offset census proves it neither consumes nor rewrites the three bytes. No conclusion about its deeper selection algorithm is required here.

## Constructor and lifecycle defaults

`HouseClass__Constructor` initializes:

- `0x004F56F1`: `House+0x1EE = 0`
- `0x004F570A`: `House+0x1F2 = 0`
- `0x004F5710`: `House+0x1F3 = 0`

Nearby but distinct bytes include `+0x1EF AutocreateAllowed = 0` and `+0x1F0 = 1`. The three target bytes are ordinary bytes, not bits in a shared mask. Native writers emit only `0` or `1`; consumers test zero/nonzero, so a noncanonical loaded nonzero byte behaves as true.

The latches do not expire with time and have no pause-specific transition. A paused simulation simply performs no updating writer. Fresh state is zero; successful qualifying deployment is all one; save/load restores the stored values; later explicit writers may diverge them.

## Mechanism A — `+0x1EE Production`

### Complete writer set

| Address/path | Value and gate | Stock relevance |
|---|---|---|
| `0x004F56F1`, constructor | `0` | Always |
| `0x004F85B0`, `HouseClass__Update` | `1` after AI-auto-enable gate | Active |
| `0x0050A7EF`, `HouseClass__ComputerTakeover` | `1` after successful takeover/base-unit path | Active |
| `0x006DEAC0`, trigger action 3 | `1` after resolving a nonnull House | Stock campaign active |
| `0x006E99CC`, team script opcode 29 | `1`, then advances the team step | Compiled/custom; zero stock-data hits |
| `0x007398FF`, `UnitClass__Deploy` | `1` under the shared deploy gate | Active |

There is no runtime zero writer. Once enabled, Production stays enabled for the House lifetime or until loading a saved zero state.

### Consumer

`FUN_004500F0` reads the Building-like receiver's owner at `+0x21C`, then tests `House+0x1EE` at `0x0045024E` and again at `0x004502FA`. It gates the AI automatic-production tail after completed-production handling. The tail excludes construction/selling missions, requires a factory-capable type, resolves/creates a FactoryClass when necessary, applies unit naval/nonnaval factory compatibility, and begins production; failed/expired paths clean up. The second check protects the no-current-factory creation path. This is a direct deterministic production gate, not descriptive state.

### House update ingress and `[IQ] Production`

`HouseClass__Update 0x004F8564..0x004F85B7` first rejects native player-controlled Houses (campaign: `CurrentPlayer || PlayerControl`; nonzero mode: `CurrentPlayer`). Its AutoBaseBuilding read is at `0x004F858B`. For a noncontrolled House it enables when either:

- `House+0x1F3 AutoBaseBuilding != 0`, or
- signed `House+0x24C CurrentIQ >= Rules+0x143C [IQ] Production`.

It then writes, in order, `AutoBaseBuilding=1`, `Production=1`, `AutocreateAllowed=1`. Both retail `rules.ini` and `rulesmd.ini` use `[IQ] Production=5`; the Rules constructor default is also 5. This is not `[IQ] MaxIQLevels`: that separate field is `Rules+0x1434`.

A deploy sets AutoBaseBuilding and Production but not `AutocreateAllowed`; the next eligible House update sets the latter because AutoBaseBuilding is already true, even if CurrentIQ is below 5.

## Mechanism B — `+0x1F2 AITriggersActive`

### Complete writer set

| Address/path | Value and gate | Stock relevance |
|---|---|---|
| `0x004F570A`, constructor | `0` | Always |
| `0x0050A7F6`, `HouseClass__ComputerTakeover` | `1` | Active |
| `0x006DF2FA`, trigger action 74 (`AITriggersBegin`) | `1` after nonnull House resolution | Stock campaign active |
| `0x006DF339`, trigger action 75 (`AITriggersStop`) | `0` after nonnull House resolution | Stock campaign active |
| `0x0073990C`, `UnitClass__Deploy` | `1` under the shared deploy gate | Active |

### Consumer

`FUN_006F0AB0`, the AITrigger selector, reads the byte at `0x006F0B12`. Zero prevents trigger selection. Native ordering is significant: the selector consumes `RandomRanged(1,100)` before its ratio test and before the `AITriggersActive` byte check. A disabled House therefore still consumes that first RNG draw when the selector runs. Re-enabling with action 74 or a later qualifying deploy resumes from the advanced RNG state.

No other runtime consumer survived the full instruction census. Trigger action 75 can split this latch back to zero while Production and AutoBaseBuilding remain true.

## Mechanism C — `+0x1F3 AutoBaseBuilding`

### Complete writer set

| Address/path | Value and gate | Stock relevance |
|---|---|---|
| `0x004F5710`, constructor | `0` | Always |
| `0x004F85A9`, `HouseClass__Update` | `1` under the update gate above | Active |
| `0x0050A7FD`, `HouseClass__ComputerTakeover` | `1` | Active |
| `0x006DE21B/0x006DE29F`, trigger action 30 | action parameter nonzero -> `1`; zero -> `0` | Compiled/custom; zero stock-data hits |
| `0x00739919`, `UnitClass__Deploy` | `1` under the shared deploy gate | Active |

### Consumer 1: `UnitClass__AI`

`UnitClass__AI 0x007363DE..0x0073645B` reads AutoBaseBuilding at `0x0073641A` and queues mission `HUNT (0x0F)` only when all of these hold:

- the UnitType's `DeploysInto` target is present in source-ordered `[AI] BuildConst`;
- owner `AutoBaseBuilding` is nonzero;
- owner is not human-controlled;
- `g_GameMode != 0`;
- the House owns zero live BuildConst Buildings (`House+0x60 == 0`);
- the current mission is neither HUNT nor UNLOAD.

### Consumer 2: `UnitClass__Mission_Guard`

`UnitClass__Mission_Guard` reads the byte at `0x007409FE`. For a BuildConst-deployer unit owned by a nonhuman House with AutoBaseBuilding nonzero, it queues mission `UNLOAD (0x10)`, then returns through the randomized mission-timer path. Together the two consumers form the native auto-mobilize/auto-deploy loop for MCV-like units; the House latch, BuildConst identity, ownership state, mission state, and game mode all matter.

Retail BuildConst data is active and edition-specific:

- RA2 `rules.ini`: `GACNST,NACNST`
- YR `rulesmd.ini`: `GACNST,NACNST,YACNST`

### Trigger action 30 detail

Action 30 resolves the target House before branching on its action byte. A zero parameter clears only AutoBaseBuilding. A nonzero parameter sets it and, when the House owns a BuildConst Building, derives the primary and BasePlan centers from the first such object; an empty BasePlan invokes Recalc. It does not clear or set Production/AITriggersActive as a group. If it clears AutoBaseBuilding while CurrentIQ remains at least `[IQ] Production`, the next eligible House update sets it back to one.

## `HouseClass__ComputerTakeover`

`HouseClass__ComputerTakeover @ 0x0050A5C0` supplies a second active three-latch transaction:

1. It enters only for a currently player-controlled House (campaign: `CurrentPlayer || PlayerControl`; nonzero: `CurrentPlayer`).
2. It clears the control bytes, stamps CurrentIQ from `Rules+0x1434 MaxIQLevels`, abandons factories, and performs takeover setup.
3. It searches owned units backward for a live `BaseUnit` type. If no match exists, it returns before centers or target latches.
4. With a match, it writes the primary center; in nonzero mode it invokes `Computer_Paranoid`, optionally recalculates an empty BasePlan, writes node zero and the BasePlan center.
5. It writes `Production=1 @ 0x0050A7EF`, `AITriggersActive=1 @ 0x0050A7F6`, then `AutoBaseBuilding=1 @ 0x0050A7FD` — the same semantic and byte order as deploy.

Unlike `UnitClass__Deploy`, this path does not call `FUN_0050C920`. Retail BaseUnit data is RA2 `AMCV,SMCV`; YR adds `PCV`.

## Script ingress boundaries

- Trigger action 3 (`ProductionBegins`) resolves its target House through `FUN_006E45E0`; null returns failure, nonnull writes Production=1 and returns success.
- Trigger actions 74/75 use the same nonnull House-resolution boundary and set/clear only AITriggersActive.
- Team script opcode 29 gets the House from `Team+0x2C`, writes Production=1, sets `Team+0x80=1` to advance/complete that step, and returns.
- YRpp enum names corroborate IDs 3, 30, 74, and 75, but the active binary supplies the behavioral authority.

## Save, load, and checksum lifecycle

`HouseClass__Save @ 0x00504080` calls `AbstractClass__Save`, which serializes the receiver's raw object block using its virtual size. `HouseClass SizeOf @ 0x00504730` returns `0x160B8`. `HouseClass__Load @ 0x00503040` calls `AbstractClass__Load` before reconstructing/swizzling dynamic members. All three offsets lie inside the raw block, so all three values persist exactly.

The raw House CRC routine is a missed function boundary at `0x00502D60..0x0050303F`:

- `0x00502E58` feeds `House+0x1EE` through the boolean CRC helper `0x004A1CA0`.
- `0x00502E74` feeds `House+0x1F2` through the same sequence.
- The complete `+0x1F3` census finds no direct CRC read. AutoBaseBuilding is persisted but deliberately omitted from the direct native House CRC stream.

This asymmetry is native and must be preserved in any native-shaped compatibility hash: hash Production and AITriggersActive at their corresponding House position; do not directly add AutoBaseBuilding merely because it is serialized. AutoBaseBuilding can still affect later deterministic state and later CRC indirectly, notably by causing House update to set Production.

## Retail activation and TS-legacy exclusions

Installed-retail map/script enumeration found:

| Container | Action 3 | Action 30 | Action 74 | Action 75 | Team opcode 29 |
|---|---:|---:|---:|---:|---:|
| `expandmd01.mix` | 2 | 0 | 2 | 0 | 0 |
| `MAPS01.MIX` | 28 | 0 | 30 | 0 | 0 |
| `MAPS02.MIX` | 25 | 0 | 24 | 1 | 0 |
| `mapsmd03.mix` | 25 | 0 | 21 | 1 | 0 |
| loose maps, `MULTI.MIX`, `multimd.mix` | 0 | 0 | 0 | 0 | 0 |
| `ai.ini` / `aimd.ini` scripts | n/a | n/a | n/a | n/a | 0 |

Therefore action 3, action 74, and action 75 are active stock-campaign data surfaces. Action 30 and team opcode 29 are compiled and valid for inherited/custom-map compatibility but are not activated by the enumerated stock retail corpus. They are evidence-backed exclusions from the ordinary stock-skirmish implementation row, not evidence that the underlying latches are TS-dead. Deploy, House update, takeover, selector gating, and AutoBaseBuilding consumers are active YR code/data mechanisms.

## Current Rust delta

### Missing state and rule authority

- `src/sim/house_state.rs::HouseState` has no independent Production, AITriggersActive, or AutoBaseBuilding fields; `HouseState::new` therefore cannot reproduce native zero defaults.
- `src/rules/ruleset.rs` parses `MaxIQLevels`, `RepairSell`, and `SellBack` from `[IQ]`, but not `[IQ] Production` or its constructor default 5.
- Snapshot schema version 107 persists the Phase-3 BasePlan center but none of these latches.
- `src/sim/world/world_hash.rs::hash_houses` hashes neither Production nor AITriggersActive. AutoBaseBuilding should be serialized but not directly included in the native-shaped House hash.

### Missing deploy/update behavior

- `src/sim/world/world_spawn.rs::deploy_mcv` currently stops after primary center, optional Recalc, node-zero, and BasePlan-center writes. It omits the three ordered stores and the following dispersal behavior.
- There is no House-update latch step implementing `AutoBaseBuilding || CurrentIQ >= Production` and the ordered AutoBaseBuilding/Production/AutocreateAllowed writes.
- There is no House computer-takeover path matching the native takeover writer.

### Missing consumers and script ingress

- `src/sim/team_script_vm.rs` explicitly labels AITrigger selector semantics as a later stage; there is no selector and therefore no AITriggersActive gate or pre-gate RNG consumption.
- `src/sim/trigger_runtime.rs` does not implement actions 3, 30, 74, or 75.
- `src/sim/team_script_vm.rs` supports only a small opcode subset; opcode 29 enters `UnsupportedAction`.
- Production is driven by `src/sim/ai.rs` without a House Production latch equivalent.
- `src/sim/ai.rs::AiPlayerState::mcv_deployed` is a local one-shot “attempted” marker. It is not House-owned, does not share native writers/reset/persistence/hash semantics, does not gate on BuildConst identity or AutoBaseBuilding, and bypasses the native HUNT -> GUARD -> UNLOAD mission flow. It must not be reused as any of the three latches.

## Implementation handoff

Implement the three mechanisms as separate House-owned booleans, with one shared native-ordered deploy/takeover enabling helper if useful.

| Requirement | Minimum exact behavior | Acceptance evidence |
|---|---|---|
| State/defaults | Add three separate serialized fields, all false on fresh House construction. | Constructor and save/load tests for every independent combination. |
| Rules | Add `[IQ] Production`, constructor/default 5, retail override parsing. | Missing-key=5 and explicit-key tests. |
| Qualifying deploy | After BasePlan center: Production=true, AITriggersActive=true, AutoBaseBuilding=true, in that order; only nonhuman + ConstructionYard + nonzero mode. | Gate matrix; successful/blocked/facing-only/human/campaign/non-ConYard cases; ordering observation hook or focused transaction test. |
| House update | For native noncontrolled House, if AutoBaseBuilding or CurrentIQ>=Production, set AutoBaseBuilding, Production, and the separately owned AutocreateAllowed behavior in native order. | IQ below/equal/above; latch persistence; human/control-mode cases. |
| Production | Native AI auto-production tail must remain disabled while Production=false and enabled when true; do not substitute CurrentIQ directly at every consumer. | Fresh AI vs enabled AI production tests. |
| AI triggers | Selector consumes its first RNG draw before checking AITriggersActive; false blocks selection, true permits later gates. | Same-seed draw-position and enable/disable tests. |
| Auto base building | Reproduce BuildConst, nonhuman, nonzero-mode, zero-owned-BuildConst, and mission-state gates, including HUNT then UNLOAD behavior. | GACNST/NACNST/YACNST data tests and negative gate matrix. |
| Persistence/hash | Serialize all three; directly native-hash Production and AITriggersActive only. | Round trip all combinations; differential hash proves only the first two directly change House hash. |
| Stock triggers | Implement actions 3/74/75 if the Phase-3 row includes active campaign parity. | Nonnull/invalid House and independent-latch tests. |
| Compatibility residual | Action 30 and team opcode 29 may stay outside ordinary stock scope only with their zero-hit evidence recorded; do not conflate their absence with latch closure. | Explicitly scoped residual, or implement their exact independent writes. |
| Computer takeover | If takeover is already in/enters scope, share the same three-one ordering only after the live BaseUnit success path. | No-BaseUnit return and successful takeover tests. |
| Dispersal boundary | The three latches must commit before the post-deploy dispersal call; dispersal failure/empty candidates cannot roll them back. | Empty/eligible dispersal cases retain all latches. |

The smallest parity-safe Phase-3 closure is not “add three deploy booleans.” It is: state/default/persistence/hash, exact deploy transaction, `[IQ] Production`/House-update activation, and the active ordinary consumers. Action 3/74/75 are also required for stock campaign parity. Action 30/opcode 29 are separately scoped compatibility entry points.

## Coverage ledger

| Surface | Result |
|---|---|
| Deploy gates, values, order, and call boundary | Covered, instruction-verified |
| Constructor/defaults | Covered |
| `+0x1EE` writers/readers | Complete census |
| `+0x1F2` writers/readers | Complete census |
| `+0x1F3` writers/readers | Complete census; constant false positive excluded |
| House update / `[IQ] Production` | Covered against binary and both retail rules files |
| ComputerTakeover | Covered only through relevant prerequisite and three stores |
| `FUN_00505180` / Computer_Paranoid | Boundary and noninteraction proved |
| `FUN_0050C920` | Immediate call-order and noninteraction proved; deeper internals excluded by scope |
| Trigger and team-script ingress | Covered; stock activation enumerated |
| Save/load/raw size | Covered |
| Native House CRC | Covered; missed boundary identified |
| Active YR mode/data vs inherited surfaces | Covered |
| Rust state/deploy/rules/AI/triggers/snapshot/hash | Directly scanned |
| Dynamic debugger observation | Zero-add: static binary/data evidence is decisive; no timing or indirect-call ambiguity remained |

## Zero-add pass

No additional capture or dynamic run would change the implementation contract. The stores are direct literal byte writes on a straight-line path; every operand reference is statically enumerable; data activation is directly countable; persistence uses the raw-object serializer; and current Rust state is directly inspectable. A debugger watchpoint would only replay already-proved store order. No evidence gap was hidden behind a virtual call, data-dependent pointer, or unparsed retail container.

## Adversarial five

1. **What if `+0x1EE/+0x1F2/+0x1F3` are padding accidentally written together?** Rejected: each has independent named behavior, multiple writers, and live readers; two are CRC-fed and all three persist.
2. **What if `FUN_00505180` or `Computer_Paranoid` is the real enable gate?** Rejected: neither accesses the bytes; explicit stores occur only after the helper returns.
3. **What if `FUN_0050C920` decides whether the writes count?** Rejected: stores precede the call, no target access exists in the callee census, and there is no rollback edge.
4. **What if Rust's `mcv_deployed` already supplies parity?** Rejected: it is per-AI-controller attempted-state, not House state, has different gates and lifecycle, and cannot represent action 75 or the three independent combinations.
5. **What if stock data never exercises the individual mechanisms?** Rejected: deployment/update/selector/BuildConst paths are active; campaign data contains actions 3, 74, and 75. Only action 30/opcode 29 are zero-hit compatibility surfaces.

## Tiny details that are easy to lose

1. The deploy store order is Production, AITriggersActive, AutoBaseBuilding — not offset-sorted across all neighboring bytes.
2. The three writes occur after BasePlan node zero and `House+0x5750`, before dispersal.
3. No call or conditional lies between the three stores.
4. Human-control semantics differ between campaign and nonzero game modes.
5. A successful deploy does not directly set neighboring `AutocreateAllowed +0x1EF`.
6. House update later sets AutocreateAllowed because AutoBaseBuilding is already true.
7. `[IQ] Production` is `Rules+0x143C`; `MaxIQLevels` is `+0x1434`.
8. `[IQ] Production=5` in both retail rules files and in the constructor.
9. Production has no runtime clear writer.
10. AITriggersActive has a stock-active explicit clear writer: trigger action 75.
11. AutoBaseBuilding has an explicit action-30 clear writer, but stock installed maps do not invoke it.
12. Clearing AutoBaseBuilding can be undone next update when CurrentIQ remains at least Production.
13. The AITrigger selector consumes RNG before noticing AITriggersActive is false.
14. AutoBaseBuilding requires the deploy target, not merely the source unit, to be in BuildConst.
15. Unit AI refuses to requeue when already on HUNT or UNLOAD.
16. Mission Guard is the path that queues UNLOAD; Unit AI first queues HUNT.
17. The owned-BuildConst zero-count gate is separate from source type membership.
18. YR adds `YACNST` to BuildConst and `PCV` to BaseUnit.
19. ComputerTakeover returns before all three stores if it finds no live BaseUnit.
20. ComputerTakeover uses the same three-store order but has no following `FUN_0050C920` call.
21. All three bytes raw-persist; only Production and AITriggersActive are directly House-CRC-fed.
22. Consumers treat any nonzero loaded byte as true even though native writers use 0/1.
23. `0x00640CF1 MOV EAX,0x1F3` is a numeric constant false positive, not a House field access.
24. `0x00537830`-area `0x1EE1/0x1EE3/0x1EE5` immediates are not `House+0x1EE` references.

## Annotation candidates — reported, not applied

- Create/recognize the missed function boundary at `0x00502D60` as the raw House checksum/CRC body ending before `HouseClass__Load @ 0x00503040`.
- Candidate House field labels: `+0x1EE Production`, `+0x1F2 AITriggersActive`, `+0x1F3 AutoBaseBuilding`.
- Candidate `RulesClass+0x143C` label: `IQProductionThreshold`.

No Ghidra metadata was changed.

## Sources

- Active retail `gamemd.exe`, live read-only Ghidra (`UnitClass__Deploy`, House constructor/update/save/load/CRC/takeover, `FUN_00505180`, `Computer_Paranoid`, trigger actions, team script, AITrigger selector, Unit AI/Guard).
- Retail `rules.ini`, `rulesmd.ini`, `ai.ini`, `aimd.ini`, and installed map archives listed above.
- Current Rust tree on `feature/phase3-map-spatial-close`.
- `docs/research/PHASE3_HOUSECLASS_ORDINARY_BASE_PLACEMENT_005060B0_GHIDRA_REPORT.md` for the already-closed anchoring context.
- `docs/research/GACNST_ISDEPLOYABLE_SPECIAL_BRANCH_GHIDRA_REPORT.md` and `docs/research/PHASE3_AITRIGGER_SELECTOR_ELIGIBILITY_GHIDRA_REPORT.md` as search leads only; load-bearing claims were rechecked.
- `C:/Users/enok/Documents/YRpp/HouseClass.h` and `GeneralDefinitions.h` for semantic-name corroboration only.
