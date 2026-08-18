# Grizzly OpportunityFire Consumer - Ghidra Research Report

**Address(es):** `0x00709290` primary runtime gate, `0x00709489` passive-acquire caller, `0x006FA6AE` `TechnoClass__AI_Update` caller, `0x0071483D` parser write  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** active stock-YR consumer of `TechnoTypeClass+0x6AF OpportunityFire` for a normal MTNK/Grizzly move-or-guard auto-target scan.  
**Non-Scope:** target scoring internals behind vtable `+0x39C`, complete combat damage, projectile flight, all mission enum variants, and non-MTNK special weapons.  
**Confidence:** High for the gate and callers; Medium for exact target-scanner internals because vtable `+0x39C` was treated as out of scope.  
**Active in YR:** Yes.

## Target Question

What active stock-YR code consumes `TechnoTypeClass+0x6AF OpportunityFire` for Grizzly/MTNK, and when does it let a moving or idle/guarding Grizzly auto-acquire a target without an explicit attack order?

## Non-goals

- Full target scoring behind vtable `+0x39C`.
- Full combat damage, projectile motion, warhead effects, and burst timing.
- Full mission enum recovery beyond the mission ids directly observed in the consumer chain.
- Any Rust implementation or patch.

## Evidence Needed to Mark COMPLETE

- Parser proof that `[MTNK] OpportunityFire=yes` maps to `TechnoType+0x6AF`.
- Runtime binary read of `TechnoType+0x6AF`, not the instance byte at object `+0x6AF`.
- Caller proof that the reader is active in standard YR AI/passive acquisition paths.
- Source scan showing the Rust parser/runtime gap and a concrete acceptance scenario.

## Stop Conditions

- Stop after the first active consumer chain is proven with decompile plus bytes/xrefs.
- Stop before entering target scoring internals behind vtable `+0x39C`.
- Stop before damage/projectile/fire animation internals.
- Stop if further proof requires live runtime frame tracing; record it as remaining uncertainty.

## 1. Overview

`OpportunityFire=yes` is not a direct "fire now" flag. It is a permissive gate in `FUN_00709290` that allows passive target acquisition while the object is on non-attack missions such as `MISSION_MOVE` (`2`) and `MISSION_GUARD` (`5`). For stock Grizzly (`[MTNK] OpportunityFire=yes`, `Primary=105mm`, `Range=5`, `Turret=yes`), the flag lets the normal Techno AI path run the target scanner while the tank is moving or guarding without a player attack order.

The actual shot is still normal combat: the gate calls the normal target-acquisition virtual (`vtable+0x39C`) and any later fire goes through the normal mission/combat/turret path. There is no MTNK-specific branch and no damage/projectile special case here.

## 2. Class Layout / Key Offsets

| Class | Offset | Type | Meaning | Evidence | Active in YR |
|---|---:|---|---|---|---|
| `TechnoTypeClass` | `+0x6AF` | byte/bool | `OpportunityFire` | parser bytes at `0x00714836..0x00714850`; runtime read at `0x007093DF..0x007093E5` | Yes |
| `TechnoClass`/derived foot object | `+0x2B4` (`param_1[0xAD]`) | object ptr | current target / TarCom | `FUN_00709290`, `TechnoClass__Passive_Target_Acquire`, `TechnoClass__AI_Update` | Yes |
| `MissionClass` subobject | `+0xAC` (`param_1[0x2B]`) | int | current mission id | `TechnoClass__AI_Update` compares `2`, `10`, `5`; `FUN_00709290` compares `2` and `5` | Yes |
| `TechnoClass`/derived | `+0x4FC` (`param_1[0x13F]`) | frame | last passive target scan frame | written before vtable `+0x39C` in `0x00709489` and `0x006FA6AE` path | Yes |
| `TechnoClass`/derived | `+0x50C` (`param_1[0x143]`) | byte | target changed during passive acquisition | `TechnoClass__Passive_Target_Acquire` sets when target pointer changes | Yes |
| `TechnoTypeClass` | `+0xD99` | byte/bool | base eligibility flag required by `FUN_007091D0` | `FUN_007091D0` reads type via vtable `+0x84`, rejects when zero | Conditional |

Do not confuse `TechnoTypeClass+0x6AF` with the instance byte at object `+0x6AF`. The instance byte is used in Unit facing/radio/scatter logic (`UnitClass__Facing_Update`, `UnitClass__Receive_Radio`, `FootClass__Receive_Radio`, `UnitClass__Scatter`) and is not the INI `OpportunityFire` flag.

## 3. Core Logic

### Parser

`TechnoTypeClass__ReadINI` reads the existing byte at `this+0x6AF`, pushes string `0x00843A74` (`OpportunityFire`), calls `CCINIClass__ReadBool`, and writes the returned byte back to `this+0x6AF`.

Evidence bytes at `0x00714836..0x00714850`:

- `8A 95 AF 06 00 00`: load default byte from `+0x6AF`.
- `68 74 3A 84 00`: push `OpportunityFire` string address.
- call `ReadBool`.
- `88 85 AF 06 00 00`: store bool to `+0x6AF`.

Default constructor initializes `TechnoType+0x6AF` to zero at `TechnoTypeClass__Constructor`.

### Runtime Gate - `FUN_00709290`

`FUN_00709290` returns whether passive acquisition may run.

Key behavior:

1. A special non-player passenger/transport-style branch can return true before the common gate. This is active code but not relevant to stock MTNK in normal player movement.
2. It calls `FUN_007091D0`; if that returns false, the passive acquire is rejected before `OpportunityFire` is examined.
3. If current mission is `2` (`MISSION_MOVE`), some special subcases can return true before the flag check.
4. It calls the type getter (`vtable+0x84`) and reads `TechnoType+0x6AF`.
5. If `OpportunityFire` is zero and current mission is not `5` (`MISSION_GUARD`), return false.
6. If `OpportunityFire` is zero and current mission is `5`, the function does a weapon/current-target exception and can still allow guard acquisition.
7. If `OpportunityFire` is nonzero, the function reaches the final `return 1` for the common eligible path.

Evidence bytes at `0x007093DF..0x007093F0`:

- `8A 88 AF 06 00 00`: `mov cl, byte ptr [eax+0x6AF]` after the type getter.
- `84 C9`: test flag.
- `75 66`: nonzero flag jumps to the allow tail.
- `83 BE AC 00 00 00 05`: compare current mission at object `+0xAC` to `5`.
- `74 06`: mission 5 gets the guard exception, otherwise immediate false path.

### Base Eligibility - `FUN_007091D0`

Before `OpportunityFire` can matter, `FUN_007091D0` requires the object to pass general "may passively acquire" checks:

- Rejects if virtual `+0x1DC` returns true.
- Rejects if object `+0x2DC` (`param_1[0xB7]`) is nonzero.
- Rejects if `type+0xD99` is false.
- Has a building/special object conditional branch for abstract type `6`.
- Rejects player-controlled objects when virtual `+0x330` returns true.
- Finally requires virtual `+0x2AC` to return true when not in the capture-manager exception.

This report does not name all of those virtuals because the scope is the `OpportunityFire` consumer, not the full eligibility policy.

## 4. INI Keys

| Section | Key | Stock value | Binary read | Effect in this slice | Active in YR |
|---|---|---|---|---|---|
| `[MTNK]` | `OpportunityFire` | `yes` | `TechnoTypeClass__ReadINI @ 0x0071483D`, writes `+0x6AF` | Lets passive acquire run on move/non-guard missions after base eligibility passes | Yes |
| `[MTNK]` | `Primary` | `105mm` | settled prior work; weapon lookup outside this slice | Provides the weapon the scanner/fire path can use | Yes |
| `[105mm]` | `Range` | `5` | weapon parser outside this slice | Normal target range used by scanner/fire checks | Yes |
| `[MTNK]` | `Turret` | `yes` | settled prior work `TechnoType+0xCA1` | Later aiming/firing uses turret path; not an `OpportunityFire` gate | Yes |

## 5. Integration Points

### `TechnoClass__AI_Update @ 0x006F9E50`

The regular Techno AI tick calls the gate late in the tick:

1. `MissionClass__Mission_Dispatch()` runs first.
2. A scan-timer block checks a remaining-time field pair (`+0x180/+0x188` in decompiler terms). If the timer has not expired, no passive scan.
3. Virtual `+0x4C4` must return false; otherwise virtual `+0x4CC` runs instead.
4. Current mission must be `2`, `10`, or `5`.
5. It calls `FUN_00709290`.
6. If allowed, it writes current frame to `+0x4FC`, obtains current coords via virtual `+0x48`, and calls virtual `+0x39C` with the coordinate. If target changes, it sets `+0x50C`.

Evidence bytes at `0x006FA699..0x006FA6C1`: compares current mission to `2`, `10`, `5`, calls `0x00709290`, then writes `g_CurrentFrameCounter` and dispatches `vtable+0x39C`.

### `TechnoClass__Passive_Target_Acquire @ 0x00709480`

This helper directly wraps the same gate:

1. Calls `FUN_00709290`.
2. If false, returns false.
3. Saves old target from `+0x2B4`.
4. Writes current frame to `+0x4FC`.
5. Calls virtual `+0x39C` with current coords.
6. If the target pointer changed, sets byte `+0x50C`.

Evidence bytes at `0x00709488..0x007094AD`: `call 0x00709290`, `test al,al`, old target load from `+0x2B4`, current-frame write to `+0x4FC`, and virtual dispatch setup.

### `FootClass__Mission_AreaGuard @ 0x004D6AA0`

Area guard uses the base eligibility helper (`FUN_007091D0`) and another scanner helper (`FUN_0070F7E0`) when it has no current target. This confirms the common eligibility helper is not dead TS code. The specific `OpportunityFire+0x6AF` read is in `FUN_00709290`, not this mission body.

## 6. Current Rust Implementation Status

Rust currently parses MTNK combat basics (`Primary`, `Turret`, `GuardRange`, weapon ranges), but `ObjectType` has no `opportunity_fire` field and `ObjectType::from_ini_section` does not parse `OpportunityFire`.

Current auto-acquisition exists in two places:

- `src/sim/world/world_orders.rs::tick_order_intents_pre_combat`: only entities with persistent `OrderIntent` (`AttackMove`, `Guard`) and no attack target acquire.
- `src/sim/combat/combat_targeting.rs::acquire_best_target_for_entity`: selects a target by weapon compatibility and range.
- `src/sim/combat/mod.rs`: retargets during combat and pursuit, but it does not model the YR passive scan gate keyed by `OpportunityFire`.

Current mismatch: a Grizzly issued an ordinary move without `OrderIntent::AttackMove` will not auto-acquire simply because `[MTNK] OpportunityFire=yes`; gamemd can run the passive acquire gate on `MISSION_MOVE`.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `OpportunityFire` parser | verified | `0x00714836..0x00714850`, string `0x00843A74`, constructor default zero | none |
| `FUN_00709290` runtime gate | verified | decompile `0x00709290`, bytes `0x007093DF..0x007093F0` | exact names of unrelated virtuals deferred |
| `FUN_007091D0` base eligibility | verified for dependency shape | decompile `0x007091D0`, xrefs from `0x004D6EDB` and `0x007092F4` | exact virtual names deferred |
| `TechnoClass__AI_Update` caller | verified | decompile `0x006F9E50`, bytes `0x006FA699..0x006FA6C1` | exact timer field naming deferred |
| `TechnoClass__Passive_Target_Acquire` caller | verified | decompile `0x00709480`, bytes `0x00709488..0x007094AD` | external caller context `TeleportLocomotionClass__TimerCheck` not followed |
| vtable `+0x39C` target scanner | touched-not-exhausted | calls from `0x006FA6AE` and `0x007094A?` | target scoring intentionally out of scope |
| Unit instance `+0x6AF` hits | verified as negative fact | `UnitClass__Facing_Update`, `UnitClass__Receive_Radio`, `FootClass__Receive_Radio`, `UnitClass__Scatter` | none for this INI key |
| Rust parser/runtime | verified by source scan | `src/rules/object_type.rs`, `src/sim/world/world_orders.rs`, `src/sim/combat/combat_targeting.rs` | implement later |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-1 - Where is `OpportunityFire` parsed? -> `TechnoTypeClass__ReadINI` reads string `0x00843A74` and writes `TechnoType+0x6AF`.` (evidence: `0x00714836..0x00714850`)
- `[RESOLVED] OQ-2 - Is `+0x6AF` default off? -> constructor writes zero to `TechnoType+0x6AF`.` (evidence: `TechnoTypeClass__Constructor @ 0x00710AF0`)
- `[RESOLVED] OQ-3 - What runtime function reads `TechnoType+0x6AF`? -> `FUN_00709290`.` (evidence: decompile `0x00709290`, bytes `0x007093DF..0x007093F0`)
- `[RESOLVED] OQ-4 - Is the reader active in YR? -> Yes, direct caller from `TechnoClass__AI_Update`.` (evidence: xref `0x006FA6AE`)
- `[RESOLVED] OQ-5 - Does a passive helper also use it? -> Yes, `TechnoClass__Passive_Target_Acquire` calls the same gate.` (evidence: xref `0x00709489`)
- `[RESOLVED] OQ-6 - Does moving matter? -> Yes, `TechnoClass__AI_Update` only enters this block for missions `2`, `10`, or `5`; docs identify `2` as Move and `10` as Harvest.` (evidence: `0x006FA699..0x006FA6B5`; `UNIT_CLASS_SCATTER_GHIDRA_REPORT.md`; `AUDIT_LOG.md`)
- `[RESOLVED] OQ-7 - Does idle/guard matter? -> Yes, the gate has a special mission `5` path when `OpportunityFire` is false, and `OpportunityFire=yes` reaches the allow tail without that restriction.` (evidence: `0x007093DF..0x007093F0`)
- `[RESOLVED] OQ-8 - Does the flag itself score targets? -> No, it gates whether the scanner virtual runs; scoring is behind vtable `+0x39C`.` (evidence: `0x006FA6AE`, `0x007094A?`)
- `[RESOLVED] OQ-9 - Does this fire immediately in the same function? -> No direct shot/damage call in this slice; it sets/acquires target, then normal mission/combat paths fire.` (evidence: decompile `0x00709290`, `0x00709480`)
- `[RESOLVED] OQ-10 - Is there an MTNK hardcoded branch? -> No branch on MTNK/Grizzly id found in this gate.` (evidence: decompile `0x00709290`)
- `[RESOLVED] OQ-11 - Are UnitClass instance `+0x6AF` reads the same flag? -> No, those are object-instance state reads/writes, not type reads via vtable `+0x84`.` (evidence: decompile `0x00736990`, `0x00737430`, `0x004D90A7`, `0x00743CF0`)
- `[RESOLVED] OQ-12 - Does Rust parse `OpportunityFire`? -> No matching field or parser key found.` (evidence: source scan `src/rules/object_type.rs`)
- `[RESOLVED] OQ-13 - Does Rust run passive acquire for ordinary move? -> No; target acquisition is tied to `OrderIntent` and combat retarget paths.` (evidence: `src/sim/world/world_orders.rs`, `src/sim/combat/combat_targeting.rs`)
- `[DEFERRED] OQ-14 - What exact target ranking does vtable `+0x39C` use?` (category: out-of-scope; reason: target scoring explicitly excluded; next-step-if-pursued: investigate scanner virtual and `GreatestThreat` style helpers)
- `[DEFERRED] OQ-15 - Exact names of all virtuals in `FUN_007091D0`?` (category: bounded-cost-too-high; reason: not needed to prove `OpportunityFire` consumer; next-step-if-pursued: vtable-slot naming pass)
- `[DEFERRED] OQ-16 - Does a newly acquired target fire in the same frame or next mission dispatch?` (category: needs-runtime-debugger; reason: static order shows AI passive scan after `Mission_Dispatch`, but runtime interaction with timers should be measured; next-step-if-pursued: frame trace a moving MTNK passing an enemy)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `OpportunityFire=yes` is a TechnoType bool at `+0x6AF`, default false, parsed from object section. | `0x00714836..0x00714850`, constructor `0x00710AF0` | missing | `src/rules/object_type.rs`, `src/rules/ruleset.rs` tests | Add data field and parse `OpportunityFire`, default false. | MTNK parses `opportunity_fire=true`; a type with missing key parses false. | Do not infer from `Turret=yes`; non-turret content can also set the key. |
| Passive acquisition on `MISSION_MOVE` is allowed by `OpportunityFire=yes`; without it, non-guard missions fail the common gate. | `FUN_00709290`, `0x007093DF..0x007093F0`; AI caller `0x006FA699..0x006FA6C1` | missing/mismatch | `src/sim/world/world_orders.rs` or a new sim passive-acquire tick near combat/movement orchestration | Ordinary move for MTNK should periodically scan and set `attack_target` while preserving movement intent unless combat/pursuit needs to pause. | Moving Grizzly with no explicit attack order passes an enemy within 5 cells and acquires/fires. | Do not require `OrderIntent::AttackMove`; gamemd path is ordinary mission move plus flag. |
| Guard/idle acquisition is not uniquely caused by `OpportunityFire`; mission `5` has its own exception, while Grizzly's `yes` simply also passes the flag gate. | `FUN_00709290` mission `5` branch | partial | `src/sim/world/world_orders.rs`, `src/sim/combat/combat_targeting.rs` | Keep guard auto-acquire behavior, but do not use guard behavior alone as proof that `OpportunityFire` is implemented. | Idle/guard Grizzly acquires a hostile in range; a no-OpportunityFire guard-capable armed unit may still acquire under guard rules. | Do not make `OpportunityFire=false` mean "never auto-acquire while guarding". |

### Stale Docs / Follow-up Docs

Replace the stale MTNK wording:

> `OpportunityFire` consumer in the auto-targeting/scan path - offset verified, runtime gating function DEFERRED.

with:

> `OpportunityFire=yes` is consumed by `FUN_00709290`, called from `TechnoClass__AI_Update` and `TechnoClass__Passive_Target_Acquire`. The gate reads `TechnoType+0x6AF`; when true, eligible units on `MISSION_MOVE`/`MISSION_HARVEST`/`MISSION_GUARD` can run the passive target scanner (`vtable+0x39C`) without an explicit attack order. For Grizzly, this enables ordinary move-by opportunistic acquisition; firing still uses the normal target, weapon, and turret paths.

## Sources

- Ghidra decompiled/read: `0x0071483D`, `0x00710AF0`, `0x007091D0`, `0x00709290`, `0x00709480`, `0x006F9E50`, `0x004D6AA0`, `0x00736990`, `0x00737430`, `0x004D90A7`, `0x00743CF0`.
- Byte evidence: `0x00714836..0x00714850`, `0x007093DF..0x007093F0`, `0x00709488..0x007094AD`, `0x006FA699..0x006FA6C1`, `0x004D6EC8..0x004D6EEF`.
- INI checked: `ini/rulesmd.ini` `[MTNK]`, `[105mm]`; `ini/rules.ini` base fallback.
- Rust scanned: `src/rules/object_type.rs`, `src/rules/ruleset.rs`, `src/sim/world/world_orders.rs`, `src/sim/combat/combat_targeting.rs`, `src/sim/combat/mod.rs`, `src/sim/components.rs`.
- Prior docs referenced: `C:/Users/enok/Documents/ra2-rust-game-docs/units/allied/MTNK.md`, `UNIT_CLASS_SCATTER_GHIDRA_REPORT.md`, `AUDIT_LOG.md`.
