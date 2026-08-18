# OpportunityFire — Ghidra Research Report

**Address(es):** `0x00843A74` (INI key string), `TechnoTypeClass+0x6AF` (storage), six readers across FootClass / UnitClass mission & radio handlers
**Confidence:** HIGH (string xref confirmed; all six field-access sites disassembled; INI reader store at `0x0071484A` verified; adjacent fields cross-checked against the actual string table at `0x00843A64..0x00843A8F`)
**Active in YR:** Yes — set on 14 standard YR units

---

## 1. Overview

`OpportunityFire=yes` is a **firing-posture preservation flag** on TechnoTypeClass. It does **not** trigger a target scan. Instead, it tells six specific mission-transition and radio-handshake sites to **skip their "reset locomotor / hand off / scatter / rate-reset" side effects** so a unit that already has a TarCom stays locked on it while other parts of its state change.

**This is the opposite of the reading implied by the INI comment** (`"Can fire at targets while performing other actions, like moving"`). The engine's default per-tick firing pipeline (`Fire_At`) fires whenever a TarCom is set regardless of mission or motion — that capability is always on. What `OpportunityFire=yes` really buys you is **TarCom persistence through mission transitions**: the unit doesn't drop its target when it docks, arrives, scatters, or deploys.

## 2. Critical correction of prior docs

The existing archive docs carry **two offset mislabels** that must be corrected:

| Offset | Prior label | Correct label | Evidence |
|---|---|---|---|
| `TechnoTypeClass+0xD33` | "CanBeScattered" (TARGET_ACQUISITION_GHIDRA_REPORT.md:433, and my own GREATEST_THREAT_SCAN §7) | **`CanApproachTarget`** | INI reader at `0x007144A7` pushes string `0x843C2C` = `"CanApproachTarget"` and stores ReadBool result at `[EBP+0xD33]`; confirmed in rulesmd.ini line 6836 (`CanApproachTarget=no` on `[SPY]`) |
| `TechnoTypeClass+0xD34` | "OpportunityFire" (TARGET_ACQUISITION_GHIDRA_REPORT.md:434, and my own GREATEST_THREAT_SCAN §6.5 heading / §7) | **`CanRecalcApproachTarget`** | INI reader at `0x007144C1` pushes string `0x843C14` = `"CanRecalcApproachTarget"` and stores ReadBool result at `[EBP+0xD34]` |
| `TechnoTypeClass+0x6AF` | (unassigned in existing docs) | **`OpportunityFire`** | INI reader at `0x0071483D` pushes string `0x843A74` = `"OpportunityFire"` and stores ReadBool result at `[EBP+0x6AF]` |

**Behavioral impact of the mislabel:** the retarget-abort branch in `FootClass::Greatest_Threat_Scan` (§3(D) of `GREATEST_THREAT_SCAN_GHIDRA_REPORT.md`) reads `TypeClass+0xD34` — which is `CanRecalcApproachTarget`, not `OpportunityFire`. The *behavior* documented in that report is still correct (drop the archive approach target when distance exceeds `ApproachTargetResetMultiplier × range`); only the INI key that gates it was wrong. `CanRecalcApproachTarget` is the right gate — if a unit "can recalculate its approach target" is false, the engine keeps the stale archive forever.

Similarly, the check at `TypeClass+0xD33` in `FootClass::Greatest_Threat_Scan` §3(A)ii (the "scatter allowed" flag I labeled CanBeScattered) is actually **CanApproachTarget**. That's consistent: `CanApproachTarget=yes` enables the approach-to-firing-position logic; `CanApproachTarget=no` (default on spies, per rulesmd.ini line 6836: *"this will not apply to an Attack Mission"*) disables it. The SPY / infiltrator behavior makes sense under the correct label.

## 3. Class Layout / Key Offsets

TechnoTypeClass flag triple around `+0x6AE..+0x6B0` (read by consecutive `CCINIClass::ReadBool` calls in TechnoTypeClass::ReadINI):

| Byte Offset | Name | INI Key | INI Default | Evidence |
|---|---|---|---|---|
| `+0x6AE` | MobileFire | `MobileFire=` | ? (no unit sets it in vanilla YR INI) | ReadBool result stored at `0x00714830` |
| `+0x6AF` | **OpportunityFire** | `OpportunityFire=` | `no` (INI comment at rulesmd.ini line 3524) | ReadBool result stored at `0x0071484A` |
| `+0x6B0` | DistributedFire | `DistributedFire=` | `no` (INI comment at line 3503) | ReadBool result stored after 0x0071485F |

Corrected TechnoTypeClass flag pair at `+0xD33..+0xD34`:

| Byte Offset | Name | INI Key | INI Default |
|---|---|---|---|
| `+0xD33` | CanApproachTarget | `CanApproachTarget=` | `yes` (explicit `no` only on `[SPY]` in rulesmd.ini:6836) |
| `+0xD34` | CanRecalcApproachTarget | `CanRecalcApproachTarget=` | ? (no unit sets it in vanilla YR INI) |

## 4. Core logic — what reading `+0x6AF` actually does

Six live readers in the shipping binary (confirmed by 7-byte `MOV AL, byte ptr [reg+0x6AF]` pattern search, which excludes the dialog-control-ID constant `CMP ESI, 0x6AF` false positives):

| Address | Function | Case / Branch | Behavior when `+0x6AF == 0` (OpportunityFire OFF) |
|---|---|---|---|
| `0x004D90A5` | `FootClass::Receive_Radio` @ `0x004D8FB0` | case `0x17` (arrive-at-destination) | Issues `vtable+0x174(DAT_008B3DA8, 1, 1)` — locomotor reset to null-coord sentinel. **OFF → do reset; ON → skip reset (keep firing stance).** |
| `0x007376BF` | `UnitClass::Receive_Radio` @ `0x00737430` | case `0xE` (docking / approach handshake) | After base-class delegate, issues `Receive_Radio(0x13, target)` then `Receive_Radio(0x12, ..., target)` — a two-step docking protocol. **OFF → run handshake; ON → skip, stay in combat posture.** |
| `0x00737A41` | `UnitClass::Receive_Radio` @ `0x00737430` | case `0x16` (some radio event — possibly dock-complete) | Issues `Locomotor->SetRate(0x4000)` (full fixed-point speed). **OFF → reset rate; ON → skip rate reset.** |
| `0x0073D892` | `UnitClass::Mission_Deploy_Building` @ `0x0073D630` | deploy state machine | Not disassembled in this pass — parallel behavior to §4 case 0xE expected. |
| `0x0073DF7A` | `UnitClass::Mission_Deploy_Building` @ `0x0073D630` | deploy sub-branch | `MOV AL, [ESI+0x6AF]; TEST AL, AL; JNZ`... `MOV word ptr [ESP+0x34], 0x4000`. **OFF → set rate word to 0x4000; ON → skip.** |
| `0x00740EA7` | `FUN_00740E80` (convoy leader catch-up helper) | post-loco-query | If `Locomotor->QueryInterface_slot10() == NULL` AND `+0x6AF == 0`: calls `FUN_004A51F0(convoy_coord)` — begins a move toward the convoy coordinate. **OFF → catch up to convoy; ON → stay put, keep firing.** |
| `0x00743CFD` | `UnitClass::Scatter` @ `0x00743A50` | pre-scatter gate | Condition: `if (archive_target_set AND (param_2==0 OR +0x6AF!=0)) goto SKIP_SCATTER`. **OFF + archive target → scatter; ON + archive target → skip scatter (stay on target).** |

Plus three non-gameplay readers (serialization / init):

| Address | Function | Purpose |
|---|---|---|
| `0x004D3422` | `FootClass::Constructor` | Zero-init of `+0x6AF` |
| `0x00711159` | `TechnoTypeClass::Constructor` | Zero-init of the class field |
| `0x00717791` | `FUN_007171A0` (TechnoTypeClass Save / CRC) | Hashes the three-flag triple `+0x6AE..+0x6B0` via `FUN_004A1CA0` — save/checksum, no behavior |
| `0x00714838` / `0x0071484A` | `TechnoTypeClass::ReadINI` | The key read itself (load default at `0x00714838`, store result at `0x0071484A`) |
| `0x00741233` | (no function boundary) | Likely inside an unrecognized function in the UnitClass range; not a gameplay reader |

**Pseudocode for the core behavior:**

```
// Consistent pattern across all six sites:
if (this->OpportunityFire == 0) {
    perform_mission_transition_side_effect();   // reset locomotor / dock / scatter / rate
}
// else: skip the side effect. TarCom remains bound, Fire_At pipeline continues firing.
```

**There is no Mission_Move / Mission_Attack / Fire_At / Greatest_Threat reader of `+0x6AF`.** Confirmed by searching every `[reg+0x6AF]` addressing-mode byte-pattern and finding zero hits in those functions. **OpportunityFire does not gate target acquisition.** The "fire while moving" feel described by the INI comment is an *emergent* consequence of (a) the Fire_At pipeline being mission-agnostic, plus (b) OpportunityFire=yes preventing TarCom from being cleared at mission transitions.

## 5. INI keys

| Key | Section / Scope | Type | Default | YR units that set it | Effect |
|---|---|---|---|---|---|
| `OpportunityFire` | per-unit (`[UnitType]`) | bool | `no` | `MTNK`, `HTNK`, `LTNK`, `UTNK`, `YTNK`, `AEGIS`, `DEST`, `CDEST`, `ROBO`, `HARV`, `TELE`, `MIND`, `DISK`, `SMIN` (14 units) | See §4. Documented as "Can fire at targets while performing other actions, like moving" — **the docs mislead**; see §1 and §4. |
| `MobileFire` | per-unit | bool | ? | none (grep: zero hits) | Unused in vanilla YR. Field at `+0x6AE` is read only by the save/CRC pass. TS-legacy or modder-only. |
| `DistributedFire` | per-unit | bool | `no` (INI line 3503 comment) | none found in vanilla YR | INI comment: *"whether the unit continually retargets nearby units and fires at all of them"*. Field at `+0x6B0` also appears only in save/CRC in this pass. **Likely TS-legacy or never fully wired** — needs a separate investigation to confirm. |
| `MovingFire` | per-unit | bool | `yes` (INI line 3612 comment) | none (INI key not present) | Comment: *"The vehicle does not need to stop before it can fire"*. The INI docs mention the key but no unit sets it, and the TechnoTypeClass::ReadINI I disassembled reads `"MobileFire"` (`+0x6AE`), not `"MovingFire"`. Either the docs are incorrect about the key spelling, or `MovingFire` is an alias the reader accepts elsewhere — not confirmed. |
| `CanApproachTarget` | per-unit | bool | `yes` | `SPY` (set to `no`, rulesmd.ini:6836) | Field `+0xD33`. Comment: *"9/15 Re-put in. But now this will not apply to an Attack Mission."* Gates the "scatter-to-approach" branch in `FootClass::Greatest_Threat_Scan`. |
| `CanRecalcApproachTarget` | per-unit | bool | ? | unknown (grep of rulesmd.ini returned no explicit setters in this pass) | Field `+0xD34`. Gates the retarget-abort branch in `FootClass::Greatest_Threat_Scan` §3(D). |

### Units with `OpportunityFire=yes` in rulesmd.ini (authoritative list)

Line refs from the INI scan:

- Vehicles: `[MTNK]` L6646, `[HTNK]` L7728, `[LTNK]` L8479, `[UTNK]` L8323, `[YTNK]` L8581, `[AEGIS]` L7218, `[HARV]` L8245, `[ROBO]` L7462, `[TELE]` L8633, `[MIND]` L8684, `[DISK]` L8742, `[SMIN]` L9054
- Naval: `[DEST]` L7124, `[CDEST]` L10493

Classic heavy-weapons platforms and capital ships — exactly the units you'd want to stay locked on a target while maneuvering.

## 6. Integration points

### Which mission-transition events `+0x6AF` guards

| Event | Function | What a non-OpportunityFire unit does | What an OpportunityFire unit skips |
|---|---|---|---|
| Arrived at destination (radio) | `FootClass::Receive_Radio` case `0x17` | Reset locomotor to null-coord | Reset skipped |
| Docking handshake start (radio) | `UnitClass::Receive_Radio` case `0xE` | Two-step handshake `0x13`→`0x12` | Handshake skipped |
| Post-dock rate reset (radio) | `UnitClass::Receive_Radio` case `0x16` | Force rate to 0x4000 | Rate reset skipped |
| Deploy-building transition | `UnitClass::Mission_Deploy_Building` | Set rate word to 0x4000 | Rate setup skipped |
| Scatter request | `UnitClass::Scatter` | Scatter out of current cell | Scatter skipped when an archive target is set |
| Convoy catch-up | `FUN_00740E80` | Begin move to convoy coord | Catch-up skipped |

### What it does NOT touch

- **Target acquisition** (`TechnoClass::Greatest_Threat @ 0x006F8DF0`, `Scan_Cell_For_Target @ 0x006F8960`) — not a reader of `+0x6AF`
- **Approach / firing-position search** (`FootClass::Greatest_Threat_Scan @ 0x004D5690`) — not a reader of `+0x6AF` (my earlier doc claimed the `+0xD34` reader was OpportunityFire; that was the mislabel — see §2)
- **`Fire_At` pipeline** — not a reader of `+0x6AF`; firing is mission-agnostic
- **Mission_Move** — not a reader of `+0x6AF`
- **Mission_Attack** — not a reader of `+0x6AF`

### Tick-cycle position

Every reader is in a reactive or transition handler, not the per-tick AI loop:
- Four in radio-message handlers (`Receive_Radio`) — run when a radio event arrives
- Two in mission state machines (`Mission_Deploy_Building`, `Scatter`) — run during mission transitions
- One in convoy catch-up (`FUN_00740E80`) — run when a convoy member lags

OpportunityFire has **no per-tick cost**. It's a modifier on already-infrequent events.

## 7. YR activity and TS-legacy check

**Active in YR:** **Yes, definitively.** Fourteen standard YR units set `OpportunityFire=yes` (see §5); the gating logic is in live code paths (radio / mission / scatter); none of the six readers are gated behind `SpecialFlags` or TS-only conditions.

**TS-legacy siblings in the same flag triple:**
- `MobileFire` (`+0x6AE`) — zero vanilla YR units set it; it's touched only in serialization. Possibly TS-era. Not needed for parity.
- `DistributedFire` (`+0x6B0`) — same story, zero vanilla setters. The INI comment describes continual retargeting but the field isn't read in any combat function I can see. Likely either TS-legacy or never fully implemented. Needs its own investigation if a mod uses it.

**TS-ghost watch (false leads found and dismissed):**
- The byte sequence `AF 06 00 00` (interpreted as the offset `0x6AF`) has 29 matches in the binary, but **most are false positives** — the value `0x6AF` also happens to be a dialog control ID range (`CMP ESI, 0x6AF` inside `FUN_00600CA0`, a Windows dialog helper). Only the 10 instructions above with the `MOV reg, [reg+0x6AF]` addressing-mode prefix are real field accesses.

## 8. Current Rust implementation status

Per the earlier Rust-scan pass:

| Feature | Rust status | Notes |
|---|---|---|
| `OpportunityFire` INI parsing | **Missing** — listed in `docs/gap-scans/2026-04-23-gap-scan-xref.md` as an unparsed combat-behavior key on `ObjectTypeClass` | [src/rules/object_type.rs](../src/rules/object_type.rs) has no field for it |
| Per-tick scan during movement | **Missing, but not needed** — the binary does not do this either. The Rust engine already fires at any set `attack_target` per tick; the missing piece is the *persistence* of `attack_target` across mission transitions |
| TarCom persistence through docking / arrival | **Not applicable yet** — the Rust port does not have docking, radio handshakes, or a scatter system, so there's nothing yet that *would* clear a TarCom inappropriately |
| Retaliation | **Implemented** ([combat_targeting.rs::tick_retaliation](../src/sim/combat/combat_targeting.rs#L252)) — different mechanic (hit-back when attacked) |
| Attack-move target acquisition | **Partial** — `acquire_best_target_for_entity` runs during pre-combat phase, not mid-movement |

**What implementation should look like (no code yet, just shape):**

1. Parse `OpportunityFire=yes` into `ObjectType`. Keep the spelling exactly as the INI has it.
2. In each of the six mission-transition sites (as they get implemented), *only reset the attack-target / firing state if `!OpportunityFire`*. This is a one-line guard added per site rather than a new system.
3. No per-tick scan code. No dedicated OpportunityFire module. It's a negative predicate applied to transition logic, nothing more.

This is a **much smaller** implementation task than the INI comment suggests. It's the persistence of an already-existing `attack_target`, not a new scanning system.

## 9. Open questions (follow-up pass — all resolved or narrowed)

1. ~~**`MovingFire` vs `MobileFire` spelling.**~~ **Resolved:** `MovingFire` string **does not exist** in gamemd.exe. Only `MobileFire` (at `0x00843A84`) is a real INI key. The rulesmd.ini comment at line 3612 is incorrect — the actual key is `MobileFire`, not `MovingFire`. A mod author following the INI comment would silently get the default (since no `MovingFire` key is parsed).

2. ~~**`DistributedFire`**~~ **Resolved — unwired.** Field at `+0x6B0` has **zero gameplay readers** in the binary. Byte-pattern search for every `MOV/CMP/TEST [reg+0x6B0]` addressing mode returned only:
   - `0x00714850` — load-default inside TechnoTypeClass::ReadINI (the ReadBool call's default-value plumbing)
   - `0x004DBD44` — inside FootClass::ComputeChecksum (serialization / CRC hash)
   
   No combat, mission, movement, scatter, or radio handler reads `+0x6B0`. The INI key is parsed but never observed. Likely TS-legacy or an unfinished feature. Safe to ignore for YR parity.

3. ~~**`UnitClass::Mission_Deploy_Building` reader at `0x0073D892`.**~~ **Resolved.** Disassembly:
   ```asm
   0073d892: MOV AL, [ESI + 0x6AF]    ; OpportunityFire
   0073d898: TEST AL, AL
   0073d89a: JNZ 0x0073E289           ; SKIP the reset if OpportunityFire=yes
   0073d8a0: MOV [ESI + 0xBC], 0x3    ; OTHERWISE: force field +0xBC to 3 (state reset)
   0073d8aa: ...                       ; exit sequence, return 1
   ```
   Same "OpportunityFire=yes → skip the state-clear; OpportunityFire=no → do the state-clear" pattern as the other five readers. Field `+0xBC` on the unit is a state field (low offset — probably a per-cycle timer or mission-substate byte; exact identity not verified).

4. ~~**`0x00741233` in an unrecognized function.**~~ **Resolved.** Instruction at that address IS a real `MOV AL, [ESI + 0x6AF]` read, **but the enclosing function has no Ghidra boundary**. Surrounding assembly:
   ```asm
   00741218: JZ 0x00741229                     ; branch from earlier code
   ...
   00741229: MOV AL, [ESI + 0x68D]             ; HasReachedDock flag
   0074122f: TEST AL, AL
   00741231: JNZ 0x0074125C                    ; if reached dock, skip
   00741233: MOV AL, [ESI + 0x6AF]             ; OpportunityFire
   00741239: TEST AL, AL
   0074123b: JZ  0x0074125C                    ; if OpportunityFire=no, skip
   0074123d: MOV EDX, [EBX + 0xA0]             ; else: load Weapon->Warhead (+0xA0)
   00741243: MOV EAX, [EDX + 0x2DC]            ; load Warhead+0x2DC (a warhead flag)
   00741249: TEST EAX, EAX
   0074124b: JNZ 0x0074125C                    ; if flag set, skip
   ```
   **Semantics are inverted from the other readers:** the block runs *only when* OpportunityFire IS set (and HasReachedDock is false). This is the one site where OpportunityFire *enables* extra work rather than suppressing a reset — the block looks like a weapon/warhead evaluation for units still firing while en route to a dock. Consistent with the overall meaning: "keep firing while transiting." `0x0074125C` is the common fall-through.

5. **`MobileFire` (+0x6AE) readers.** Found in this follow-up pass, not in the original report. Two gameplay sites:
   - `UnitClass::Mission_Enter @ 0x0073ADE3`: if archive target NULL AND `+0x5E0 == -1` AND **MobileFire=0**, call `vtable+0x318` with arg 1 (a state-assignment call, probably Set_Target or similar). MobileFire=yes → skip the reset. Same "preservation" pattern.
   - `FUN_0054DB00`: a small predicate function (~67 bytes) that returns 1 if `(TypeClass->+0x6AD != 0) OR (TypeClass->+0x6AE (MobileFire) != 0)` AND some other conditions. An "is this unit allowed to fire without stopping?" helper.
   
   Zero vanilla YR units set `MobileFire=yes` (confirmed by INI scan), so this code is live but inactive in standard play. Any mod setting `MobileFire=yes` would exercise these paths.

6. **Whether the TARGET_ACQUISITION archive doc should be corrected.** §2 of this doc flags the two mislabels (`+0xD33`, `+0xD34`). That is a research-archive hygiene question separate from this investigation — user decision.

7. **Whether `GREATEST_THREAT_SCAN_GHIDRA_REPORT.md` should be updated.** Same question for the in-repo doc I authored this session. The *behavior* documented there is correct; only two INI key names are wrong. Easy edit if desired.

## Sources

**Ghidra decompiled:**
- `FootClass::Receive_Radio @ 0x004D8FB0`
- `UnitClass::Receive_Radio @ 0x00737430`
- `UnitClass::Scatter @ 0x00743A50`
- `FUN_00740E80` (convoy catch-up helper)
- `FUN_007171A0` (TechnoTypeClass serialization)
- `FUN_004A51F0` (convoy-coord move helper — for context only)

**Ghidra assembly context:**
- `TechnoTypeClass::ReadINI @ 0x00712170`, specifically lines `0x00714816..0x0071485F` (the three-flag triple load sequence)
- `TechnoTypeClass::ReadINI` at `0x007144A0..0x007144D4` (the `CanApproachTarget` / `CanRecalcApproachTarget` pair)
- Byte-pattern search for all `MOV AL, byte ptr [reg+0x6AF]` encodings

**Memory reads:**
- `0x00843A64` (`"DistributedFire"`), `0x00843A74` (`"OpportunityFire"`), `0x00843A84` (`"MobileFire"`)
- `0x00843C14` (`"CanRecalcApproachTarget"`), `0x00843C2C` (`"CanApproachTarget"`)

**Docs referenced:**
- `C:/Users/enok/Documents/ra2-rust-game-docs/TARGET_ACQUISITION_GHIDRA_REPORT.md` — two mislabels (§2)
- `c:/Users/enok/Documents/ra2-rust-game/docs/GREATEST_THREAT_SCAN_GHIDRA_REPORT.md` — inherits same mislabels (§2)
- `C:/Users/enok/Documents/ra2-rust-game-docs/WAR_MINER_REFERENCE.md` — player-facing flavor ("fire while harvesting")
- `c:/Users/enok/Documents/ra2-rust-game/docs/gap-scans/2026-04-23-gap-scan-xref.md` — notes OpportunityFire as unparsed in `ObjectType`

**INI:**
- `c:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini`: `OpportunityFire=` on 14 units (§5), `CanApproachTarget=no` on `[SPY]` (line 6836)
- `c:/Users/enok/Documents/ra2-rust-game/ini/rules.ini`: identical mapping; no `OpportunityFire=` on any RA2 base unit

**Rust source:**
- [src/sim/combat/combat_targeting.rs](../src/sim/combat/combat_targeting.rs)
- [src/rules/object_type.rs](../src/rules/object_type.rs)
- [docs/gap-scans/2026-04-23-gap-scan-xref.md](gap-scans/2026-04-23-gap-scan-xref.md)
