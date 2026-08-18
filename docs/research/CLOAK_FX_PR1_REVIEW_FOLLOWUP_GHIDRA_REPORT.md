---
title: Cloak FX PR1 — Plan-Review Follow-Up Ghidra Findings
date: 2026-05-11
confidence: high (all five questions verified directly in gamemd.exe)
supersedes: assumptions in CLOAK_FX_SHADER_BRIDGE_GHIDRA_REPORT.md noted inline
---

# Cloak FX PR1 — Plan-Review Follow-Up Ghidra Findings

Five focused questions surfaced during `/review-plan` of the
2026-05-11 cloak FX integration plan. Each answer is verified directly
in `gamemd.exe`. The original research report
`CLOAK_FX_SHADER_BRIDGE_GHIDRA_REPORT.md` is mostly correct; corrections
below override that doc where noted.

---

## Q1 — ReCloakDelayTimer constant

**Status:** RESOLVED. Plan's "30 ticks" guess is wrong; correct value is
**18 frames** from `[General] CloakDelay=0.02` (minutes) × 900 (frames/minute).

### Verification trace

`TechnoClass__CloakingTick @ 0x006FB740`, state-3 → state-0 transition
(when uncloak fade completes, at 0x006FB9AC):

```
006fb9ac: MOV EAX,[g_CurrentFrameCounter]
006fb9b1: LEA EDX,[ESI + 0x22c]          ; CDTimerClass (shimmer / cloak ts)
006fb9b7: XOR ECX,ECX
006fb9b9: MOV [ESI+0x238],EBX            ; AttachedTag = 0 (clear pending decloak)
006fb9bf: MOV [EDX],EAX                  ; +0x22C = current frame
006fb9c5: MOV [EDX+4],EAX                ; +0x230
006fb9c8: MOV [EDX+8],ECX                ; +0x234
006fb9cb: MOV [ESI+0x224],EBX            ; CloakProgress = 0
006fb9d1: MOV [ESI+0x220],EBX            ; CloakState = 0 (Uncloaked)

; --- Compute ReCloakDelayTimer ---
006fb9d7: MOV ECX,[g_RulesClass_Instance]
006fb9dd: MOV EDI,[g_CurrentFrameCounter]
006fb9e3: FLD double ptr [ECX + 0x1410]  ; rules.CloakDelay (minutes, double)
006fb9e9: FMUL double ptr [0x007e27f8]   ; × 900.0
006fb9ef: CALL Math__ftol                ; EAX = trunc(minutes × 900)
006fb9f4: MOV ECX,[ESP + 0x14]
006fb9f8: LEA EDX,[ESI + 0x240]          ; ReCloakDelayTimer (CDTimerClass)
006fba00: MOV [EDX],EDI                  ; +0x240 = start = current_frame
006fba02: MOV [EDX+4],ECX                ; +0x244 = (unused)
006fba07: MOV [EDX+8],EAX                ; +0x248 = duration in frames
```

### Constant identification

| Address | Value | Meaning |
|---------|-------|---------|
| `dbl@0x007E27F8` | `0x408C200000000000` = **900.0** | Frames per minute (60 sec × 15 fps) |
| `RulesClass+0x1410` (double) | parsed from `[General] CloakDelay=` | Default value from constructor; INI key string at `0x0083BF34` |
| Stock `rulesmd.ini` value | **`CloakDelay=0.02`** (line 64) | 0.02 minutes |

### Result

`ReCloakDelayTimer = ftol(0.02 × 900) = ftol(18.0) = 18 frames` ≈ 1.2s @ 15 Hz.

Stock INI comment: *"forced delay that subs will remain on surface before
allowing to submerge"*. This matches the timer's role.

### Implications for plan

- Add `cloak_delay_frames: u32` to `GeneralRules`, computed at parse time:
  `(cloak_delay_minutes × 900).round() as u32`, default 18.
- Replace the hard-coded `30` in Task 12's state-3 → state-0 path with
  `rules.general.cloak_delay_frames`.
- Same pattern as the existing `damage_delay_minutes` /
  `spy_power_blackout_frames` precomputed fields.

---

## Q2 — Allied-shimmer trigger semantics

**Status:** RESOLVED with **plan revision required**. The plan restricts
shimmer to `entity.owner == local_owner_id`. The binary applies
shimmer to **all** cloaked unit draws regardless of owner — the
`IsHumanPlayer` gate is never reached in retail YR because the
shimmer-suppression timer is dormant.

### Verification trace

`TechnoClass__ModifyCloakDrawFlags @ 0x0070ED80`, entry sequence:

```
0070ed8c: MOV EDI,ECX                    ; EDI = TechnoClass `this`
0070ed8e: MOV EBX,[EDI + 0x1ec]          ; EBX = suppression_start
0070ed94: MOV ECX,[EDI + 0x1f4]          ; ECX = suppression_duration
0070ed9a: CMP EBX,-1                     ; uninitialized sentinel?
0070ed9d: JZ 0x0070eda9                  ; (not taken at runtime)
0070ed9f: MOV EDX,EAX                    ; EAX = current_frame
0070eda1: SUB EDX,EBX                    ; elapsed since start
0070eda3: CMP EDX,ECX                    ; elapsed >= duration?
0070eda5: JGE 0x0070edc1                 ; → skip IsHumanPlayer gate
0070eda7: SUB ECX,EDX                    ; (only reachable when elapsed<duration)
0070eda9: TEST ECX,ECX
0070edab: JZ 0x0070edc1                  ; ECX==0 → skip gate
0070edad: MOV ECX,[EDI + 0x21c]          ; ECX = cloaked unit's Owner.House
0070edb3: CALL HouseClass__IsHumanPlayer
0070edb8: TEST AL,AL
0070edba: JZ 0x0070ee1b                  ; not human → return param_2 (no flags)
```

### Constructor inits +0x1EC and +0x1F4

`TechnoClass__Constructor @ 0x006F2B40`:
- At `0x006F2CD5`: `+0x1EC = g_CurrentFrameCounter` (start = construction frame)
- At `0x006F2CDB`: `+0x1F4 = 0` (duration zero)

### Byte-pattern verification of "no live writer"

Search for `MOV [reg+0x1F4], reg` (`89 ?? f4 01 00 00`):
- `0x006F2CDB` — `TechnoClass__Constructor` (confirmed)
- `0x005F7140` — `ObjectTypeClass__Constructor` (different class — `ObjectTypeClass+0x1F4`)
- `0x005F9403` — `ObjectTypeClass__ReadINI` (different class)
- `0x00611305`, `0x00665922`, `0x00669889`, `0x0075E9E1...` — all unrelated classes

**No live writer to `TechnoClass+0x1F4` outside the constructor.** Confirmed.

### Runtime trace

With `+0x1EC = construction_frame` and `+0x1F4 = 0`, the suppression-active
path requires `EBX == -1` (false: it's a real frame number) OR
`elapsed < duration` (false: 0 ≤ duration ≤ 0 is the only case, and
the JGE-on-equal at `0x0070EDA5` jumps past the gate). The `JGE`
always takes for every cloaked unit, every draw — `IsHumanPlayer` is
dead code in retail YR.

### HouseClass__IsHumanPlayer semantics (for completeness)

`@ 0x0050B6F0`:
```c
bool IsHumanPlayer(HouseClass* h) {
    if (g_GameMode != 0)              // skirmish/MP
        return h == g_PlayerPtr;       // ← only THE local player
    if (h.IsHuman || h.IsControlled)  // single-player campaign
        return true;
    return false;
}
```

So in skirmish/MP, "human player" = the local player specifically (not "any
human-controlled house"). Allied MP players are not "human" from each
other's perspective in this check.

### Implications for plan

- Drop the `entity.owner == local_owner_id` gate around `shimmer_phase_alpha`
  in `compute_cloak_fx_uniform` (Task 15).
- Shimmer applies whenever `visual_state == 3` regardless of cloak owner.
  This makes sensor-revealed enemy cloaked subs (state 3 via SensorCount > 0)
  also pulse — which matches gamemd's observable output.
- Keep `shimmer_phase_alpha` itself unchanged (the 4-band table is correct).
- Document this as: "Shimmer-suppression timer is shipping-but-dormant
  retail-YR code; gating shimmer by `IsHumanPlayer` is unreachable in
  practice. Implementing per-owner shimmer gating would diverge from
  player-visible output."

---

## Q3 — `TechnoClass+0x41A` is NOT a discovered-by bitmask

**Status:** RESOLVED with **major plan revision**. The plan + prior research
called this "discovered" and assumed per-house semantics. The field is
actually `IsLocalPlayerOwned: bool` — a single byte that mirrors
`Owner == g_PlayerPtr` at all times.

### Verification trace

`TechnoClass__ChangeOwner @ 0x007014A0`. Ghidra docstring header notes:

> "update +0x41A IsLocalPlayerOwned = (new == g_PlayerPtr)"

Decompiled body confirms:

```c
param_1[0x87] = param_2;                              // Owner = new_owner
if ((param_2 == g_PlayerPtr) && (g_PlayerPtr != 0)) {
    uVar3 = 1;
} else {
    uVar3 = 0;
}
*(undefined1 *)((int)param_1 + 0x41a) = uVar3;        // +0x41A = (new == local) ? 1 : 0
```

### All writers to `+0x41A` (byte-pattern search)

| Address | Function | What it does |
|---------|----------|--------------|
| `0x006F3F55`, `0x006F3F5E` | `TechnoClass__Init_Managers` | Init to 0 |
| `0x006F2FB5` | `TechnoClass__Constructor` | Init to 0 |
| `0x006F4616` | `MissionClass__Constructor` | Init to 0 |
| `0x00701751` | `TechnoClass__ChangeOwner` | Set to `(new == g_PlayerPtr)` |
| `0x007018DD`, `0x007018E8` | `TechnoClass__ChangeOwner` (BuildingClass override branch) | Same |
| `0x00707A94` | `TechnoClass__PointerExpired` | Clear to 0 on owner expiry |
| `0x00741862` | `UnitClass__PerCellProcess` | Copy from absorbed unit (crush/pickup edge case) |

No "OR bit into +0x41A" pattern exists (`80/0C ?? 1A 04 00 00 ??` returns
zero matches). Field is overwritten wholesale; not used as a bitmask.

### Use in GetVisualState (the "discovered-clamp")

`TechnoClass__GetVisualState @ 0x00703860`:

```c
// At the high end of visual_raw (>= 0xC0):
if (visual_raw < 0xC0) return 3;
if ((char)param_2 == 0 && param_1[0x41a] != 0)   // "no explicit viewer AND mine"
    return 3;                                     // clamp to 50% blend
return (0xFE < iVar3) + 4;                        // 4 or 5
```

The "discovered" clamp is really: *when rendering my own cloaked unit in
the default no-explicit-viewer path, clamp to state 3 (visible 50%)
instead of state 4/5 (more transparent or hidden).* This is the local
player's own cloaked units rendering visibly to themselves.

### Implications for plan

- Delete the proposed `entity.is_discovered_by(house) -> bool` helper. It
  doesn't exist in gamemd's data model.
- The "discovered-clamp" in `visual_state` (Task 9) becomes:
  ```rust
  if entity.owner == local_owner_id {
      return 3;
  }
  ```
- No new per-house bitmask field is needed on `GameEntity`.
- The plan's "Discovered-clamp at visual_raw >= 0xC0 returns 3 (not 4)"
  parity-critical item still holds — but the trigger is "is mine", not
  "ever spotted by enemy".

### Note on the prior research doc

`CLOAK_FX_SHADER_BRIDGE_GHIDRA_REPORT.md` labels `+0x41A` as a "discovered"
or "revealed" field. That label is incorrect for retail YR semantics.
Update or supersede on next pass.

---

## Q4 — CloakStop / "is idle" check is gated on WeaponsFactory destinations

**Status:** RESOLVED with **plan revision required**. Plan reduces the auto-cloak
gate to `entity.movement_target.is_some()`, which is wrong. The real gate is
"destination is a `WeaponsFactory=yes` building".

### Verification trace

`TechnoClass__CloakingTick @ 0x006FB740`, auto-cloak prerequisite at
`0x006FB7FD`:

```
006fb7fd: PUSH 0                        ; arg = 0
006fb7fe: MOV ECX,ESI
006fb800: CALL FootClass__GetDestination ; returns *(this->NavCom_array + 0)
006fb805: MOV EDI,EAX
006fb807: TEST EDI,EDI
006fb809: JZ 0x006fb829                 ; no destination → allow auto-cloak

006fb80b: MOV EAX,[EDI]
006fb80d: MOV ECX,EDI
006fb80f: CALL [EAX + 0x2c]             ; virtual WhatAmI()
006fb812: CMP EAX,0x6                   ; == 6 (BuildingClass)?
006fb815: JNZ 0x006fb829                ; not a building → allow auto-cloak

006fb817: MOV ECX,[EDI + 0x520]         ; building.Type (cached BuildingTypeClass*)
006fb81d: CMP byte [ECX + 0x16bd],BL    ; BuildingTypeClass.WeaponsFactory == 0?
006fb823: JNZ 0x006fbc80                ; nonzero → BAIL OUT (don't auto-cloak)
```

### Field identifications (cross-referenced from existing docs)

| Offset | Field | Source |
|--------|-------|--------|
| `BuildingClass+0x520` | Cached `BuildingTypeClass*` | `TERRAIN_CLASS_GHIDRA_REPORT.md:1158` ("RTTI=6 is BuildingClass; +0x520 is cached TypeClass") |
| `BuildingTypeClass+0x16BD` | `WeaponsFactory: bool` | `BUILDINGTYPECLASS_FIELDS.csv:291` (ctor at `0x0045E139`) |
| `Abstract WhatAmI() == 6` | BuildingClass RTTI | Same source |

### Retail YR INI confirms naval shipyards are WeaponsFactory=yes

```
ini/rulesmd.ini:
  [GAYARD] line 11850: WeaponsFactory=yes
  [NAYARD] line 12638: WeaponsFactory=yes
  [YAYARD] line 13388: WeaponsFactory=yes
```

So a SUB/DLPH/SQD that is en route to its faction's Naval Shipyard for repair
**will not auto-cloak** during the trip. A SUB cruising in open water with a
nav-target on a water cell **will** auto-cloak (destination is not a building).

### Implications for plan

Replace the proposed gate
```rust
if type_data.cloak_stop && entity.movement_target.is_some() { continue; }
```
with:
```rust
// Auto-cloak unless heading into a WeaponsFactory (e.g., naval shipyard for repair).
// Movement to any other destination type — water cells, non-factory buildings,
// enemy units — does not block auto-cloak.
let blocked = entity.movement_target.is_some_and(|target| {
    target_is_building_with_weapons_factory(world, rules, interner, target)
});
if blocked { continue; }
```

This requires:
- `movement_target` resolution to determine if the target is a building
- Building → type lookup → `weapons_factory` flag

Both data shapes already exist in the codebase (target IDs, `ObjectType.weapons_factory`).
**Verify `weapons_factory` is parsed on `ObjectType`** — if not, add it (likely 1 line).

### Side observation: this check ignores `CloakStop=`

Note that the binary's gate does NOT consult any `CloakStop=` field on
the cloaked unit's TypeClass — it's hard-coded behavior. The INI key
`CloakStop=yes` on SUB/DLPH/SQD in retail YR is parsed by the engine
but is used elsewhere (likely controlling cloak behavior while
queued / mission-paused). Do not assume `CloakStop=` controls this
gate.

---

## Q5 — Sensors system has 6 retail-YR users; cannot be hardcoded to 0

**Status:** RESOLVED with **scope expansion required**. The plan would
hardcode `cell_sensor_count = 0` because no Sensors implementation
exists in the repo. But retail YR sets `Sensors=yes` on six unit types,
including the three cloakable units themselves. Hardcoding 0 breaks
sub-vs-sub detection.

### Verification

`grep ^Sensors=yes ini/rulesmd.ini` yields six matches:

| Unit ID | Role | Sensors=yes? |
|---------|------|--------------|
| `[DEST]` | Allied Destroyer | yes |
| `[DLPH]` | Allied Dolphin | yes (and Cloakable=yes) |
| `[SUB]` | Soviet Typhoon Attack Sub | yes (and Cloakable=yes) |
| `[SQD]` | Yuri Giant Squid | yes (and Cloakable=yes) |
| `[BSUB]` | Soviet Boomer Sub (YR exclusive) | yes |
| `[CDEST]` | (carrier-related; verify section name) | yes |

### Behavioral impact

In retail YR:
- Two enemy SUBs that get within sensor range mutually reveal each other —
  both are cloaked + both have Sensors, so each acts as a detector for the other.
- A DEST (no Cloakable) reveals enemy SUBs/DLPHs/SQDs in sensor range.
- A DLPH reveals enemy cloaked subs and is itself sensor-visible to enemy DESTs.

If PR 1 hardcodes `cell_sensor_count = 0`, every enemy SUB renders at
`visual_state == 5` (skip) regardless of nearby detectors. The defining
"detected sub" rendering does not happen. Players notice immediately.

### Implications for plan

PR 1 needs at minimum a per-cell-per-house sensor counter:

- `Cell` gains a `sensor_count: HashMap<HouseId, u8>` (or `BTreeMap` for
  determinism), incremented for each Sensors=yes unit owned by `house`
  within `SensorsSight` cells, decremented when the unit leaves.
- Updated in `tick_vision` (or a new `tick_sensors` adjacent to it).
- `visual_state` reads `cell.sensor_count_for(viewer_house)`.

This is roughly the same architecture as the existing fog/vision system
but counting Sensors=yes units instead of vision-range units, with house
partitioning.

Either:
- (a) **Bundle Sensors into PR 1.** Adds ~1 file (~150 lines)
  for `sim/sensors.rs` + ~50 lines for cell integration. Sound choice
  given that 3 of 3 PR 1 cloakable units have `Sensors=yes`, so the
  feature is co-dependent.
- (b) **Defer to PR 1.5.** Then PR 1 has to disable cloak rendering
  entirely (visual_state == 5 for all enemy cloak) or accept that
  cloak detection is purely vision-based until 1.5 ships. The latter
  is visibly wrong for sub-vs-sub combat.

Recommend (a). The Sensors counter is a small, deterministic, sim-side
addition that integrates cleanly with cell/vision tick ordering.

---

## Summary table

| Question | Plan said | Binary truth | Plan action |
|----------|-----------|--------------|-------------|
| Q1: ReCloakDelayTimer | 30 ticks (guessed) | `ftol([General] CloakDelay × 900)` = 18 frames | Read from `GeneralRules`; precompute frames |
| Q2: Allied-shimmer trigger | `entity.owner == local_owner_id` (too strict) | Shimmer applies to ALL cloaked draws | Remove the owner gate |
| Q3: `+0x41A` discovered | Per-house bitmask helper needed | `IsLocalPlayerOwned: bool` (mirrors `owner == g_PlayerPtr`) | Use `entity.owner == local_owner_id`; no new field |
| Q4: Auto-cloak gate | `movement_target.is_some()` (too strict) | "Destination is a WeaponsFactory building" | Building+type lookup; verify `weapons_factory` parsed on `ObjectType` |
| Q5: Sensors | Hardcoded `cell_sensor_count = 0` | 6 retail YR units (incl. SUB/DLPH/SQD) | Add minimal Sensors counter to PR 1 |

---

## Verified gamemd.exe addresses cited

- `TechnoClass__CloakingTick` — `0x006FB740`
- `TechnoClass__GetVisualState` — `0x00703860`
- `TechnoClass__ModifyCloakDrawFlags` — `0x0070ED80`
- `TechnoClass__ChangeOwner` — `0x007014A0`
- `TechnoClass__PointerExpired` — `0x007077C0`
- `TechnoClass__Constructor` — `0x006F2B40` (inits `+0x1EC`, `+0x1F4`, `+0x41A`)
- `UnitClass__PerCellProcess` — `0x007416A0`
- `HouseClass__IsHumanPlayer` — `0x0050B6F0`
- `FootClass__GetDestination` — `0x0065AD30`
- `RulesClass__ReadGeneral` (parses `CloakDelay`) — `0x00670AFF`
- `Math__ftol` — `0x007C5F00`
- `dbl@0x007E27F8` = `900.0` (frames per minute)
- `s_CloakDelay` — `0x0083BF34`
- `RulesClass+0x1410` (double) — `CloakDelay` minutes
- `BuildingClass+0x520` — cached `BuildingTypeClass*`
- `BuildingTypeClass+0x16BD` — `WeaponsFactory: bool`
- `TechnoClass+0x41A` — `IsLocalPlayerOwned: bool`
- `TechnoClass+0x1EC` — shimmer-suppression timer start (dormant)
- `TechnoClass+0x1F4` — shimmer-suppression timer duration (dormant; constructor-only writer)
