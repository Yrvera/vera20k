# Phase 3 House-update AI activation — exhaustive active-retail Ghidra report

**Date:** 2026-08-27  
**Binary:** installed active Yuri's Revenge `gamemd.exe` in live project `testProsjekt`  
**Primary code:** `HouseClass__Update @ 0x004F8440`, activation block `0x004F8564..0x004F85B7`  
**Investigation class:** exhaustive-slice; research only  
**Status:** COMPLETE for the bounded House-update activation mechanism  
**Confidence:** High. The transition, direct field census, tick owner, initialization, parsers,
save/load/CRC lifecycle, retail inputs, and current Rust delta were all checked directly.

## Overview and verdict

Every non-null `HouseClass` is visited from the House-array tail of
`LogicClass__PerTickUpdate @ 0x0055AFB0`. Near the start of
`HouseClass__Update`, after early power/radar/anger/extension work and before
victory/defeat/strategy/production management, the active binary performs this exact transition:

```text
controlled = CurrentPlayer || (GameMode == 0 && PlayerControl)
if !controlled && (AutoBaseBuilding != 0 || CurrentIQ >= Rules.IQ.Production) {
    AutoBaseBuilding = 1
    Production = 1
    AutocreateAllowed = 1
}
```

The IQ comparison is signed and inclusive. `AutoBaseBuilding != 0` bypasses IQ entirely.
The three literal-one stores are adjacent and ordered exactly as shown. The block has no RNG,
timer, defeated/passive, power, difficulty, scenario-type, or additional mode gate and makes no
calls. Repeated eligible updates rewrite the same bytes; there is no expiry or toggle.

The most important negative finding is that `House+0x1EF AutocreateAllowed` has **no direct
ordinary-gameplay reader in the entire active executable**. Its direct accesses are constructor
clear, this update's set, trigger action 13's set, and House CRC. It is also persisted through the
raw House save block. It must therefore exist, save, and hash, but it must not be invented as a
gate for TeamType creation or another subsystem.

The current Rust implementation already has the correct control predicate, CurrentIQ load paths,
factory-before-House-tail order, and three neighboring deploy latches. It lacks `[IQ] Production`,
the fourth `AutocreateAllowed` latch, this House-tail transition, its snapshot/hash coverage, and
trigger action 13. Current offline modal admission correctly freezes the entire simulation: native
Menu, Abort Confirm, and Options are blocking `ProcessModalServicePump` owners, and modes 0/5 never
reenter `Main_Tick` from that pump. If an eligible network-mode modal does reenter `Main_Tick`,
`g_GameState != 0` skips only the normal input/render block and still executes the entire PerTick,
including this transition as part of the full House update; it is not an activation-only lane.

## Scope boundary and prior-work relation

This report extends only the explicit gap left by:

- `docs/research/PHASE3_UNIT_DEPLOY_HOUSE_FLAGS_GHIDRA_REPORT.md` — deploy, takeover, and the
  three neighboring latch families; it left the House-update/Autocreate mechanism open.
- `docs/research/PHASE3_HOUSECLASS_ORDINARY_BASE_PLACEMENT_005060B0_GHIDRA_REPORT.md` — ordinary
  BasePlan selection/placement and `AutoBaseBuilding` placement consumers; it left this update
  transition open.

This report rechecks only the neighboring accesses needed to prove the transition. It does not
re-open ordinary base placement, MCV deploy geometry/dispersal, the AITrigger selector,
factory-production behavior beyond its `Production` gate, or computer-takeover setup.

## Field and global map

| Owner | Offset/address | Width | Meaning in this slice | Evidence |
|---|---:|---:|---|---|
| House | `+0x1EC` | byte | `CurrentPlayer`; always participates in control gate | `0x004F856B` |
| House | `+0x1ED` | byte | `PlayerControl`; participates only when `GameMode == 0` | `0x004F8577` |
| House | `+0x1EE` | byte | `Production`; second activation store | `0x004F85B0` |
| House | `+0x1EF` | byte | `AutocreateAllowed`; third activation store | `0x004F85B7` |
| House | `+0x1F2` | byte | `AITriggersActive`; neighboring deploy/takeover latch, **not touched here** | full operand census |
| House | `+0x1F3` | byte | `AutoBaseBuilding`; precondition/bypass and first store | `0x004F858B`, `0x004F85A9` |
| House | `+0x1F7` | byte | next `Savour`/result-state block; proves transition boundary | `0x004F85BE` |
| House | `+0x24C` | signed dword | `CurrentIQ` | `0x004F859B` |
| Rules | `+0x1434` | signed dword | `[IQ] MaxIQLevels`; distinct from activation threshold | ctor/read/create-Houses |
| Rules | `+0x143C` | signed dword | `[IQ] Production`; activation threshold | `0x004F85A1` |
| global | `0x00A8B238` | dword | `GameMode`; zero is campaign-family control semantics | `0x004F8564` |
| global | `0x008871E0` | pointer | live `RulesClass` | `0x004F8595` |

`HouseClass` size is `0x160B8` (`0x00504730: MOV EAX,0x160B8; RET`), so every listed House
field is inside the raw persisted object block.

## Exact activation disassembly and branch semantics

```asm
004F8564  MOV EAX,[00A8B238]          ; GameMode
004F8569  CMP EAX,EBP                 ; EBP == 0
004F856B  MOV AL,[ESI+1EC]            ; CurrentPlayer
004F8571  JNZ 004F8587                ; nonzero mode ignores PlayerControl
004F8573  TEST AL,AL
004F8575  JNZ 004F8585
004F8577  MOV AL,[ESI+1ED]            ; campaign-only PlayerControl
004F857D  TEST AL,AL
004F857F  JNZ 004F8585
004F8581  XOR AL,AL
004F8583  JMP 004F8587
004F8585  MOV AL,1
004F8587  TEST AL,AL
004F8589  JNZ 004F85BE                ; controlled House skips all work
004F858B  MOV AL,[ESI+1F3]            ; AutoBaseBuilding
004F8591  TEST AL,AL
004F8593  JNZ 004F85A9                ; any nonzero bypasses IQ
004F8595  MOV ECX,[008871E0]           ; Rules
004F859B  MOV EAX,[ESI+24C]            ; signed CurrentIQ
004F85A1  CMP EAX,[ECX+143C]           ; signed [IQ] Production
004F85A7  JL  004F85BE                ; strictly below skips; equality passes
004F85A9  MOV byte ptr [ESI+1F3],1
004F85B0  MOV byte ptr [ESI+1EE],1
004F85B7  MOV byte ptr [ESI+1EF],1
004F85BE  MOV AL,[ESI+1F7]             ; next independent block
```

### Mode/control truth table

| Game mode | CurrentPlayer | PlayerControl | Controlled for this block? | Can activate? |
|---:|---:|---:|---:|---:|
| `0` | `0` | `0` | no | yes, by AutoBase/IQ |
| `0` | `0` | nonzero | yes | no |
| `0` | nonzero | either | yes | no |
| nonzero | `0` | either | no (`PlayerControl` ignored) | yes, by AutoBase/IQ |
| nonzero | nonzero | either | yes | no |

The control result matches Rust `HouseState::is_controlled_by_human` at
`src/sim/house_state.rs:395-397` exactly.

## Tick owner, ordering, and pause/mode activation

### Scheduler owner

`LogicClass__PerTickUpdate @ 0x0055AFB0` reaches the House array at
`0x0055B68D..0x0055B6B1`. It:

1. loads live House count from `0x00A80238`;
2. walks forward from index zero;
3. reloads the array base (`0x00A8022C`) for each item;
4. null-checks the item;
5. calls vtable slot `+0x5C`;
6. reloads the live count after the callback before the next comparison.

The House vtable entry at `0x007EA8FC` contains little-endian `0x004F8440`, proving that slot
is `HouseClass__Update`. Global FactoryClass updates finish immediately before the House loop.
The activation is therefore House-owned, once per reached House visit, not an AI-player pass and
not a factory callback.

### Position inside `HouseClass__Update`

The full live decompile places activation after:

- blackout/power timer handling and power assessment;
- radar/low-power recheck;
- the 100-frame anger-score decay;
- the optional `House+0x57E0` callback.

It is before:

- `House+0x1F7` victory/result (`Savour`) work;
- rally/scatter and alert work;
- superweapon readiness;
- multiplayer defeat processing and trigger events;
- strategy and later production/chooser management.

Thus a House can become activated on the same House visit before all later House AI/defeat work.
The block does not check `is_defeated`; eligibility is evaluated first.

### Caller-level modal ownership and special-frame matrix

`Main_Tick @ 0x0055D360` has one direct call to `LogicClass__PerTickUpdate`, at
`0x0055DC9E`. Its internal `g_GameState != 0` branch skips the normal gameplay block but not
PerTick. That fact does **not** establish that every modal owner calls `Main_Tick`.
`Main_Game @ 0x0048CCC0` calls `State_Machine @ 0x0048C8B0` after its ordinary Main Tick;
states 1, 3, and 5 enter blocking Menu, Abort Confirm, and Options owners, each looping
`ProcessModalServicePump @ 0x00623120`. The pump's caller-level admission establishes:

| Native outer state | Main Tick / PerTick / House activation? |
|---|---|
| `g_GameActive == 0` | no Main Tick |
| `g_GameRunning == 0` focus/network wait | no PerTick |
| `Scenario+0x62C != 0` intro/delay early-return | no PerTick |
| offline mode 0/5 Menu, Abort Confirm, or Options open | **no reentrant Main Tick**; pump runs network-message/service work only, so activation and frame advance are frozen |
| eligible network-mode modal, blockers clear and not reentrant | **yes**; reentrant Main Tick skips the normal gameplay block, then runs the entire PerTick/House update and late frame tail |
| modal pump blocked or already reentrant | no Main Tick from that pump iteration |
| replay playback | yes |
| ordinary active frame | yes |

There is no pause check in the House block itself, but offline modal exclusion occurs above it at
the blocking pump's caller boundary. Current Rust `src/app/match_runtime/sim_tick.rs` admits modal
simulation only for network mode and therefore correctly freezes offline campaign/skirmish. The
activation pass belongs on every admitted full House-update frame: ordinary active frames and any
future eligible network-modal frame, never a special offline or activation-only modal lane.

## Construction, rules, scenario parsing, and overrides

### House defaults

`HouseClass__Constructor` clears the contiguous flags:

- `+0x1EC = 0` at `0x004F56E5`;
- `+0x1ED = 0` at `0x004F56EB`;
- `+0x1EE = 0` at `0x004F56F1`;
- `+0x1EF = 0` at `0x004F56F7`;
- `+0x1F3 = 0` at `0x004F5710`.

Constructor `CurrentIQ +0x24C` is also zero: `0x004F57BB` copies the already-zero House
`+0x1D0` seed. The scenario parser does not parse or override `+0x1EE/+0x1EF/+0x1F3`.

### `[IQ] Production`

`RulesClass` construction uses the same `EDX=5` for `MaxIQLevels +0x1434` and
`Production +0x143C` (`0x006671A1..0x006671C1`). `RulesClass__ReadIQ @ 0x00674240`:

- leaves constructor values unchanged if `[IQ]` is absent;
- supplies the current stored value as the `ReadInt` default;
- reads the `Production` string at `0x0083D4F0` (memory bytes decode to `Production\0`);
- writes the signed result directly at `0x006742C1` with no clamp.

Consequently custom negative or above-five thresholds are valid native inputs. Hardcoding 5 or
clamping to `MaxIQLevels` is wrong. Installed `ini/rules.ini:2630` and
`ini/rulesmd.ini:3160` both explicitly set `Production=5`; both also set
`MaxIQLevels=5` (`rules.ini:2628`, `rulesmd.ini:3158`).

### House `IQ=` parser

`HouseClass__Read_Scenario_INI @ 0x00500B40` reads the named House section's `IQ=` with
default zero (`0x00500D94..0x00500DA2`). It signed-compares the result to
`Rules.MaxIQLevels +0x1434`:

- value `<= MaxIQLevels`: retain it, including negatives;
- value `> MaxIQLevels`: replace it with literal `1`, **not** with MaxIQLevels;
- store the same value to `House+0x1D0` and `CurrentIQ +0x24C` at
  `0x00500DBA/0x00500DC0`.

Current Rust already reproduces this quirk through
`scenario_current_iq` and `src/sim/scenario_bootstrap.rs:1816-1822`.

### Generated noncampaign Houses

`ScenarioClass__Create_Houses @ 0x00687F10` leaves generated humans at constructor IQ zero.
For a generated computer House in nonzero game mode, it loads Rules `+0x1434` and stores it to
House `+0x24C` at `0x0068828D`. Stock skirmish therefore creates computer Houses at IQ 5,
equal to Production 5, so each activates on its first reached House update even without an MCV
deploy. Generated neutral/special Houses normally remain IQ 0 and do not activate unless another
writer sets AutoBaseBuilding or IQ/threshold makes them eligible.

`HouseClass__ComputerTakeover` clears `+0x1EC/+0x1ED`, stamps CurrentIQ from MaxIQLevels at
`0x0050A614`, and later writes Production/AITriggersActive/AutoBaseBuilding when its base-unit
path succeeds. It never directly writes `+0x1EF`; either the newly high IQ or nonzero AutoBase
causes the next eligible House update to add AutocreateAllowed.

## Complete `AutocreateAllowed +0x1EF` access census

A zero-add whole-program `search_instructions` pass scanned 1,161,572 instructions for operand
`0x1ef`. It returned six textual matches; two are unrelated immediate constants (`PUSH 0x1EFC`
and `PUSH 0x1EFE`). The four real field accesses are exhaustive:

| Address | Owner | Access | Exact behavior |
|---:|---|---|---|
| `0x004F56F7` | House constructor | write | clear to zero through zeroed `BL` |
| `0x004F85B7` | House update | write | set literal one, third activation store |
| `0x00502E66` | raw House CRC boundary | read | pass byte to `CRCEngine__AddBool @ 0x004A1CA0` |
| `0x006DEB41` | `TriggerAction__Execute` case 13 | write | set literal one after nonnull House resolution |

No instruction clears it after construction. No direct ordinary gameplay function reads it.
Raw save/load accesses it indirectly as part of the object block and are covered below.

### Trigger action 13

Case 13 reads its target parameter from `TActionClass+0x90`, calls the shared House resolver
`FUN_006E45E0`, returns false on null, otherwise writes `House+0x1EF=1` at `0x006DEB41` and
returns true. It does not touch Production, AITriggersActive, or AutoBaseBuilding.

`FUN_006E45E0` is the same House-resolution boundary used by trigger action 3 in the parent
deploy-latch report. Its active paths reject a null context and parameter `-1`, handle special
parameter `0x2325`, otherwise resolve through scenario/country House lookup. Exact generic House
resolution belongs to the shared trigger-runtime action layer, not to House-update activation.

An effective mounted-name census of 310 map payloads from `expandmd01.mix`, `mapsmd03.mix`,
`multimd.mix`, `MAPS01.MIX`, `MAPS02.MIX`, and `MULTI.MIX` parsed every counted eight-token
`[Actions]` chunk and found **zero action-13 chunks**. The writer is valid compiled active code,
but no shipped map in that mounted corpus invokes it.

## Relevant neighboring field census

### `Production +0x1EE`

The whole-program operand census found these real accesses:

- constructor clear `0x004F56F1`;
- House-update set `0x004F85B0`;
- direct CRC read `0x00502E58`;
- computer-takeover set `0x0050A7EF`;
- trigger action 3 set `0x006DEAC0`;
- Team script opcode 29 path set `0x006E99CC`;
- MCV/base-unit deploy set `0x007398FF`;
- two reads in `FUN_004500F0` at `0x0045024E/0x004502FA`.

`FUN_004500F0` is a factory/production-tail routine. Both tests return immediately when the
owning House's Production byte is zero; after the gate it can acquire a primary factory, create a
`FactoryClass`, and start/resume production. This proves Production is a real downstream latch,
not merely diagnostic state. Its full production policy remains owned by the parent production
row. No direct later zero writer was found.

### `AutoBaseBuilding +0x1F3`

The complete real-access set is:

- constructor clear `0x004F5710`;
- House-update read/set `0x004F858B/0x004F85A9`;
- computer-takeover set `0x0050A7FD`;
- trigger action 30 set/clear `0x006DE21B/0x006DE29F`;
- `UnitClass__AI` read `0x0073641A`;
- successful deploy set `0x00739919`;
- `UnitClass__Mission_Guard` read `0x007409FE`.

One unrelated `MOV EAX,0x1F3` immediate at `0x00640CF1` is not a field access. The Unit AI/Guard
consumers and action-30 semantics are already closed in the parent deploy/base-placement reports.
The important interaction here is:

- any nonzero AutoBase forces this update's three stores regardless of IQ;
- action 30 clear can leave Production/Autocreate true and AutoBase false;
- if IQ is still at/above Production, the next update restores AutoBase;
- if IQ is below threshold, that split state persists;
- deploy writes Production, AITriggersActive, AutoBase but not Autocreate, so this update fills
  the missing Autocreate latch on the first subsequent eligible House visit.

`AITriggersActive +0x1F2` is deliberately not written by this block.

## Save, load, and CRC lifecycle

`HouseClass__Save @ 0x00504080` first calls `AbstractClass__Save`; the base serializer writes the
raw virtual-sized House block. `HouseClass__Load @ 0x00503040` calls `AbstractClass__Load` before
reconstruction/swizzling of dynamic members. With House size `0x160B8`, Production,
AutocreateAllowed, AutoBaseBuilding, and CurrentIQ all persist through save/load.

The raw House CRC routine occupies a missed function boundary at
`0x00502D60..0x0050303F`:

- `+0x1EE Production` -> `CRCEngine__AddBool` at `0x00502E58..0x00502E61`;
- `+0x1EF AutocreateAllowed` -> `CRCEngine__AddBool` at `0x00502E66..0x00502E6F`;
- `+0x1F2 AITriggersActive` -> `CRCEngine__AddBool` at `0x00502E74..0x00502E7D`;
- `+0x24C CurrentIQ` -> integer CRC helper at `0x00502E90..0x00502E99`;
- exhaustive `+0x1F3` census: no direct CRC read.

`CRCEngine__AddBool @ 0x004A1CA0` normalizes any nonzero input byte to boolean one before folding.
Raw save/load preserves a noncanonical byte exactly, but all active code writers emit only 0/1.
On the next eligible noncontrolled update, a noncanonical nonzero AutoBase is normalized to literal
one and causes Production/Autocreate to be set. Arbitrarily hand-mutated native save bytes are not
a reachable active-retail writer surface and do not require Rust to replace its boolean state with
raw bytes.

The correct compatibility-hash shape is therefore Production, AutocreateAllowed,
AITriggersActive included; AutoBaseBuilding persisted but not directly hashed. It can affect later
hashes indirectly by causing this transition.

## Retail data, TS legacy, and custom exclusions

### Shipped active YR data

- `[IQ] Production=5` and `MaxIQLevels=5` in both installed base `rules.ini` and YR
  `rulesmd.ini`.
- The 310-map mounted corpus contains House `IQ=` values in `{0,1,2,3,4,5}` only; 42 maps have
  at least one nonzero House IQ, and none has a negative or above-five House IQ.
- The same corpus contains zero trigger-action-13 chunks.
- Installed `aimd.ini` contains 163 TeamTypes and all 163 say `Autocreate=yes`.

That TeamType key is a **different field and mechanism**. Rust currently loads it into
`TeamTypeMetadata.autocreate`, but active `gamemd.exe` contains no read of House `+0x1EF` that
could gate it. Do not connect the two merely because their names resemble each other.

### Evidence-backed exclusions

- No TS executable or TS-only behavior is used as authority. The report is based on active YR
  `gamemd.exe`; installed base/RA2 INI is consulted only because YR's effective data inherits it.
- No hidden “autocreate team selector” is inferred. The exhaustive field census disproves a direct
  House-byte consumer in this binary.
- Trigger action 13 is absent from shipped mounted maps. It is a valid compiled/custom-map writer,
  but is separable from the stock House-update transaction and shares the generic trigger House
  resolver.
- Custom `[IQ] Production` is **not excluded**: the active parser accepts signed integers with no
  clamp, so Rust must parse and use the effective value.
- Corrupt/hand-edited noncanonical native save bytes are excluded as unreachable from all active
  writers; ordinary reachable save states are exactly boolean.
- This transition does not inherit neighboring TS/RA2 assumptions about `AITriggersActive`; it
  does not touch `+0x1F2` at all.

## Current Rust parity status

### Already matching and must be preserved

- `src/sim/house_state.rs:395-397` implements the exact mode-aware control predicate.
- `src/sim/house_state.rs:341` owns signed `current_iq`.
- `src/sim/scenario_bootstrap.rs:1816-1822` reproduces named-House `IQ=` default/above-max-to-one.
- `src/sim/scenario_bootstrap.rs:1070-1075` stamps nonhuman noncampaign Houses from
  `MaxIQLevels`.
- `src/sim/house_state.rs:222-226` owns three independent persistent neighboring latches;
  deploy writes Production/AITriggersActive/AutoBase in native deploy order.
- `src/sim/world/mod.rs:7188-7224` advances factories/production before
  `run_late_region`; `run_late_region` begins the House tail and performs defeat/AI work.
- `src/sim/world/world_hash.rs:603-608` correctly excludes AutoBaseBuilding from the direct hash.

### Missing or wrong

1. `GeneralRules` parses `MaxIQLevels`, `RepairSell`, and `SellBack`, but no signed
   `[IQ] Production` field/default.
2. `HouseAiActivationLatches` has no `autocreate_allowed` fourth field; its comment currently maps
   only `+0x1EE/+0x1F2/+0x1F3`.
3. No call site reads `ai_activation.auto_base_building` or `current_iq` to run this activation.
4. `run_late_region` starts with vision reconciliation and then defeat; there is no ordered
   House activation pass between early House-like work and defeat/AI.
5. Snapshot version 108 round-trips only eight combinations of the three existing latches.
6. `world_hash` includes Production and AITriggersActive only; it omits native-direct
   AutocreateAllowed.
7. `trigger_runtime.rs` does not dispatch action 13.
8. TeamType `autocreate` is loaded but should remain separate; using it as the missing House byte
   would be architecture and evidence drift.

## Exact bounded implementation partition

### Partition A — mandatory House-update activation row

1. Add signed `iq_production` (name may follow local convention) to the `[IQ]` rules owner:
   constructor/default 5, effective INI `Production` override, no clamp.
2. Add `autocreate_allowed: bool` to `HouseAiActivationLatches`, default false. Preserve existing
   three fields and deploy helper behavior; deploy must still not set it directly.
3. Add one House-owned transition helper with the exact control/AutoBase/IQ gates and literal-one
   stores in native order: AutoBase, Production, Autocreate. It must not touch AITriggersActive.
4. Visit Houses in `ScenarioSession.house_order` at the House-tail point after the factory/
   production sweep and early House-like reconciliation, before defeat and AI command generation.
   Do not route this through `AiPlayerState`; neutral/special/non-AI-player Houses are still House
   array members and must be evaluated.
5. Extend snapshot serialization and bump the current schema once. Round-trip all 16 combinations
   of the four independent booleans.
6. Extend the House hash with AutocreateAllowed between Production and AITriggersActive; keep
   AutoBase excluded directly. Preserve CurrentIQ hashing.
7. Do not admit offline modal frames for this transition. Preserve the current mode-0/5 freeze.
   If future network-modal simulation is enabled, it must run this transition only as part of the
   complete native-equivalent PerTick/House update and late frame tail, not as a selective pass.

### Partition B — alternate compiled writer

Implement trigger action 13 as a small separate trigger-runtime writer once the shared exact House
resolver is available: null target -> failure/no mutation; nonnull -> set only
`autocreate_allowed`, success. Its absence from shipped maps is sufficient to keep it out of the
ordinary stock-skirmish critical path, but not sufficient to claim full compiled/custom action
parity.

Production's factory consumer, AutoBase's Unit AI/Guard consumers, action 30, AITriggersActive,
computer takeover, and MCV deploy behavior remain owned by their existing parent rows. They must
use these shared House latches, not duplicate state.

## Acceptance scenarios

| ID | Scenario | Required result |
|---|---|---|
| A1 | fresh House | all four latches false; CurrentIQ zero |
| A2 | noncontrolled, AutoBase false, IQ one below threshold | no field changes |
| A3 | noncontrolled, AutoBase false, IQ exactly threshold | AutoBase then Production then Autocreate true; AITriggers unchanged |
| A4 | noncontrolled, IQ above threshold | same as A3 |
| A5 | noncontrolled, AutoBase true, IQ far below threshold | same as A3; proves bypass |
| A6 | campaign CurrentPlayer true | skip even with AutoBase/high IQ |
| A7 | campaign PlayerControl true but not CurrentPlayer | skip |
| A8 | nonzero mode, PlayerControl true but CurrentPlayer false | PlayerControl ignored; activate if latch/IQ qualifies |
| A9 | nonzero mode CurrentPlayer true | skip |
| A10 | repeated eligible visits | idempotent final state, no RNG/counter/timer change |
| A11 | action-30-like clear after split state, IQ below threshold | AutoBase remains false; Production/Autocreate remain independently true |
| A12 | same clear with IQ equal/above threshold | next House visit restores AutoBase and rewrites the other two |
| A13 | deploy state `{Production=1, AITriggers=1, AutoBase=1, Autocreate=0}` | next eligible House visit sets only missing Autocreate semantically; AITriggers retained |
| A14 | computer-takeover high IQ with/without AutoBase | next eligible visit sets Autocreate |
| A15 | threshold `-1`, CurrentIQ `0` | activate; proves signed/unclamped rule input |
| A16 | threshold `6`, generated stock-like AI IQ `5`, AutoBase false | no activation |
| A17 | forward `house_order` with human, AI, neutral | every House evaluated once; only eligible noncontrolled members change |
| A18 | factory completes immediately before House tail | activation still runs after factory sweep and before defeat/AI |
| A19 | House becomes defeated later on same visit | activation precedes defeat processing |
| A20 | offline mode-0/5 Menu, Abort Confirm, or Options remains open across service pumps | no activation, world/session frame advance, trigger polling, House AI, or pending-delete drain; activation resumes on the next ordinarily admitted frame after close |
| A20N | eligible future network modal reenters Main Tick | the full PerTick/House update runs, including activation, triggers, later House AI, command tail, frame commit, and pending-delete drain; no activation-only subset |
| A21 | snapshot all 16 latch combinations | exact round trip under bumped schema |
| A22 | hash delta | toggling Production, Autocreate, or AITriggers changes hash; toggling only AutoBase does not |
| A23 | action 13 null/non-null fixture | null no-op/failure; nonnull sets only Autocreate/success |
| A24 | TeamType `Autocreate=yes/no` with House byte varied | no invented coupling |

Focused implementation validation should use scoped `cargo test -p vera20k --lib <filter>` while
working; the parent run owns the one final full `cargo test -p vera20k --lib` certification.

## Adversarial review questions

1. **Could `CMP/JL` be unsigned or strict-greater?** No. `JL` is signed-less, so equality activates.
2. **Could AutoBase merely skip this mechanism?** No. Its nonzero branch jumps directly to the
   three stores, bypassing the IQ load/compare.
3. **Could AutocreateAllowed secretly gate TeamType autocreation through an indirect named
   consumer?** No direct field operand exists beyond CRC, and the complete 1.16M-instruction census
   is untruncated. Raw save/load is the only indirect access found.
4. **Could controlled Houses activate through deploy-set AutoBase?** No. The control branch occurs
   first and jumps beyond all three stores.
5. **Does `g_GameState != 0` inside `Main_Tick` prove that offline ESC/Options executes this pass?**
   No. Offline states 1/3/5 are blocking pump owners, and modes 0/5 never call `Main_Tick` from
   that pump. The internal branch applies only after a caller admits Main Tick, notably an eligible
   network modal; such an admitted tick executes the complete PerTick, not this block alone.

All five adversarial questions are resolved by direct evidence; none remains open.

## Tiny-details ledger

1. `PlayerControl` is read only when `GameMode == 0`.
2. `CurrentPlayer` is read in every mode.
3. Any nonzero control byte is true; no `==1` test is used.
4. Any nonzero AutoBase byte is true.
5. The IQ comparison is signed.
6. Equality activates.
7. AutoBase bypasses both Rules pointer use and IQ comparison.
8. The block performs no call and consumes no RNG.
9. The three stores have no intervening branch.
10. AITriggersActive is not one of the three stores.
11. Repeated visits rewrite literal one; there is no “already fully active” early-out.
12. A controlled House skips even if AutoBase is nonzero.
13. Constructor CurrentIQ and all four reachable latch states start at zero/false.
14. House `IQ=` above MaxIQLevels becomes one rather than max.
15. Rules Production has an independent default even when `[IQ]` is absent.
16. Stock skirmish AI reaches equality because Create_Houses uses MaxIQLevels.
17. `+0x1EF` has no post-constructor zero writer.
18. CRC normalizes its byte as boolean, but raw save/load preserves it.
19. AutoBase is persisted yet omitted from direct House CRC.
20. Trigger action 13 changes only AutocreateAllowed.
21. Trigger action 13 is absent from the 310 effective shipped map payloads checked.
22. `aimd.ini` TeamType `Autocreate=` is not the House byte.
23. House count is reloaded after each native House callback.
24. The next instruction after the transition begins a distinct `+0x1F7` result block.
25. Offline modal exclusion is a caller-level pump decision, not a House-update pause predicate.
26. States 1, 3, and 5 all own blocking pump loops in active retail.
27. An admitted network-modal Main Tick runs trigger polling and the rest of House AI alongside
    this transition; native provides no selective activation-only modal path.

## Coverage Ledger

| Surface | Evidence method | Result | Status |
|---|---|---|---|
| activation `0x004F8564..0x004F85B7` | live assembly + full decompile | exact gates/order | complete |
| control-mode matrix | live assembly | campaign/nonzero behavior exact | complete |
| signed IQ boundary | live assembly | `JL`, equality passes | complete |
| position in House update | live full-function decompile | before result/defeat/AI tail | complete |
| House scheduler owner | live PerTick assembly + vtable bytes | forward House array, slot `+0x5C` | complete |
| modal owner/caller matrix | live `Main_Game`, `State_Machine`, states 1/3/5 owners, and modal-pump decompile | offline 0/5 no Main Tick; eligible network modal full PerTick | complete |
| House constructor | live assembly/census | latch/IQ defaults | complete |
| Rules constructor | live assembly | Production default 5 | complete |
| Rules `[IQ]` parser | live assembly/string bytes | signed no-clamp override | complete |
| House `IQ=` parser | live assembly | default 0, above-max -> 1 | complete |
| nonzero-mode computer IQ | live Create_Houses assembly | MaxIQLevels stamp | complete |
| `+0x1EF` all direct accesses | 1.16M-instruction operand census | 4 real, 2 false constants | complete |
| trigger action 13 | live dispatch/resolver decompile | nonnull set-only writer | complete |
| `+0x1EE` relevant accesses | whole-program operand census + consumer decompile | writers, CRC, factory gates | complete |
| `+0x1F3` all direct accesses | whole-program operand census | writers/readers/false constant split | complete |
| save/load/size | live decompile/assembly | raw-block persistence | complete |
| CRC | live raw-boundary assembly + bool-helper decompile | Production/Autocreate/AITriggers in; AutoBase out | complete |
| installed rules data | direct `ini/rules*.ini` scan | both Production 5 | complete |
| shipped map action/data use | six-archive effective map extraction/parser census | 310 maps, action13 zero, IQ range known | complete |
| TeamType name collision | installed `aimd.ini` + executable negative census | distinct mechanism | complete |
| Rust House/rules/tick | direct source scan | exact matches/gaps enumerated | complete |
| Rust snapshot/hash | direct source scan | v108 three-bit state, missing Autocreate | complete |
| Rust modal admission | direct source scan vs native caller matrix | current offline freeze matches; future NetworkModal must retain full-PerTick semantics | complete |
| zero-add pass | repeated `+0x1EF/+0x1EE/+0x1F3` census and nearby CRC/action checks | no new material surface | complete |
| cold spot 1 | decompile `CRCEngine__AddBool` | nonzero normalization proven | complete |
| cold spot 2 | decompile action-13 House resolver | null/special/general branches proven | complete |

Coverage deferrals: **0/26 (0%)** inside the stated exhaustive slice.

## Open Questions — final state

Initial question set and disposition:

1. exact control predicate — resolved;
2. campaign versus nonzero mode use of PlayerControl — resolved;
3. AutoBase short-circuit direction — resolved;
4. signedness and equality boundary — resolved;
5. store order and fields touched — resolved;
6. repeated/idempotent behavior — resolved;
7. passive/defeated/power/timer/RNG gates — resolved absent;
8. PerTick owner and House visit order — resolved;
9. position relative to factories/defeat/AI — resolved;
10. pause/modal/replay activation — resolved;
11. constructor and parser defaults — resolved;
12. Rules/House override and clamp behavior — resolved;
13. every `+0x1EF` reader/writer — resolved exhaustive;
14. relevant `+0x1EE/+0x1F3` interactions — resolved;
15. save/load/CRC behavior — resolved;
16. stock YR rule/map/script activation — resolved;
17. TeamType `Autocreate` relationship — resolved as no direct coupling;
18. TS/custom boundary — resolved;
19. current Rust owner/tick/state delta — resolved;
20. bounded implementation and tests — resolved.

**Open:** 0. **Deferred inside scope:** 0. **Unverified:** 0. **Approximate:** 0.

## Ghidra annotation candidates

No metadata was changed. Existing labels for `HouseClass__Update`, `LogicClass__PerTickUpdate`,
`TriggerAction__Execute`, save/load, and CRC helpers were adequate. The raw House CRC boundary
`0x00502D60..0x0050303F` remains a structural create-function candidate, but creating it was not
authorized for this research-only run and is not required to implement the mechanism.

## Sources

### Live active-retail Ghidra

- `HouseClass__Update @ 0x004F8440`, especially `0x004F8564..0x004F85BE`
- `Main_Game @ 0x0048CCC0`, `State_Machine @ 0x0048C8B0`
- Menu `FUN_004F10E0`, Abort Confirm `FUN_004F1840`, Options `0x004E1D00`
- `ProcessModalServicePump @ 0x00623120`, `Main_Tick @ 0x0055D360`
- `LogicClass__PerTickUpdate @ 0x0055AFB0`, House loop `0x0055B68D..0x0055B6B1`
- House vtable entry `0x007EA8FC -> 0x004F8440`
- `HouseClass__Constructor @ 0x004F56D0`
- `HouseClass__Read_Scenario_INI @ 0x00500B40`
- `ScenarioClass__Create_Houses @ 0x00687F10`
- `HouseClass__ComputerTakeover @ 0x0050A5C0`
- `RulesClass` constructor `0x00665EB0`, initialization `0x006671A1..0x006671C1`
- `RulesClass__ReadIQ @ 0x00674240`
- `TriggerAction__Execute @ 0x006DD8B0`, case 13 `0x006DEB18..0x006DEB54`
- shared House resolver `FUN_006E45E0`
- `FUN_004500F0` Production consumer
- `HouseClass__Save @ 0x00504080`, `HouseClass__Load @ 0x00503040`, SizeOf `0x00504730`
- raw House CRC `0x00502D60..0x0050303F`
- `CRCEngine__AddBool @ 0x004A1CA0`

### Repository/data

- `ENGINE.md`, `AGENTS.md`, `.agents/skills/re-investigate/SKILL.md`
- `docs/research/ghidra-workflow.md`
- the two parent Phase-3 reports named in Scope
- `docs/research/MODAL_PUMP_00623120_SERVICE_TICK_CONTRACT_GHIDRA_REPORT.md`
- `docs/research/FRAME_COUNTER_NONADVANCE_PAUSE_SCENARIO_MATRIX_GHIDRA_REPORT.md`
- installed `ini/rules.ini`, `ini/rulesmd.ini`, `ini/aimd.ini`
- direct Rust scans of `src/rules/ruleset.rs`, `src/sim/house_state.rs`,
  `src/sim/scenario_bootstrap.rs`, `src/sim/world/mod.rs`, `src/sim/world/world_hash.rs`,
  `src/sim/snapshot.rs`, `src/sim/trigger_runtime.rs`, `src/app/match_runtime/sim_tick.rs`
- read-only `asset.exe` mounted-map extraction/census; all temporary extraction files were removed.
