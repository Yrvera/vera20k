---
title: Disparity Scan - Trigger Action 119 and House destruction sweep
date: 2026-08-29
scope: Active-YR House destructive result sweep, its three callers, and retail Action 119 data
methodology: docs-first discovery, direct Rust verification, active-binary verification, mounted-retail census
---

# Disparity Scan - Trigger Action 119 and House destruction sweep

## Scope and evidence basis

This scan covers the active function currently mislabeled
`HouseClass__ScatterAllUnits @ 0x004FC6D0`, its callers in
`HouseClass__Update @ 0x004F8440` and Trigger Action 119, the House operand
resolver, effective ownership, reversible mind-control preservation, Temporal
detachment, C4 receiver dispatch, and mutation-safe global Techno traversal.
It does not reopen ordinary mission Scatter (`Techno` virtual `+0x174`), the
rest of `MPlayer_Defeated`, unrelated Trigger actions, or custom-map-only
registry operand formats absent from the mounted retail corpus.

Native evidence was read from the active `gamemd.exe` program. Retail evidence
was counted from all 184 mounted `.map`/`.mpr` files under
`target/phase3-retail-census/extract`. Current Rust was read directly at
committed HEAD `ba7bf4a3`.

## Summary

- 8 behavior candidates inventoried
- 8 active-YR claims verified
- 3 coupled implementation gaps verified
- 0 doc-derived candidates awaiting verification
- 5 prerequisite mechanisms verified present
- 2 evidence-backed active-retail exclusions

This report is a dated disparity snapshot and implementation handoff, not a
phase completion certificate.

> **Post-implementation registry correction (2026-08-29):** the first repair
> incorrectly mapped the global Techno array to Rust's `LogicVector` via
> `Simulation::tactical_registration_order`. Active assembly reads
> `g_TechnoClass_Array` at `0x004FC6EC` and reloads
> `g_TechnoClass_Count` at `0x004FC771`; Conceal/limbo does not remove a
> Techno from that class registry. The repair now uses the independent alive
> `EntityStore` construction-order projection (`techno_registration_len` /
> `techno_registration_id_at`). Stable IDs are monotonic construction
> identities, every `GameEntity` is one of the four Techno analogues, UnInit
> clears `object_alive` synchronously, and deferred physical deletion does not
> keep a dead tombstone in the projected registry. This note supersedes the
> dated prerequisite and implementation wording below wherever it names
> `LogicVector` as the House sweep source.

## Verified gaps

### 1. The shared House destruction sweep is absent

`0x004FC6D0` walks the live global Techno array in forward registration order.
For each Techno whose effective owner is the target House, it may rewrite a
reversible mind-control node's saved original owner to the resolved Civilian
House and spare the victim. Otherwise it detaches the victim's incoming
Temporal chain and calls concrete `ReceiveDamage` (virtual `+0x16C`) with a
stack copy of current health, distance zero, `Rules+0xFA8` C4Warhead, null
attacker, and receiver flags `(0,1,1,0)`.

The loop deliberately does not increment its live-array index after a receiver
call. It remembers that pointer: synchronous removal exposes the compacted next
entry at the same index, while a surviving receiver is recognized on the next
iteration and advances once. At scanned HEAD, Rust had the synchronous
damage/lifecycle path but only exposed `LogicVector` order; that is not the
independent class registry required by this operation.

### 2. Trigger Action 119 is unsupported

`TriggerAction__Execute @ 0x006DD8B0` case `0x77` calls `FUN_006E3180`.
The call at `0x006DF972` passes House, Object, Trigger, and Cell; after the
callee's leading push, its `[ESP+0x10]` guard is the `TriggerClass*`, not the
cell. A null Trigger returns false. Operand `0x2325` resolves
`TriggerClass+0x2C` (raising House); `-1` returns false; `0x117B..0x1182`
resolve start slots A..H; all other signed values resolve the first House whose
country index matches. A missing House returns false; a resolved House runs
`0x004FC6D0` and returns true.

Rust's typed `TriggerTransaction` preserves per-instance `raising_house`,
`ScenarioSession.start_slot_houses`, House registration order, materialized
numeric operands, and native boolean results, but `execute_action` has no case
119 and does not receive the current instance identity needed by `0x2325`.

### 3. Both active House-update callers omit the sweep

`HouseClass__Update` calls `0x004FC6D0` at two distinct sites:

- `0x004F87C2..0x004F87FA`: when pending-result byte `House+0x1F6` is set and
  the shared signed timer reaches exact zero, clear the byte and immediately
  run the destruction sweep;
- `0x004F8E86..0x004F8F82`: in nonzero game mode, for a non-defeated,
  non-passive House after frame zero whose native defeat count is zero, run the
  sweep and then `MPlayer_Defeated`.

Rust `check_defeat` discards `HouseResultAdvance.pending_expired`, never runs a
sweep, and says surviving units scatter/persist. It also evaluates the defeat
block without the native nonzero-game-mode gate. The sweep must execute before
the defeat transition and in House registration order; pending expiry remains
active in campaign mode.

## Native mechanism details

### Effective ownership and reversible control

`FUN_0070F820` resolves effective owner in this precedence:

1. reversible MC link at `Techno+0x2C0` -> capture node original House;
2. otherwise a non-null temporary-transfer marker at `+0x2CC` -> saved source
   House at `+0x2E0` (which may itself be null);
3. otherwise current owner at `+0x21C`.

When effective owner differs from current owner and the victim has the
reversible MC link, `0x004FC6D0` calls
`CaptureManagerClass__SetOriginalOwner @ 0x00472330`. That function resolves
the literal `Civilian` through HouseType name/ID authority, finds the first
matching House in global order, reverse-walks the controller's node vector,
and rewrites every node for the victim. Resolution success returns true and
spares the victim; failure falls through to damage. Stock `rulesmd.ini` maps
the `Civilian` side/name to the `Neutral` country entry.

### Temporal detachment

If victim `Techno+0x278` is non-null, the sweep calls `FUN_0071AD40` on that
incoming head `TemporalClass` before damage. The helper clears the victim's
warped byte and backlink, recursively clears both neighbor directions, and
zeros each detached manager's target, chain links, and accumulated state.

### Receiver packet

Assembly `0x004FC731..0x004FC766` proves the payload. The warhead is the same
existing rules authority already represented by `RuleSet.bridge_warheads.c4_name`
and `ResolvedRuleHandles.c4`; no crate-owned or hardcoded C4 identity is needed.

## Mounted-retail census

The 184-file mounted corpus contains exactly seven Action 119 chunks, all in
campaign maps and all using `ParamType=0` with numeric country operands:

| Map | `[Actions]` line | Trigger ID | Chunk | Operand |
|---|---:|---|---:|---:|
| `all01umd.map` | 9320 | `08AE3E3C` | 3 | 9 |
| `all01umd.map` | 9486 | `0611BABC` | 0 | 1 |
| `all03umd.map` | 14878 | `06B0CCCC` | 1 | 4 |
| `sov01umd.map` | 10126 | `096879AC` | 4 | 6 |
| `sov06lmd.map` | 4264 | `0782720C` | 0 | 9 |
| `sov06lmd.map` | 4265 | `09A0EC1C` | 0 | 0 |
| `sov06lmd.map` | 4266 | `09A0C36C` | 0 | 1 |

No mounted retail skirmish map uses Action 119. No mounted retail Action 119
row uses registry `ParamType` 6, 7, or 8. Numeric country resolution is the
ordinary shipped path; raising-House and start-slot sentinels remain executable
behavior and require synthetic acceptance coverage.

## Verified prerequisites and matches

| Required authority | Current Rust state |
|---|---|
| Global forward Techno order with synchronous compacting lifecycle | Corrected after implementation review: `EntityStore` is the four-Techno typed store and stable IDs preserve monotonic construction order; the alive projection must be re-read after each `commit_noncombat_aoe_hits` callback. `LogicVector` is explicitly not this authority because Conceal removes from Logic without destroying the Techno. |
| Reversible MC node and reciprocal victim link | `CaptureManagerState.controlled_nodes`, `CaptureNodeState.original_owner`, and `GameEntity.mind_control_controller_id` are persisted and restore-validated. |
| Temporary owner-transfer precedence inputs | `temporary_owner_transfer_marker` and `temporary_owner_transfer_source` are persisted, hashed, and restore-validated. |
| Incoming/outgoing Temporal chain state | `TemporalManagerState`, `temporal_targeting_me_id`, and `being_temporally_warped_out` are persisted, hashed, and reciprocal-validated. |
| C4 identity and concrete receiver pipeline | `ResolvedRuleHandles.c4`, `EntityDamageEvent::direct_receiver`, and `commit_noncombat_aoe_hits` already own this behavior; crate HealBase supplies a close packet/order precedent. |

## Doc errors discovered

- `docs/research/.swarm-claims.md` is an immutable historical coordination
  ledger. Its 2026-05-28 `ScatterAllUnits` claim is retained as history but is
  explicitly superseded by this active-binary correction.
- `HOUSECLASS_MPLAYER_DEFEATED_SCATTER_PRODUCTION_TAIL_RESWARM_20260528.md`
  and `MULTIPLAYER_DEFEAT_VICTORY_GHIDRA_REPORT.md` misname
  `0x004FC6D0` as `ScatterAllUnits` and misread current health / Temporal state /
  concrete ReceiveDamage as scatter coordinates and flags.
- Ordinary mission Scatter is virtual `+0x174`, independently visible at
  `HouseClass__Update 0x004F899F`; the destruction sweep calls `+0x16C`.
- Current Rust comments in `check_defeat` repeat the stale scatter/persistence
  interpretation and must be corrected with the code.

## Evidence-backed exclusions

- Action 119 has no mounted stock-skirmish authoring; its shipped activation is
  campaign-only. This does not exclude the House-update defeat caller, which is
  active in ordinary nonzero-game-mode skirmish.
- Registry operand parameter types 6/7/8 have zero Action 119 rows in the full
  mounted retail corpus. Supporting those custom-map-only encodings is not an
  active-retail closure requirement for this mechanism.

## Implementation handoff and acceptance

Implement one shared synchronous House destruction operation and call it from
Action 119, pending-result expiry, and multiplayer defeat. Acceptance must
cover: seven retail-like numeric operands; `0x2325`, `-1`, and A..H sentinels;
first House country match; false results for missing Trigger/House/rules;
current, temporary, and reversible effective owners; Civilian rewrite success
and failure; full incoming Temporal-chain clear before damage; configured C4
identity and `(ignore_defenses=true,arg6=true)` packet; attacker/source absence;
live compacting order plus survivor duplicate guard; pending expiry in campaign;
defeat sweep before `MPlayer_Defeated`; and the nonzero-game-mode defeat gate.

No snapshot-version change should be necessary because every future-affecting
link and timer input already has persisted/hash authority. Re-run the existing
result-link snapshot/hash/restore tests to prove that assumption.

## Ghidra annotation candidates

- Rename `HouseClass__ScatterAllUnits @ 0x004FC6D0` to a destructive House
  sweep name and correct the `+0x16C` receiver semantics.
- Name `FUN_006E3180` as Trigger Action 119's House destruction wrapper.

No Ghidra metadata was modified during this scan.
