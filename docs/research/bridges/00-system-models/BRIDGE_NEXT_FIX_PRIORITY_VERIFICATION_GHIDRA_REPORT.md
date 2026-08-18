# Bridge Next Fix Priority Verification - Ghidra Research Report

**Address(es):** `0x00574000`, `0x00574C20`, `0x0047DD70`, `0x00575EE0`  
**Investigation Mode:** coverage-map verification slice  
**Claimed Scope:** re-check whether the next implementation should be hut fallback, collapse sound, or event `0x1F` trigger delivery.  
**Non-Scope:** full re-investigation of all bridge collapse state machines, complete trigger enum naming, or Rust implementation.  
**Confidence:** High for priority ordering; Medium for stock-map incidence of the hut no-overlay branch.  
**Active in YR:** Conditional by subsystem: bridge collapse sound is active in standard YR; hut fallback is live but topology-conditional; event `0x1F` is live but trigger-content-conditional.

## 1. Overview

This verification pass re-read the three fresh swarm reports, cold-decompiled the decisive binary functions, and re-scanned the current Rust surfaces. The earlier recommendation stands: implement the CABHUT no-overlay fallback first, then bridge collapse audio, and keep event `0x1F` stubbed until campaign trigger runtime is in scope.

The reason is not that the hut fallback is common. It is that current Rust has a concrete algorithmic mismatch in a live path, while the sound work is a missing audio route on an otherwise close visual effect path, and event `0x1F` has no skirmish effect unless authored triggers exist.

## 2. Class Layout / Key Offsets

| Owner | Offset / value | Meaning | Evidence | Active in YR |
|---|---:|---|---|---|
| `CellClass` | `+0x140` | Hut fallback bridge flags; accepted starter mask is `0x500` | `0x00574000`, `0x00574C20` | Conditional |
| `CellClass` | `+0x2C` | Anchor source when starter has `0x100` but not `0x80` | `0x00574000`, `0x00574C20` | Conditional |
| `RulesClass` | `+0x15C/+0x168` | `BridgeExplosions` vector/count | `0x0047DD70`; `rulesmd.ini:529` | Yes |
| `CellClass` | `+0x3C` | Attached cell tag gate before event `0x1F` delivery | `0x00575EE0` | Conditional |
| Trigger event | `0x1F` | Numeric event delivered by bridge span collapse broadcaster | `0x00575EE0` | Conditional |

## 3. Core Logic

### Hut fallback priority check

`MapClass__DestroyBridge_High_OnHutDeath @ 0x00574000` and the low twin at `0x00574C20` both confirm the fallback is single-starter. After the 5x5 overlay scan fails, the binary checks only `flags & 0x500` at the hut cell, then direction indices `0..7`, distances `1,2,3`, and stops at the first accepted cell.

The pure `0x400` branch re-confirmed the easy-to-misread detail: it walks E when `0x800` is clear or S when set, but after the first non-`0x400` cell it offsets two cells in the opposite direction. Four consecutive continuation cells returns early.

Current Rust still returns and damages a traced list through `find_hut_fallback_cells` and `append_hut_fallback_trace` in `src/sim/world/bridge_orchestrator.rs:442..494`. That is a concrete mismatch with the binary, not just missing polish.

### Collapse sound priority check

`CellClass__BlowUpBridge @ 0x0047DD70` re-confirmed no direct sound call. It spawns one delayed `BridgeExplosions` anim per cell that passes the gates. The current Rust `spawn_bridge_debris` already creates delayed `WorldEffect` entries for selected bridge explosion anims in `src/sim/world/bridge_orchestrator.rs:925..999`, but does not route the anim `Report` / `StartSound` to audio.

This is player-visible, but the implementation shape is narrower than the hut fallback fix: attach or emit the selected anim sound when the delayed effect starts. It should not use `BridgeRepaired`.

### Event `0x1F` priority check

`RepairBridgeSegment @ 0x00575EE0` re-confirmed event `0x1F` delivery is gated by `CellClass+0x3C` and uses endpoint-exclusive span footprint iteration. The call is `TechnoClass__ProcessCellAction(0x1f, 0, DAT_00abd480, 0, 0)`.

Because this is trigger dispatch only, current Rust's `notify_bridge_span_collapse` no-op in `src/sim/world/bridge_orchestrator.rs:873..875` remains acceptable for skirmish. It becomes important when campaign/custom trigger runtime is implemented.

## 4. INI Keys

| Key / data | Standard YR value | Relevance | Priority effect |
|---|---|---|---|
| `[General] BridgeExplosions=` | `TWLT026,TWLT036,TWLT050,TWLT070` | Active collapse anim pool | Confirms audio work is real and player-visible |
| `TWLT026/036/050/070 Report=` | `ExplosionShard`, `Explosion06`, `Explosion07`, `Explosion09` | Exact sound keys | Use selected anim sound, not one global sound |
| `[AudioVisual] RepairBridgeSound=` | `BridgeRepaired` | Repair-only | Do not reuse for collapse |
| `[CABHUT] BridgeRepairHut=` | `yes` | Enables hut bridge destruction entry | Confirms hut path is live |
| Scenario `[CellTags]` / `[Events]` | map-authored | Required for event `0x1F` effect | Skirmish stub is safe without authored event 31 |

## 5. Integration Points

Hut fallback is reached from live `BridgeRepairHut` destruction callers and can mutate bridge state differently from Rust on no-overlay topologies.

Collapse sound is reached from every `BlowUpBridge` cell that passes the binary effect gates. Rust visuals are already created on that path; audio routing is missing.

Event `0x1F` is reached through bridge endpoint/span collapse paths, but only produces scenario-scripted effects when matching cell tags and trigger conditions exist.

## 6. Current Rust Implementation Status

| Area | Current Rust status | Verified delta |
|---|---|---|
| Hut fallback | `find_hut_fallback_cells` returns traced cells and `apply_hut_damage_to_cell` applies generic per-cell damage | Mismatch with binary single-starter, anchor, and bounded retry behavior |
| Collapse audio | `spawn_bridge_debris` creates selected delayed `WorldEffect`s but no collapse sound event | Missing selected anim `Report` / `StartSound` playback |
| Event `0x1F` | `notify_bridge_span_collapse` is no-op | Acceptable for skirmish; missing only for campaign/custom trigger support |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| High hut fallback | verified | cold decompile `0x00574000` | none for priority decision |
| Low hut fallback | verified | cold decompile `0x00574C20` | none for priority decision |
| Current Rust hut fallback | verified | `bridge_orchestrator.rs:442..494`, `:685..710` | implementation still needed |
| BlowUpBridge sound source | verified | cold decompile `0x0047DD70`; prior `0x00424CE0`, `0x00427D00` reports | implementation still needed |
| Current Rust collapse visuals/audio | verified | `bridge_orchestrator.rs:925..999`; sound surfaces only expose repair event | implementation still needed |
| Event `0x1F` delivery | verified | cold decompile `0x00575EE0`; prior dispatcher report | campaign trigger implementation deferred |
| Stock-map hut fallback incidence | deferred | prior loose file scan only | unpack MIX maps if incidence ranking matters |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-1 - Does the hut fallback still look like the highest-risk implementation item after cold re-read? -> Yes; binary is single-starter/ramp-walk, Rust traces and damages multiple evidence cells.` (evidence: `0x00574000`, `0x00574C20`, `bridge_orchestrator.rs:442..494`)
- `[RESOLVED] OQ-2 - Could bridge sound outrank hut fallback because it is more visible? -> It is visible, but the Rust delta is narrower: route selected anim sound at effect start; no structural bridge-state mismatch.` (evidence: `0x0047DD70`, `bridge_orchestrator.rs:925..999`)
- `[RESOLVED] OQ-3 - Is event `0x1F` unsafe to leave stubbed for skirmish? -> No; binary effect requires attached cell tags and trigger conditions.` (evidence: `0x00575EE0`, `BRIDGE_DESTROYED_TRIGGER_EVENT_0X1F_GHIDRA_REPORT.md`)
- `[DEFERRED] OQ-4 - How often does the no-overlay hut fallback occur in shipped packed maps?` (category: `requires-different-system-context`; reason: MIX map extraction was not part of this verification; next-step-if-pursued: unpack all shipped maps and scan CABHUT plus nearby overlays/flags)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Hut no-overlay fallback picks one starter and derives one anchor/ramp walk | `0x00574000`, `0x00574C20` | mismatch | `src/sim/world/bridge_orchestrator.rs:442..494`, `:685..710` | Replace traced fallback cells with binary starter/anchor/retry behavior | No-overlay hut with N distance-2 and E distance-1 flags chooses N distance-2 only | Do not BFS, trace, or full-span collapse from all evidence cells |
| Collapse sound comes from selected delayed `BridgeExplosions` anim | `0x0047DD70`; `artmd.ini` TWLT `Report=` keys | missing audio route | `src/sim/world/bridge_orchestrator.rs:925..999`, app effect/audio routing | Play selected anim sound when delayed effect starts | Seed selects `TWLT036`; effect start emits `Explosion06`, not `BridgeRepaired` | Do not add immediate global bridge collapse sound |
| Event `0x1F` is trigger-only and tag-gated | `0x00575EE0`, `0x006E53A0` report | no-op by design for skirmish | `notify_bridge_span_collapse`, future trigger runtime | Keep stub until campaign trigger runtime; later deliver event 31 to tagged footprint cells | Skirmish bridge collapse has no extra trigger side effects | Do not mutate bridge state or play audio from event 31 |

## Sources

- Ghidra cold decompile: `0x00574000`, `0x00574C20`, `0x0047DD70`, `0x00575EE0`
- `docs/research/BRIDGE_HUT_FALLBACK_FLAGS_RAMP_ONLY_GHIDRA_REPORT.md`
- `docs/research/BRIDGE_COLLAPSE_SOUND_SOURCE_GHIDRA_REPORT.md`
- `docs/research/BRIDGE_DESTROYED_TRIGGER_EVENT_0X1F_GHIDRA_REPORT.md`
- `src/sim/world/bridge_orchestrator.rs`
- `ini/rulesmd.ini`
- `ini/artmd.ini`
