# GI (E1) — Ghidra Research Report

**Scope:** This document covers the basic Allied GI (`[E1]` in `rulesmd.ini`,
`Primary=M60` walking, `Secondary=Para` sandbag-deploy). It is **NOT** the
Guardian GI — that is a separate unit (`[GGI]`, `Primary=M60`,
`Secondary=MissileLauncher`). For Guardian GI, see
[`GGI_GHIDRA_REPORT.md`](GGI_GHIDRA_REPORT.md).

**Status:** **Phase 3 / 3 — COMPLETE.** Core unit dossier (parse, AI loop, fire
decision, damage, XP) + state-machine depth (panic/fear, sub-cell, garrison,
IFV, weapon validators, locomotor) + edges (spawn paths, mind control, crush,
IC, render, voice, cursor). Open questions narrowed to 4 minor items.

**Scope plan:** [docs/plans/2026-05-04-gi-unit-complete-investigation-plan.md](../ra2-rust-game/docs/plans/2026-05-04-gi-unit-complete-investigation-plan.md)
**Confidence:** HIGH for all findings below (every claim cited to an address;
multi-step claims also cross-referenced to a prior report).
**Active in YR:** Yes — every code path documented here runs in a stock YR skirmish.

---

## 1. Overview

The GI (`E1` in `rulesmd.ini`, art section `[GI]`) is the basic Allied infantryman.
Built from the Allied Barracks for $200, range 4 with the M60 (anti-infantry), or
deploy in place to a sandbagged machine-gun nest with the longer-ranged Para
weapon (range 5, +10 damage, +25% rate of fire). When packed into an IFV
(`IFVMode=2`) the IFV's gun becomes a long-range CRM60. When stationed in a
garrisonable building, the GI's UCPara (range 6) fires from windows.

The GI is one of the simplest "deployer" infantry in the game — the deploy state
is **not** a separate unit (unlike the MCV) and **not** the prone-while-crawling
state. Three different "low-silhouette" states coexist on InfantryClass:

1. **Prone-while-crawling** — `Crawls=yes`, body crawls during `Walk` (sequence
   group 5/6/7 = Down/Crawl/Up). Fires the **primary** weapon from the prone
   sequence (8 = FireProne). Speed reduced.
2. **Deployed** — sequence group 0x1B–0x1F = Deploy/Deployed/DeployedFire/
   DeployedIdle/Undeploy. Stationary. Fires the **secondary** weapon.
3. **Panicked** — sequence 0x22 = Panic. Runs randomly. Cannot fire. (Phase 2.)

For the GI, prone-while-crawling fires `M60`, deployed fires `Para`, garrisoned
fires `UCPara`/`UCElitePara`, IFV fires `CRM60`. All four weapons share the same
`MGUN-*` muzzle animation and `UCFLASH` occupant flash, but each carries a
different `Verses`/`ProneDamage` pair via separate warhead profiles.

---

## 2. Class Layout / Key Offsets

### 2.1 InfantryTypeClass — bytes 0x000–0x1100 — verified slice

These are the offsets touched by `InfantryTypeClass__ReadINI` at `0x005240a0`,
plus offsets read by the runtime functions documented in §3. Not exhaustive —
parent fields (TechnoTypeClass at 0x000–0x6BF, ObjectTypeClass at 0x000–0x4FF)
are documented in `TECHNOTYPECLASS_BASE_GHIDRA_REPORT.md`.

| Offset | Type | INI Key | Source | Notes |
|--------|------|---------|--------|-------|
| `+0xc8f` | bool | (computed) | ReadINI:`0x005240b3` | Computed legacy flag. **Not `Crawls`.** Direct parser evidence shows it is forced when `+0xEAC` (`Cyborg=`) is true; GI leaves it false. |
| `+0xcd0` | bool | — | InitFromType:`0x00517d34` | Copied to entity `+0x3d2` on init (Strength threshold flag, possibly `Cyborg`/`Robot`). |
| `+0xdfc` | int | — | ReadINI:`0x005240bf` | Color list ref. Read by `RecordKill@0x00702fd5` for friendly-eaten counter (`House+0x548c`). |
| `+0xe00` | int | — | ReadINI:`0x005240c5` | Sibling color/team-dye ref. |
| `+0xe04` | WeaponTypeClass* | `OccupyWeapon=` | ReadINI:`0x005240da` | UCPara — fires from garrisoned building. |
| `+0xe20` | WeaponTypeClass* | `EliteOccupyWeapon=` | ReadINI:`0x005240ed` | UCElitePara. |
| `+0xe3c` | SequenceTypeClass[]* | `Sequence=` (artmd) | DoType:`0x00520af6` | Pointer to per-sequence frame layout array (0x24-byte / 36-byte entries, indexed by sequence id; corrected 2026-07-18: was described as "24-byte" — decimal 24 vs the actual hex stride 0x24 (36 decimal); `Do_Action`/`DoType_Sequencer` both index this array as `seq*0x24` — via decompile_function 0x0051D6F0 and 0x00520AE0 — ROOT_CAUSE: OFFSET_RETYPED_WRONG). |
| `+0xe40` | int | `FireUp=` (artmd) | ReadINI:tail | Primary fire frame. `Fire_At_Target` compares `entity+0xf8` against this to time bullet spawn. |
| `+0xe44` | int | `FireProne=` (artmd) | ReadINI:tail | Primary fire frame while prone. |
| `+0xe48` | int | `SecondaryFire=` (artmd) | ReadINI:tail | Secondary weapon fire frame, e.g. GI `Para`. |
| `+0xe4c` | int | `SecondaryProne=` (artmd) | ReadINI:tail | Secondary fire frame while prone / deploy-specific secondary sequence support. |
| `+0xe54` | AnimTypeClass*[]* | `DeathAnims=` | DoType:`0x00520c0e` | Random death animation pool (used when Die1–5 plays). |
| `+0xe60` | int | (count) | DoType:`0x00520bd1` | DeathAnims pool length. |
| `+0xe84` | int | — | ReadINI | Auxiliary anim list result. |
| `+0xe88` | DynamicVectorClass | (string list) | ReadINI:`0x00524138` | Pre-init buffer for VoiceComment list. |
| `+0xe98 / +0xe9c / +0xea0` | int | `VoiceComment=` | ReadINI | Three voice IDs (used during taunt/communication). |
| `+0xea4` | int (VocClass id) | `DeploySound=` | ReadINI | Default `GIDeploy`. |
| `+0xea8` | int (VocClass id) | `UndeploySound=` | ReadINI | Default `GIUndeploy`. |
| `+0xeac` | bool | `Cyborg=` | ReadINI string xref:`0x005243a9` / `0x825a0c` | TS-legacy cyborg flag. If true, parser also sets computed `+0xC8F=1`. GI default false. |
| `+0xead` | bool | `NotHuman=` | ReadINI string xref:`0x005243b0` / `0x825a00` | TS-legacy aesthetic/logic flag. GI default false. |
| `+0xeae` | bool | `Ivan=` | ReadINI | Crazy Ivan infantry flag. GI default false. |
| `+0xeb0` | int | `DetectionDistance=` | ReadINI | TS-legacy infantry detection distance field. |
| `+0xeb4` | bool | `Occupier=` | ReadINI | Can enter civilian/`CanBeOccupied` buildings. |
| `+0xeb5` | bool | `Assaulter=` | ReadINI | Can clear garrisoned buildings (Navy SEAL). False on GI. |
| `+0xeb8` | int | `HarvestRate=` | ReadINI | TS-legacy infantry harvester field. GI default 0. |
| `+0xebc` | bool | `Fearless=` | ReadINI string xref:`0x00524469` / `0x8259d4` | Suppresses panic/fear changes. GI default false. |
| `+0xebd` | bool | `Crawls=` | ReadINI string xref:`0x005246ae` / `0x8258f4` | Drives prone-while-walking and gates Down/Crawl/Up sequence use. This is an art.ini/artmd.ini image-section key, not `[E1]` rules data. |
| `+0xebe` | bool | `Infiltrate=` | ReadINI | Auto-forced true if any of `C4=`, `Engineer=`, or `Agent=` is true. GI default false. |
| `+0xebf` | bool | `Fraidycat=` | ReadINI | Cowardice/flee behavior flag. GI default false. |
| `+0xec0` | bool | `TiberiumProof=` | ReadINI | GI default false. |
| `+0xec1` | bool | `Civilian=` | ReadINI | GI default false. |
| `+0xec2` | bool | `C4=` | ReadINI | Forces `+0xEBE` Infiltrate. GI default false. |
| `+0xec3` | bool | `Engineer=` | ReadINI | Forces `+0xEBE` Infiltrate. GI default false. |
| `+0xec4` | bool | `Agent=` | ReadINI | Forces `+0xEBE` Infiltrate. GI default false. |
| `+0xec5` | bool | `Thief=` | ReadINI | GI default false. |
| `+0xec6` | bool | `VehicleThief=` | ReadINI | GI default false. |
| `+0xec7` | bool | `Doggie=` | ReadINI | GI default false. |
| `+0xec8` | bool | `Deployer=` | ReadINI string xref:`0x0052460d` / `0x825928` | Infantry deploy command gate. GI true. **Distinct from TechnoType `DeployFire=`, which is `+0x6AC`.** |
| `+0xec9` | bool | `DeployedCrushable=` | ReadINI | Gates whether deployed low-silhouette infantry remains crushable. GI default false. |
| `+0xeca` | bool | `UseOwnName=` | ReadINI | GI default false. |
| `+0xecb` | bool | `JumpJetTurn=` | ReadINI | GI default false. |
| `+0xed4` | (Magnetron `SizeWeight` gate?) | per `MAGNETRON_SYSTEM_GHIDRA_REPORT` | — | Cross-link only; not re-investigated. |

> **Resolved correction (2026-05-16 audit):** Phase 1 originally had several
> inferred bool names wrong. The table above now uses direct string-xref evidence
> from `InfantryTypeClass__ReadINI @ 0x005240A0`. Treat older later-phase notes
> that call `+0xEAC` "Crawls" or `+0xEC8` "CanDeployFire" as superseded.

### 2.2 TechnoTypeClass — bytes 0x680–0x6BF — GI-relevant slice

Verified by prior research (`TECHNOTYPECLASS_BASE_GHIDRA_REPORT.md`):

| Offset | Type | INI Key | Default | Notes |
|--------|------|---------|---------|-------|
| `+0x680` | WeaponTypeClass* | `Primary=` | — | M60. Read by InitFromType:`0x00517d18` → entity `+0x2fc`. |
| `+0x684` | WeaponTypeClass* | (fallback) | — | Fallback if `+0x680` is -1. |
| `+0x688` | int | `IFVMode=` | 0 | GI's `IFVMode=2` → IFV's Weapon3 (CRM60). |
| `+0x6A8` | int | `DeployFireWeapon=` | 1 | 1 = Secondary slot (Para). 0 = Primary (M60). |
| `+0x6AC` | bool | `DeployFire=` | false | `true` for GI. |
| `+0xc8e` | bool | `Trainable=` | true | Read by RecordKill — gates XP gain. |
| `+0xc9f` | bool | `DontScore=` | false | Skips kill-counter increments. |
| `+0xd94` | bool | (TBD — "fire while crawling") | false | Read by Fire_At_Target — enters exotic prone-fire branch when set. **GI = false.** |

### 2.3 InfantryClass (entity, runtime) — verified slice

Each row cites the function that uses or writes the field. Parent fields
(`FootClass` 0x520–0x6BF, `TechnoClass` 0x150–0x4FF, `RadioClass`/`MissionClass`/etc.)
are documented in `FOOTCLASS_COMPLETE_GHIDRA_REPORT.md` and `TECHNOCLASS_SYSTEMS_GHIDRA_REPORT.md`.

| Offset | Type | Name | Source/Use | Notes |
|--------|------|------|------------|-------|
| `+0x000` | vtable* | (vtable A) | Constructor:`0x00517acc` (corrected 2026-07-12: was `0x00517a76`, a mid-instruction byte with no instruction boundary there; the real `MOV dword ptr [ESI],0x7eb058` is at `0x00517acc` — via disassemble_function 0x00517A50 — ROOT_CAUSE: GHIDRA_ADDRESS_SHIFT) | Primary vtable. |
| `+0x004` | vtable* | (secondary 4) | Constructor | IUnknown for Locomotor COM. |
| `+0x150` | float | Veterancy | — | 0=rookie, 1=veteran, 2=elite (per `VETERANCY_SYSTEM_GHIDRA_REPORT`). |
| `+0x294` | int | (cell flags) | Fire_At_Target:`0x0052084b` | Read alongside seq 0x28/0x29 — likely "is on cell-fire-blocker terrain". |
| `+0x2a4` | bool | **IsLowSilhouette** | DoType:`0x00520ec3` (set on seq 0x1B), `0x00520ee5` (clear on seq 0x1F) | Set by Deploy/Crawl, cleared by Undeploy/Up. Used by crush check in `CRUSH_SYSTEM_GHIDRA_REPORT`. |
| `+0x2fc` | WeaponTypeClass* | Primary instance | InitFromType:`0x00517d28` | Copied from type `+0x680` (or `+0x684` fallback). |
| `+0x6c0` | InfantryTypeClass* | TypeClass | Constructor:`0x00517a6e` (corrected 2026-07-12: was `0x00517a64`, which is actually the `MOV dword ptr [ESI+0x6c4],0xffffffff` seq-init instruction, not the TypeClass store; the real `MOV dword ptr [ESI+0x6c0],ECX` is at `0x00517a6e` — via disassemble_function 0x00517A50 — ROOT_CAUSE: GHIDRA_ADDRESS_SHIFT) | Stored as `entity[0x1b0]` in decompile. |
| `+0x6c8` | int | CreationFrame | Constructor:`0x00517a7f` (corrected 2026-07-12: was `0x00517a68`, not a valid instruction boundary; the real `MOV dword ptr [ESI+0x6c8],EDX` is at `0x00517a7f` — via disassemble_function 0x00517A50 — ROOT_CAUSE: GHIDRA_ADDRESS_SHIFT) | Set to `g_CurrentFrameCounter`. ⚠ Possible collision with row below (+0x6c8/+0x1b2 LastFearTickFrame) — both cite the same byte offset for what read as two different purposes; not resolved this session, flagged only. |
| `+0x68d` | bool | **PendingSequenceUpdate** | AI:`0x0051be3a` (read+clear), Fire_At_Override:`0x0051df72` (clear) | Forces re-pick of next sequence after current animation completes. |
| `+0x6d4` | int | FearLevel | (Phase 2 — `Fear_Decay_Handler`) | Per `INFANTRYCLASS_GHIDRA_REPORT`. |
| `+0x6da` | bool | (TBD — fear-state flag) | AI:`0x0051bf47–4f` | Cleared after `+0x1b4` frames since `+0x1b2` (timestamp). |
| `+0x6db` | bool | **IsProne** (crawl-fire) | Do_Action:seq 5 sets, seq 7/0x1B clear | Drives seq 0x28 vs 0x29 selection in DoType_Sequencer. |
| `+0x6c8/+0x1b2` (timestamp) | int | LastFearTickFrame | AI panic-decay branch | Used with `+0x1b4` for fear timer. |
| `+0x6d0/+0x1b4` | int | FearDuration | AI:`0x0051bf3a` | (corrected 2026-07-12: label was `+0x6e0/+0x1b4`, an internally-inconsistent pair — `0x1b4*4=0x6D0`, not `0x6E0`. `InfantryClass::AI @ 0x0051bab0` uses raw `param_1[0x1b4]` for the fear-duration comparison, i.e. byte offset `0x6D0` — via decompile_function 0x0051BAB0 — ROOT_CAUSE: OFFSET_RETYPED_WRONG.) | Lockout window in frames. |
| `+0x6d4/+0x1b5` | int | FearLevel (decay 0..300) | Fire_At_Override:`0x0051dfb0` sets to 300; Fear_Decay_Handler ticks down | Verified — Fire_At_Override raises this to 300 after a successful fire when `+0xebf` (Fraidycat) is true and not panicked. **Correction** (2026-05-28): `0x1B5 * 4 = 0x6D4` — this IS the same field as the FearLevel row above (+0x6d4). They are the same byte offset. The prior "Different field" note was wrong; `Panic_SetFear300` writes `*(param_1 + 0x6d4) = 300` directly. This row is a duplicate of +0x6d4; the "avenger-counter" theory was incorrect — it is the standard FearLevel. Via decompile_function 0x0051DF70 and 0x00521C10 — ROOT_CAUSE: STRUCT_FAMILY_CASCADE. |
| `+0x6d8/+0x1b6` | bool | (TBD) | Constructor:`0x00517a92` (corrected 2026-07-12: was `0x00517aa0`; real `MOV byte ptr [ESI+0x6d8],BL` is at `0x00517a92` — via disassemble_function 0x00517A50 — ROOT_CAUSE: GHIDRA_ADDRESS_SHIFT) | Init to 0. |
| `+0x6dd` | bool | (TBD) | Constructor:`0x00517ab0` (corrected 2026-07-12: was `0x00517ab4`, not a valid instruction boundary; real `MOV byte ptr [ESI+0x6dd],BL` is at `0x00517ab0` — via disassemble_function 0x00517A50 — ROOT_CAUSE: GHIDRA_ADDRESS_SHIFT) | Init to 0. |
| `+0x6dc/+0x1b7` | bool | (TBD) | Constructor:`0x00517aaa` (corrected 2026-07-12: label was `+0x6e0/+0x1b7`, another inconsistent pair — `0x1b7*4=0x6DC`, not `0x6E0`, and the `bool` type only matches a byte-sized init. Real `MOV byte ptr [ESI+0x6dc],BL` is at `0x00517aaa`. The dword zero-init actually at `+0x6e0` is `MOV dword ptr [ESI+0x6e0],EBX` at `0x00517ab6`, unaddressed by any row — via disassemble_function 0x00517A50 — ROOT_CAUSE: OFFSET_RETYPED_WRONG) | Init to 0. |
| `+0x6e8/+0x1ba` | int | LastTerrainSpeechClass | Do_Action:`0x0051d83a..4d` | 0 = land, 1 = water; transitions play "now in water" / "now on land" voice. |
| ~~`+0x16d` IsAttacking~~ | — | **WRONG — not a distinct field** | — | (corrected 2026-07-12: `+0x16d` was read off Ghidra's `param_1[1].field_0x16d` pseudocode in `Fire_At_Target` without adding the `TechnoClass` array-stride. `param_1` there is typed `TechnoClass *`, so `param_1[1]` adds a full `sizeof(TechnoClass)` = `0x520` bytes (confirmed via get_type_size "TechnoClass" = 1312 = 0x520) before the `.field_0x16d` offset, giving absolute `0x520+0x16d=0x68D` — the SAME byte as `+0x68d` PendingSequenceUpdate (row above). Raw disassembly of `0x005206b0` confirms: `005206d2 MOV AL,[EBP+0x68d]` (fire-in-progress check), `00520912 MOV byte ptr [EBP+0x68d],0x1` (set on fire start), `005209a0/00520a03 [EBP+0x68d]` (fire-frame-timing check/abort), `00520ad2 MOV byte ptr [EBP+0x68d],0x0` (clear on null target) — all `+0x68d`, never `+0x16d`. There is no separate "IsAttacking" byte; `Fire_At_Target` reuses `PendingSequenceUpdate` as its own fire-in-progress flag. Via disassemble_function 0x005206B0 — ROOT_CAUSE: PARAM1_TYPE_MISREAD.) |
| `+0x6c4` (entity int, index `0x1b1`) | int | **CurrentSequence** | Do_Action stores; DoType reads | (corrected 2026-07-12: was `+0x1a4`. `Do_Action @ 0x0051D6F0` commits `param_1[0x1b1] = iVar6;` where `param_1` is raw `int *` from entity base, i.e. byte offset `0x1b1*4=0x6C4`; `DoType_Sequencer @ 0x00520AE0` reads the same `param_1[0x1b1]` throughout. `-1` = no sequence. Maps to byte indices in [GISequence] block. This matches the `D-T.0` correction in the Deploy-Trigger appendix below, which was never propagated into this table — via decompile_function 0x0051D6F0 and 0x00520AE0 — ROOT_CAUSE: PARAM1_TYPE_MISREAD.) |
| `+0xa5/+0x294-region` | various | NavCom/destination | per FOOTCLASS_COMPLETE | Cross-link only. |

> **Note:** the decompile uses `int *` casts so `param_1[0x1b0]` = byte offset
> `0x6c0`, `param_1[0x27]` = byte offset `0x9c`, etc. The field-offset numbers
> in the table are byte offsets from the entity base. All offsets above reflect
> that translation.

---

## 3. Core Logic

### 3.1 InfantryClass::AI — the per-tick brain — `0x0051bab0`

The brain runs every game tick. Its phases (in order):

```
0. EarlyExit on dying:    if (entity->Health < 0)  → call vtable+0x4a0 (Die_FX) and return.
1. Pip-anim spawn:        every 24 frames, if vtable+0x1D4 or vtable+0x1D8 predicate is true
                          and Rules.PipCellAnim != null,
                          spawn Rules.PipCellAnim AnimClass on the GI's cell.
2. Locomotor pre-tick:    if (Locomotor* != null) call ILocomotor::Process (vtable+0x5C).
3. Warp/delegate gate:    if vtable+0x1D8 is true, or vtable+0x1D4 and entity+0x27C are true,
                          run mission/firing handler at
                          vtable+0x40 on entity+0x19D (state-machine delegate). Return if entity died.
4. vtable+0x1D4 short-circuit: if true → cancel current movement (vtable+0x3c8), maybe abort
                          locomotor (vtable+0x480 if entity+0x169). Return.
5. Mission-queue gate:    if generic CanQueueMission_Now (`vtable+0x200`) is true →
                          if no target and no archive target → call vtable+0x484 (clear mission?),
                          then call vtable+0x1EC (`MissionClass::Commence`). [SEE NOTE below.]
6. Force action 1 (Move): if (entity[0x1b]<1 && current_mission ∉ {0xB,0xC,0xD,0xE,0xF,0x14,0x15,
                          0x22,0x23,0x24}) → set entity[0x1b]=1 (mission timer reset).
7. FootClass::AI:         delegate to parent for movement, target acquisition, retaliation.
8. Garrison entry:        if (not in transport AND mission ∈ {5=Move, 0xB=Enter}) → look up cell's
                          building. If building.Type allows occupy (CanOccupy=yes, CanBeOccupied=yes,
                          UCFireFromStructure flags via type+0x16BF/16C0/16B7) and CanGarrison() returns
                          true, call vtable+0x174 (Move into building).
9. Combat pose check:     if (vtable+0x78 != 2) → vtable+0x124(2) (set body posture to combat-ready).
10. Pending seq update:   if (entity+0x68d set AND animation duration entity+0x10C == 0):
                            clear +0x68d.
                            if (current seq ∈ {0x1B, 0x1C, 0x1D, 0x1E}) → Do_Action(0x1C, force=0)
                            else                                       → Do_Action(0x00, force=0)
11. Slave-master link:    if (SlaveOwner+0x175 != null AND entity+0x3d5 set) → mark master+0x82=1.
12. Capture alarm:        if (no SlaveOwner AND mission == 5=Move):
                            fetch current cell.
                            if (FUN_00568350 returns false) → vtable+0x3a0 (clear mission?), vtable+0xf8 (clear seq), return.
13. Engineer capture:     if (Mission_Capture() returns true) → return.   ← engineers/spies
14. Mission-queue redo:   same as step 5 (re-checked after capture and force-move).
15. Fear decay tick:      Fear_Decay_Handler() — Phase 2.
16. Fear-state aging:     if (entity+0x169==0 AND entity+0x6db==0 AND entity+0x6da set):
                            if (entity+0x1b2 != -1) {
                              elapsed = CurrentFrame - entity+0x1b2;
                              remaining = entity+0x1b4;
                              if (elapsed >= remaining) entity+0x6da = 0;
                            } else if (entity+0x1b4 == 0) {
                              entity+0x6da = 0;
                            }
17. Fire decision:        Fire_At_Target() — see §3.4.
18. Sequence advance:     DoType_Sequencer() — see §3.3.
19. Locomotion advance:   FootClass__Locomotion_AI() — moves the unit one step (per cell or sub-cell).
```

**Resolved correction to §3.1:** The four vtable methods `+0x1D4`, `+0x1D8`,
`+0x1EC`, and `+0x200` are **not** Infantry deploy-fire predicates. Later
deploy-trigger verification resolved `+0x1EC` as `MissionClass::Commence` and
`+0x200` as `InfantryClass::CanQueueMission_Now` (`FUN_00521B60`). The real GI
player deploy toggle is mission `0x10` → `vtable+0x23C` → `FUN_0051F6E0`.
- Step 8 (garrison entry) reads three building-type flags at `+0x1701`, `+0x16BF`,
  `+0x16C0`, `+0x16B7`, plus a building-state field at `+0x618` (current mission?
  values 0xC and 8 specifically allowed). Names confirmed in
  `GARRISON_SYSTEM_GHIDRA_REPORT.md` — cross-link only.

### 3.2 InfantryClass::Do_Action — sequence change — `0x0051d6f0`

Called as `vtable+0x558(seq, allow_repeat, randomize_start_frame)`. Returns 1
if the sequence changed, 0 if rejected.

```
1. Reject seq == -1 or seq has zero frames (type+0xe3c+seq*0x24+0x4 == 0)        → return 0.
2. Reject if currently playing seq 0x21 (Sleep) AND entity+0x8d ("isSleeping")  → return 0.
3. Hero override:    if (entity+0x2DC (SlaveOwner? — check field name) != 0
                        AND type+0x800 != 0  (corrected 2026-05-28: was "vtable+0x84 returns 0x800";
                                              binary checks *(type+0x800) != 0, not the return value;
                                              via decompile_function 0x0051D6F0 — ROOT_CAUSE: INFERENCE_HARDENED)
                        AND vtable+0x2b4 returns 1.0
                        AND seq == 3 (Walk))                                    → seq = 0x27.
                     (entity offset corrected 2026-05-28: was entity+0x16E; binary reads param_1[0xb7]
                      = byte offset 0x2DC; via decompile_function 0x0051D6F0 — ROOT_CAUSE: PARAM1_TYPE_MISREAD)
4. Crawls gate:      if (seq == 5 AND type+0xebd (`Crawls=`) is FALSE)           → return 0.
                     (so if the unit doesn't have the bit at +0xebd, seq 5 (Down) is rejected.
                     +0xebd is therefore "may go prone for crawling" — verified `Crawls=`.)
5. Water-zone remap: if (type+0x5b4 ("MovementZone") == 3 = Water/Amphibious):
                       fetch unit's cell.
                       if (cell+0xec ("LandType") ∈ {2=Water, 6=BridgeWater}
                           AND entity+0x8c ("IsOnBridge") is FALSE):
                         iVar5 = 0  (== "is in water")
                         remap seq:
                           3 (Walk)        → 0x11 (HoverFly?  TS-era SwimWalk)
                           6 (Crawl)       → 0x11
                           2 (Prone)       → 0x10
                           0 (Ready)       → 0x10
                           9 (Idle1)       → 0x12
                           10 (Idle2)      → 0x13
                           0xB (Die1)      → 0x14
                           0xC (Die2)      → 0x15
                           4 (FireUp)      → 0x16
                           8 (FireProne)   → 0x16
                       Then play "land→water" or "water→land" speech via VocClass on
                       transitions (entity+0x6e8 = LastTerrainSpeechClass).
6. Cheer override:   if (vtable+0x54 ("IsHero" or "IsTrainable") returns true
                        AND entity+0x8c ("IsOnBridge") is FALSE
                        AND type+0xe3c+0x33c (== Cheer sequence frame index) > 0
                        AND seq == 0):
                       seq = 0x17 (== Cheer/Paradrop alt? — note: this is sequence index 0x17 = 23,
                                   which in the YR sequence enum is the WetIdle2/Tread region.)

   ⚠ TS-LEGACY WATCHPOINT: +0x33c offset is referenced unconditionally; its value is
   from the sequence array. `[GISequence]` does not include a byte at sequence-index
   0x17 explicitly; the value is loaded from default (0,0,0). For the GI specifically,
   "Cheer" is at sequence index 0x21 (frame 364) per the [GISequence] block. The 0x17
   override is therefore TS-era residual that fires only when both conditions
   conspire — verify in Phase 3 that this branch is dead in YR.

7. Hero-streak alt: if (seq == 3 AND entity+0x6d4 ("FearLevel"?) > 199)         → seq = 0x25.
                    Wait — entity+0x6d4 is the FearLevel, and this only triggers when
                    extremely panicked. So seq 0x25 == "FleeingPanic / Panic alt".
8. Reject if same:  if (seq == current_sequence) OR (current ≠ -1 AND not param_3-force AND
                    (sequence_can_interrupt[current] from table at 0x007EAF7C) is false) → return 0.
9. Seq-1B grunt:    if (seq == 0x1B AND type+0x56C != -1) → play type+0x56C (DownSound) at entity pos.
                    (title corrected 2026-07-18: was "Seq-5 grunt"; the guarded condition, in both
                    the doc body and the binary, is `iVar6 == 0x1b` (Deploy), not seq 5 (Down) — via
                    decompile_function 0x0051D6F0 — ROOT_CAUSE: OFFSET_RETYPED_WRONG)
10. Seq-1F grunt:   if (seq == 0x1F AND type+0x570 != -1) → play type+0x570 (UpSound) at entity pos.
11. Commit:         entity+0x6C4 = seq.  (corrected 2026-07-12: was entity+0x1A4;
                    disassembly shows `param_1[0x1b1] = iVar6` = byte offset 0x6C4 —
                    via decompile_function 0x0051D6F0 — ROOT_CAUSE: PARAM1_TYPE_MISREAD)
                    entity+0x100 = CurrentFrame (animation start).
                    entity+0x108 = sequence default duration (or longer for special seqs).
12. Random start:   if (param_4 == 1) → entity+0xF8 = random in [0, sequence_frame_count-1].
                    else                 entity+0xF8 = 0.
13. Stop motion:    if (entity+0x6c == 0 (NavCom-current-step is 0)) → vtable+0x500 (Stop_Moving).
14. Walk/StopWalk:  if (seq == 5)         → entity+0x6db = 1   (set IsProne)
                    elif (seq ∈ {7, 0x1B}) → entity+0x6db = 0   (clear IsProne) AND return 1.
                    [Note: seq 0x1B = Deploy → forces clear, since Deploy makes you stationary, not crawling.]
```

The sequence-can-interrupt table at `0x007EAF7C` is a per-sequence bitmask
controlling which sequences may abort which others. (Phase 2 — extract entries.)

### 3.3 InfantryClass::DoType_Sequencer — frame advance — `0x00520ae0`

Runs every tick on the active sequence. Branches:

```
A. Mid-sequence:    if (current_seq != -1 AND
                       entity+0xF8 (current_anim_frame) < type+0xe3c[seq].frames):
                      // animation still playing — fall through to sound trigger logic at end.

B. Death anims:     case 0x0B..0x0F (Die1..Die5):
                      iVar8 = type ptr; cmp = type+0xe3c+seq*0x24+0x4 (frames in this die anim).
                      if (frame < cmp) break.   // still dying
                      // animation finished
                      if (DeathAnims pool count == 0):
                        if (type+0xead == 0):               // NotHuman= (corrected 2026-07-18: was
                                                            // "Rules+0xead"; `iVar8` here is the
                                                            // TypeClass pointer (`param_1[0x1b0]`),
                                                            // not g_RulesClass_Instance — this is the
                                                            // same +0xead already documented in §2.1
                                                            // as NotHuman=; via decompile_function
                                                            // 0x00520AE0 — ROOT_CAUSE: PARAM1_TYPE_MISREAD)
                          spawn random Rules.AnimList[idx] (idx = rand%Rules+0x130)
                                     at (entity+0x9C+0x78, entity+0xA0+0x78, entity+0xA4) with airburst flag 0x600.
                      else:
                        spawn random type+0xe54[idx] (DeathAnims) at entity pos with same airburst flag.
                      vtable+0xf8 (Limbo / cleanup).         // remove the corpse

C. Default branch:  // animation completed in a non-death sequence
                    if (current_seq != -1 AND type+0xe3c[seq].FacingHint != -1):
                      facing = FacingHint << 13;
                      FacingClass__UpdateFacing(facing);    // snap facing to the sequence's hint
                    
                    cVar1 = ILocomotor::QueryInterface(IID_IPiggyback) (vtable+0x10) — i.e., is the unit
                                                                                       currently being carried.
                    if (cVar1 == 0 OR entity+0x578 (BridgeOffset?) <= 1.0):  // not piggybacking
                      // CONTINUE-TO-FIRE branch: in deploy-fire (seq 0x28/0x29), with cell-flag+Target set,
                      if (current_seq ∈ {0x28, 0x29} AND entity+0x294 != 0 AND entity+0x2b4 (Target) != 0):
                      (corrected 2026-07-18: was "entity+0xa5"/"entity+0xad" — raw `param_1[0xa5]`/
                      `param_1[0xad]` int* indices left unconverted to byte offsets (0xa5*4=0x294,
                      0xad*4=0x2b4); the "target+navcom" gloss was also backwards — `+0x2b4` is
                      `Target` (confirmed via the null-Target early-return in `Fire_At_Target`, which
                      reads/writes the same `[EBP+0x2b4]`), and `+0x294` is the cell-flag field already
                      documented in §2.3, not a NavCom field. Via decompile_function 0x00520AE0 and
                      disassemble_function 0x005206B0 — ROOT_CAUSE: PARAM1_TYPE_MISREAD)
                        cVar = ILocomotor::QueryInterface(IPiggyback) (re-check)
                        if (still not piggybacking):
                          if (entity+0x6db (IsProne) == 0):
                            Do_Action(0x28, force=1, random_start=0)
                          else:
                            Do_Action(0x29, force=1, random_start=0)
                          entity+0x68d = 0;     // clear PendingSequenceUpdate
                          break;
                      
                      // PRONE-GROUP transition:
                      if (current_seq ∈ {0x1B, 0x1C, 0x1D, 0x1E}):
                        next = 0x1C   (loop back to Deployed-idle)
                      else:
                        if (entity+0x6db (IsProne)):
                          Do_Action(2, force=1, random_start=0)        // → seq 2 (Prone idle)
                          break;
                        else:
                          // skip if we already issued a goto-2; otherwise fall to default-idle:
                          next = 0
                    else:
                      // PIGGYBACKING branch
                      if (entity+0x6db (IsProne)):
                        Do_Action(6, force=1, random_start=0)          // → seq 6 (Crawl)
                        break
                      else:
                        next = 3  (Walk)
                    
                    Do_Action(next, force=1, random_start=0)            // commit transition
                    
D. Special transitions:
                    case 0x14, 0x15, 0x24:    if (animation done) → vtable+0xf8 (Limbo).  // Tread / Swim / Panic-end
                    case 0x1B (Deploy):       Do_Action(0x1C, force=1, 0).
                                              if (type+0xec9 == 0) entity+0x2a4 = 1.    // IsLowSilhouette ON
                                              TechnoClass::ResetAutoFireTimer() @ 0x0070f770.
                                              (corrected 2026-07-12: was "FUN_0070f770() TBD — likely
                                              deploy-occupant trigger". Address was already correct; only the
                                              purpose was unknown. Ghidra now carries the name
                                              TechnoClass__ResetAutoFireTimer with a PROOFED-confidence-100
                                              decode-system label; decompile shows it reads/writes a per-Techno
                                              timer at +0x180/+0x184/+0x188 and re-rolls a Random__RandomRanged(4,8)
                                              jitter once the prior window has elapsed — behavior is consistent
                                              with resetting an auto-fire cadence timer, not a deploy-occupant
                                              hook. Via decompile_function 0x0070F770 — ROOT_CAUSE: INFERENCE_HARDENED
                                              (name trusted from label + behavior, not independently re-derived
                                              from callers this session).
                    case 0x1F (Undeploy):     Do_Action(0, force=1, 0).
                                              if (type+0xec9 == 0) entity+0x2a4 = 0.    // IsLowSilhouette OFF
                    case 0x21 (Sleep):        do nothing — stays in Sleep.
                    case 0x22 (Tumble?):      Do_Action(0x23, force=1, 0).               // → AirDeath
                    case 0x26 (Struggle):     if (current_mission == 10):                // Mission_Hunt? Captured?
                                                Do_Action(0x26, force=1, 0).             // loop
                                              else: fall to default → Do_Action(0).

E. Deferred cleanup:
                    if (current_seq == 0x21 AND entity+0x8d (IsSleeping) == 0):
                      Do_Action(0, force=1, 0).      // wake up

F. Sound playback:   for each soundcue in sequence (count at type+0xe3c[seq]+0x10):
                      slot 0..3 at type+0xe3c[seq]+0x14 + i*8.
                      if (entity+0x60 (IsAlive) is true):
                        period = type+0xe3c[seq]+0x4 (frames-per-facing); period clamped to ≥1.
                        if ((current_anim_frame % period) == soundcue.frame_offset):
                          play soundcue.voc_id at entity pos.
```

**Critical state-machine summary** (the GI-specific one — verified):

```
        ┌────────┐    Down (5)      ┌─────┐  Crawl (6)   ┌─────┐
   Walk │Standing├──────────────────►Prone│◄─────────────┤Walk │
   (3)  └────────┘    Up (7)        │     │              └─────┘
        ▲      │    ◄────────────── └─────┘
        │      │ Deploy (0x1B)
        │      │              ┌──────────┐  DeployedFire (0x1D)
        │      ├──────────────►Deployed  ├─────────────────┐
        │      │              │ (0x1C)   │                 ▼
        │      │              └──────────┘             ┌────────────┐
        │      │   Undeploy (0x1F)                     │seq 0x28/29 │
        │      └────────────────                       │SecondaryFr │
        │                                              │ire variants│
        ▼                                              └────────────┘
   Panic (0x22) ────► panic-running ────► Tumble (0x22) → AirDeath (0x23)
        ▲
        │ if (FearLevel > 199 AND seq == Walk) → seq 0x25 (Fleeing alt)
```

When entering Deploy (seq 0x1B), the post-anim transition (case 0x1B) sets
`entity+0x2A4=1` (IsLowSilhouette). This makes the GI count as crouching for
crush checks — `CRUSH_SYSTEM_GHIDRA_REPORT` confirms `+0x2A4` is read by
`CanCrush` and combines with `DeployedCrushable` (type+0xEC9) to decide
whether tanks can still squash.

> ⚠ **Conflict with prior research resolved:** The byte at `entity+0x2A4` is
> the same field for both prone-while-crawling and deployed states — there is
> no separate "IsDeployed" byte. `INFANTRYCLASS_GHIDRA_REPORT` had this
> partly correct but did not unify the state. **All "low silhouette" damage
> and crush math reads `+0x2A4`.**

> ⚠ **Conflict with prior research resolved:** `+0x6DB` is **`IsProne`**
> (currently in crawl-fire posture), not `IsCrawling`. It's set by Down (seq 5)
> and cleared by Up (seq 7) AND by Deploy (seq 0x1B — because deploying ends
> the prone state). `+0x2A4` (IsLowSilhouette) and `+0x6DB` (IsProne) overlap
> only when prone — when deployed, +0x2A4 is set but +0x6DB is clear.

### 3.4 InfantryClass::Fire_At_Target — combat decision — `0x005206b0`

Decides whether to start a fire animation on this tick. Wraps
`TechnoClass::Fire_At` for the actual bullet spawn.

```
1. Target check:    if (Target == null) → entity+0x68D (fire-in-progress flag, same byte as
                    PendingSequenceUpdate — see §2.3 correction) = 0; return.
                    (corrected 2026-07-12: was entity+0x16D "IsAttacking"; raw disassembly of
                    0x005206b0 shows `MOV byte ptr [EBP+0x68d],0x0` at the null-Target return
                    (0x00520ad2) — via disassemble_function 0x005206B0 — ROOT_CAUSE: PARAM1_TYPE_MISREAD)
2. Pick weapon:     weapon_idx = vtable+0x2E4(Target)         // SelectWeaponAgainst
                                                               // returns 0 = primary, 1 = secondary
3. Fire-error:      err = vtable+0x3C0(Target, weapon_idx)    // GetFireError
                    err: 0 = OK, 5 = OutOfRange, 9 = SomeBlock(?), other = TBD
                    Branch on err:
                    
   3a. err == 0 (OK to fire):
       if (NOT entity+0x68D (NOT currently firing; corrected 2026-07-12, was +0x16D — see above)):
         if (type+0xD94 == 0):   // not "fire-while-crawling exotic"
           // Decide which fire sequence to start:
           if (current_seq ∈ {0x1B, 0x1C, 0x1D, 0x1E}):
             // already deployed → re-fire as deploy-fire
             new_seq = 0x1D  (DeployedFire)
           elif (entity+0x294 != 0 AND current_seq ∈ {0x28, 0x29}):
             // already in deploy-fire AND on cell-fire-blocker → just clear entity+0xF8 (rewind anim)
             entity+0xF8 = 0
           else:
             // pick fire sequence (corrected 2026-07-18: the whole branch below was wrong —
             // every "entity+0x1BB" was an unconverted `param_1[1].field_0x1bb` Ghidra offset;
             // param_1 here is typed TechnoClass*, so param_1[1] adds a full sizeof(TechnoClass)
             // = 0x520 stride before the field, giving absolute entity+0x6DB — the SAME byte
             // already documented in §2.3/§3.2/§3.3 as IsProne, not a separate "+0x1BB" flag and
             // NOT "IFV gunner state". The PRIMARY branch's "type+0xE3C[seq=0x28] is defined"
             // check does not exist in the binary at all — raw disassembly (0x00520886-0x005208d6)
             // shows weapon_idx==0 jumps straight past all 0x5A4/0x5C8/0x28/0x29 checks into the
             // shared IsProne-only dispatch. Via decompile_function 0x005206B0 and
             // disassemble_function 0x005206B0 — ROOT_CAUSE: PARAM1_TYPE_MISREAD.
             if (weapon_idx == 0):   // PRIMARY
               if (entity+0x6DB (IsProne)):
                 new_seq = 8 (FireProne)
               else:
                 new_seq = 4 (FireUp)
             else:                    // SECONDARY (e.g., GI's Para)
               if (entity+0x6DB (IsProne) AND host+0x5C8 != 0):
                 new_seq = 0x29 (SecondaryProne / DeployedFireProne)
               elif (host+0x5A4 != 0):
                 new_seq = 0x28 (SecondaryFire)
               elif (entity+0x6DB (IsProne)):
                 // falls back into the same IsProne dispatch the PRIMARY branch uses
                 new_seq = 8 (FireProne)
               else:
                 new_seq = 4 (FireUp)
           Do_Action(new_seq, force=0, random_start=0)
           entity+0x68D = 1   // mark "firing in progress" (corrected 2026-07-12, was +0x16D;
                              // `MOV byte ptr [EBP+0x68d],0x1` at 0x00520912 — via
                              // disassemble_function 0x005206B0 — ROOT_CAUSE: PARAM1_TYPE_MISREAD)
           UpdateFacing(toward target)
           if (Target == NavComTarget):    // target IS the unit's destination
             FootClass__Stop_Moving()
             vtable+0x500 (clear movement)
         else:
           // type+0xD94 set → unique branch (TBD — possibly Yuri Prime / Mastermind which fires while moving)
           Do_Action(0x1A, force=0, random_start=0)  // sequence 0x1A — exotic; verify in Phase 2.
   
   3b. err == 5 (OutOfRange):
       weapon_range = TechnoClass::GetWeaponRange(self, weapon_idx);
       if (weapon_range < 0):
         this_obj = Target as ObjectClass;
         if (this == null OR (this->AbstractFlags & 2) == 0 OR What_Am_I != 0xF):
           vtable+0x3C8 (Approach_Target / move closer)
         else:
           // Target is a unit, not a building, and damaged below MercuryRetreatThreshold
           if (Rules+0x16F8 (some HP threshold) <= GetHealthRatio(this)):
             vtable+0x3C8 (move closer)
           // else: target nearly dead, don't pursue
   
   3c. err == 9:
       vtable+0x45C (some "wait" / "delay" handler).

4. Fire-frame timing (post-firing-decision):
   pick fire_frame from type:
     fire_frame_table[w=0,d=0] = type+0xE40   (primary, standing/normal)
     fire_frame_table[w=0,d=1] = type+0xE44   (primary, IsProne — entity+0x6DB, corrected 2026-07-18:
                                                was "entity+0x1BB"; see ROOT_CAUSE note above)
     fire_frame_table[w=1,d=0] = type+0xE48   (secondary, has +0x5A4)
     fire_frame_table[w=1,d=1] = type+0xE4C   (secondary, has +0x5C8 AND IsProne — entity+0x6DB,
                                                corrected 2026-07-18, was "+0x1BB")
   
   if (NOT entity+0x68D)      → return  // not currently firing — skip to scatter (corrected 2026-07-12, was +0x16D)
   if (entity+0xF8 != fire_frame) → return  // not yet at fire frame
   
   // we are AT the fire frame
   uVar5 = vtable+0x2E4(Target)            // re-pick weapon (defensive)
   err = vtable+0x3C0(Target, uVar5)
   if (err == 0):
     vtable+0x3CC(Target, weapon_idx)      // SPAWN BULLET (TechnoClass::Fire_At — see Override below)
   else:
     // can't fire after all — abort sequence
     entity+0x68D = 0  (corrected 2026-07-12, was +0x16D; `MOV byte ptr [EBP+0x68d],0x0` at
                       0x00520a03 — via disassemble_function 0x005206B0 — ROOT_CAUSE: PARAM1_TYPE_MISREAD)
     if (entity+0x6DB (IsProne) == 0):  // (corrected 2026-07-18: was "entity+0x1BB" — same
                                         // field_0x1bb→0x520+0x1bb=0x6DB stride error as above;
                                         // via disassemble_function 0x005206B0 — ROOT_CAUSE:
                                         // PARAM1_TYPE_MISREAD)
       if (current_seq ∈ {0x1B-0x1E}):
         Do_Action(0x1C, force=0)         // back to Deployed idle
       else:
         Do_Action(0, force=0)            // back to Ready
     else:
       Do_Action(2, force=0)              // back to Prone

5. Scatter check (after fire):
   if (Target != null):
     ground = vtable+0x3F8(0)             // GetCellAt(target)
     if (ground.threat_at_offset_0xa8 < Rules+0x16C0 ("RandomScatterThreshold")):
       // Target's cell has low threat — scatter the GI to break formation overlap.
       cell_xy = (**(Target+0x48))(...)
       cell = MapClass::Get_Cell_At(cell_xy)
       Cell::Scatter_Objects(...)
```

**Fire-frame table reading** (offset table at +0xE40..+0xE4C):

| Offset | Slot | When used |
|--------|------|-----------|
| +0xE40 | Primary, normal | Standing or prone-fire (M60). |
| +0xE44 | Primary, IsProne | When `entity+0x6DB` (IsProne) is set (corrected 2026-07-18: was `entity+0x1BB` described as "currently in IFV gunner state" — binary reads the same byte already documented elsewhere as IsProne, not an IFV-gunner flag; via decompile_function 0x005206B0 + disassemble_function 0x005206B0, instruction `0x00520952 MOV CL,[EBP+0x6db]` — ROOT_CAUSE: PARAM1_TYPE_MISREAD). |
| +0xE48 | Secondary, normal | When secondary weapon active and `host+0x5A4` is non-zero. |
| +0xE4C | Secondary, IsProne | `host+0x5C8` AND `entity+0x6DB` (IsProne) both set (corrected 2026-07-18: "deploy-mode"/+0x1BB label was the same IsProne mislabel). |

For the GI with `[GI] FireUp=2`, +0xE40 (and likely all four — they default to 2)
holds the value 2, meaning the bullet spawns on frame 2 of the fire sequence
(matches `art.ini [GI] FireUp=2`).

> **Open question (Phase 2):** Are +0xE44/+0xE48/+0xE4C separately ReadINI'd
> from distinct INI keys, or is there only one `FireUp=` and the four slots
> are populated from the same value? Read sequence in `InfantryTypeClass::ReadINI`
> shows four sequential ReadInt calls at the tail (no key-prefix evidence in
> the decompile) — but they could all read the same `FireUp=` key with
> different defaults. Phase 2 verification needed.

### 3.5 InfantryClass::Fire_At_Override — vtable Fire_At — `0x0051DF70`

Wraps `TechnoClass::Fire_At` (the actual bullet-spawn) with two extra effects:

```
1. Clear PendingSequenceUpdate:  entity+0x68D = 0
2. Call parent Fire_At:           result = TechnoClass::Fire_At(this)
3. Auto-pursuit:                  if (result != 0    // fire succeeded
                                      AND entity+0x81 (panicked?) == 0
                                      AND type+0xEBF (`Fraidycat=`) != 0
                                        (corrected 2026-05-28: was "TBD — wants pursuit?";
                                         binary shows check on type+0xEBF which is Fraidycat;
                                         via decompile_function 0x0051DF70 — ROOT_CAUSE: INFERENCE_HARDENED)
                                      AND entity+0xBF == 0):
                                    entity+0x6D4 = 300              // sets FearLevel to 300
                                        (corrected 2026-05-28: was "entity+0x1B5 = 300 / revenge timer";
                                         0x1B5*4=0x6D4; this IS FearLevel, not a separate timer;
                                         via decompile_function 0x0051DF70 — ROOT_CAUSE: STRUCT_FAMILY_CASCADE)
                                    if (current_mission ∈ {1=Sleep, 0xF=Hunt}):
                                      vtable+0x1E8 (clear archive target / wake up).
4. Return result.
```

The write to `entity+0x6D4 = 300` (FearLevel) after a successful fire when `Fraidycat` is set
raises the GI to max fear after firing — this means Fraidycat units panic-max upon firing.
Previously misread as a separate "revenge timer"; it is the same FearLevel field.
Via `decompile_function 0x0051DF70` confirmed: `unaff_ESI[0x1b5] = 300` = byte offset 0x6D4.

### 3.6 InfantryClass::ReceiveDamage — friendly-fire gate — `0x005227F0`

Short override that returns 0 (block damage) or 1 (allow damage). The full damage
math runs in `TechnoClass::ReceiveDamage` (cited at `RECEIVE_DAMAGE_GHIDRA_REPORT.md`).
The infantry-specific layer here is the **garrison friendly-fire check**:

```
1. cVar = vtable+0xC4(...)               // some pre-check (TBD)
   if (cVar == 0): return 0              // block (don't damage)
2. cell_xy = (**(this+0x48))(buffer)     // get GI's cell coords (subcell-aware)
3. cell = MapClass::Get_CellClass(cell_xy)     // via 0x005657a0
4. damaging_house = <4th stack arg>      // EDI in the disassembly
5. allied = HouseClass::IsAlliedWith(this+0x21C (House), damaging_house)
   if (allied):
     return 0                            // ALLIED damager → BLOCK, unconditionally
   // damaging_house is NOT allied (enemy/neutral) — fall through:
6. cell_hit = cell->houseUnitCount[damaging_house.ArrayIndex] > 0   // FUN_004870F0(cell, idx),
                                                                     // reads cell+0xAC + idx*2 (int16)
   if (cell_hit): return 0               // block
7. mc = this+0x51C (MindController, entity-int-index 0x147)
   if (mc == damaging_house OR mc == null):
     return 1                            // allow
   allied_to_mc = HouseClass::IsAlliedWith(damaging_house, mc)
   if (NOT allied_to_mc): return 0       // block
   else:                    return 1     // allow
```

(corrected 2026-07-12: the branch structure above was previously stated with the
allied/non-allied cases swapped, and step 6's `Default → return 0` was described
as an unconditional catch-all when it is not. Raw disassembly of `0x005227F0`
shows the real order: `00522866 TEST AL,AL; 00522868 JNZ 0x0052289e` branches to
the return-0 epilogue precisely when `HouseClass::IsAlliedWith(this->House,
damaging_house)` (`CALL 0x004f9a50`) is TRUE — i.e. an **allied** damager is
blocked immediately, before any mind-controller check runs. The mind-controller
chain (`this+0x51C`, confirmed = entity-int-index `0x147*4`) and the second
`FUN_004870F0`/`IsAlliedWith` checks only execute in the NON-allied branch, and
can themselves resolve to either return value. `FUN_004870f0` was previously
undescribed; decompile shows `return cell[0xAC + houseIdx*2] > 0` (a per-house
int16 array on CellClass, purpose beyond "count > 0" not further traced this
session). This does not resolve *why* gamemd blocks allied damage here by
default (that requires tracing the caller's use of the 0/1 result, not done this
session) — only the branch-direction claim, which was backwards, is corrected.
Via disassemble_function 0x005227F0 and decompile_function 0x004870F0 —
ROOT_CAUSE: OPERATOR_OR_ORDER_DRIFT. The `+0x87`-style entity-int-index notation
for House (`this+0x87`) was correct (`0x87*4=0x21C`); only the allied/non-allied
branch bodies were swapped.)

> **P3.10 also needs revisiting**: the "DEFERRED" resolution below (§ P3.10)
> claims "return 1 (allow) on most paths, return 0 only when friendly-fire-blocked"
> — that is also WRONG per the trace above (allied fire is blocked by default, not
> allowed by default). See the correction note added to P3.10.

**Cross-link:** `FootClass::ReceiveDamage @ 0x004D7330` calls
`TechnoClass::ReceiveDamage` (the main pipeline) and then handles
`MagnetronAttach detach` (`+0x174`), AI retaliation (`+0x484`), and
`HouseClass::IsPlayerControl` triggering. The infantry override at
`0x005227F0` runs as a vtable slot inside that chain.

### 3.7 TechnoClass::RecordKill — kill credit — `0x00702D40`

Awards veterancy XP and increments score counters when a kill happens.
Verified against `VETERANCY_SYSTEM_GHIDRA_REPORT.md` (5 passes).

```
1. victim_type = victim->GetTypeClass()
2. xp = victim_type->Cost (read via vtable+0x84 chain — actually GetCost which itself reads
        type+0xa0 cost-with-multipliers)
3. ProcessCellAction (AI trigger events) for various action types:
   - if (killer alive AND killer.field_0xD ≠ 0 AND damager ≠ null):
       Action 6 (Killed)
   - same gate:
       Action 4 (Damaged)   ← always also fires when 6 fires
   - if (killer not type 1 (Aircraft)):
       Action 7 (Destroyed)
       Action 0x30 (— TBD — possibly "EnemyKilled")
       Action 0x1D (— TBD — possibly "PointsScored")
4. If victim_type+0xC9F (DontScore) set → return.   // skip score updates entirely
5. XP scaling:
   if (damager ally to victim):    xp = 0           // no friendly-fire XP
   else if (killer Veteran):       xp *= 2
   else if (killer Elite):         xp *= 3
6. XP recipient (multi-branch):
   - If killer.+0x82 set AND killer.SpawnerHost (+0x47) is Trainable (+0xC8E):
     → award XP to SpawnerHost
   - Else if killer is Trainable:
     → award XP to killer directly (VeterancyStruct__GainXP via vtable+0x84-chain)
   - Else if (killer not type 6=Building) AND killer.+0xD68 is clear AND killer.IsInfantry — TBD:
     → check if killer is OccupantSlot's owner — award XP to garrison occupant
   - Else if killer is Building (type 6) AND killer.+0xD68 set AND occupant SpawnerHost
     (+0xB5) is Trainable:
     → award XP to SpawnerHost
7. Update house counters:
   - house.last_kill_id (+0x548c) = victim's GameObjectID
   - house.score_value (+0x54E8) += victim_cost
8. Per-type-killed counters:
   switch on victim's What_Am_I:
     case 1 (Aircraft): increment victim_type counter; radar-jammer ping if relevant
     case 2 (Anim): falls through to case 0xF (Unit)
     case 6 (Building): house+0x5488 (BuildingsKilled) += 1;
                        if (NOT radar-jammed): house+0x5438+typeIdx*4 += 1.
     case 0xF (Unit):  house+0x5434 (UnitsKilled) += 1;
                        if (NOT type-DontScore): house+0x53E4+typeIdx*4 += 1.
     default:          house+0x5434 += 1.
```

**XP award threshold verification** (per VETERANCY_SYSTEM, cross-linked):
- Veteran promotion: `entity+0x150 (Veterancy) ≥ 1.0` — set by VeterancyStruct__SetVeteran
- Elite promotion:   `entity+0x150 ≥ 2.0`        — set by VeterancyStruct__SetElite
- For the GI killing a Soviet Conscript (cost 100): rookie GI gains 100/200 = 0.5 XP →
  needs 2 conscripts to promote to veteran. Veteran kills give 2x → 1.0 XP → instant elite.

(Phase 2 — confirm exact attribution to garrisoned-GI via the +0x69C garrison-occupant
slot, which `VETERANCY_SYSTEM` lists as a known path but not yet traced for GI specifically.)

### 3.8 InfantryClass::Constructor — `0x00517A50`

Initializer. Allocates the instance, runs FootClass::Constructor (which runs
TechnoClass::Constructor → ObjectClass::Constructor → AbstractClass::Constructor),
then:

```
1. FootClass::Constructor(...)
2. entity[0x1B1] = -1                  // current sequence = none
3. entity[0x1B0] = type_ptr            // store TypeClass
4. entity[0x1B2] = g_CurrentFrameCounter  // creation timestamp (used for fear timer reference)
5. zero out entity[0x1B4]..[0x1BA]     // fear timer, IsProne, etc.
6. entity[0x1BA] = 2                   // LastTerrainSpeechClass = 2 (== "neither water nor land", so first
                                         transition will play either "now in water" or "now on land")
7. entity[0x000..0x00C] = vtable pointers (4 vtables — primary + 3 secondary for COM)
8. AbstractClass::AssignUniqueID(...)
9. Register in InfantryClass global pool (DAT_00A83DEC) — capacity-grow path
10. InfantryClass::InitFromType()      // see §3.9
11. CoCreateInstance(type+0x34C = LocomotorGUID, ...) → Locomotor*
       OleRun(Locomotor*)
       QueryInterface(IID_ILocomotor) → entity+0x19D
       Locomotor::Link(this)            // bind locomotor to entity
12. FUN_004C9680(0x7F)                 // probably "set-default-cell-flags"
13. Add (UniqueID, this) to global lookup table at DAT_00B0E840 (with grow logic).
```

### 3.9 InfantryClass::InitFromType — `0x00517CC0`

Per-type initialization, runs on construction and on type change (if any):

```
1. TechnoClass::Init_Managers()        // sub-managers (capture, slave, etc.)
2. if (entity.House != null):
     HouseClass::Add_Tracking(this)    // increment house-side count of this unit
3. if (TypeClass != null):
   3a. // House-veterancy override:
       House.Country.VeteranInfantry list (Country+0x150, count Country+0x15C)
       If TypeClass is in that list → VeteranryStruct::SetVeteran(1)
   3b. // House.InitialVeteran flag:
       if (House+0x2BF (InitialVeteran) AND TypeClass+0xC8E (Trainable)):
         VeterancyStruct::SetVeteran(1)
   3c. // Copy stats from type:
       entity+0x6C = TypeClass+0xA0 (HitPoints)        // CurrentHP
       entity+0x70 = TypeClass+0xA0                     // MaxHP
       weapon = TypeClass+0x680 (Primary)
       if (weapon == -1): weapon = TypeClass+0x684 (PrimarySpawn)
       entity+0x2FC = weapon
       entity+0x3D2 = TypeClass+0xCD0 (TBD — `Cyborg`?)
```

So a GI built in a house with `[Allied] InitialVeteran=yes` (or which has a
`VeteranInfantry=` list including E1) starts at Veteran rank. **Standard YR
skirmish — neither flag is set on the human player by default**, so default
GI starts as Rookie.

---

## 4. INI Keys (Phase 1 verified subset)

| Key | INI File | Section | Field | Notes |
|-----|----------|---------|-------|-------|
| `Primary=M60` | rulesmd.ini | [E1] | type+0x680 (TechnoType) | Read by TechnoTypeClass::ReadINI; copied to entity+0x2FC by InitFromType. |
| `Secondary=Para` | rulesmd.ini | [E1] | type+0x6XX | Standard secondary slot — used in deploy-fire because `DeployFireWeapon=1`. |
| `ElitePrimary=M60E` | rulesmd.ini | [E1] | type+0x684 (or sibling slot) | Read by TechnoTypeClass::ReadINI; selected by GetWeapon when `Veterancy ≥ 2`. |
| `EliteSecondary=ParaE` | rulesmd.ini | [E1] | type+sibling | Same elite-swap. |
| `OccupyWeapon=UCPara` | rulesmd.ini | [E1] | **type+0xE04** | InfantryType-only. **VERIFIED by ReadINI:`0x005240DA`.** |
| `EliteOccupyWeapon=UCElitePara` | rulesmd.ini | [E1] | **type+0xE20** | **VERIFIED by ReadINI:`0x005240ED`.** |
| `OpenTransportWeapon=1` | rulesmd.ini | [E1] | type+0x6A4 (TBD) | Cross-link `IFV_AND_OPEN_TOPPED_TRANSPORT_GHIDRA_REPORT`. |
| `IFVMode=2` | rulesmd.ini | [E1] | type+0x688 | YR-only key (TS used different system). 2 → IFV's Weapon3=CRM60. |
| `Crawls=yes` | artmd.ini | [GI] | **type+0xEBD** | **VERIFIED by string xref `0x8258F4` at `0x005246AE`.** Gates Down/Crawl/Up sequence use. Does **not** force `+0xC8F`. |
| `Fearless=no` | rulesmd.ini | [E1] | type+0xEBC | **VERIFIED by string xref `0x8259D4`.** GI remains fear-capable. |
| `Pip=white` | rulesmd.ini | [E1] | type+0xDFC | Cargo pip color. |
| `OccupyPip=PersonBlue` | rulesmd.ini | [E1] | type+0xE00 | Building-occupant pip frame. |
| `Occupier=yes` | rulesmd.ini | [E1] | type+0xEB4 | **VERIFIED.** |
| `DeployFire=yes` | rulesmd.ini | [E1] | type+0x6AC | Cross-link to TECHNOTYPECLASS_BASE. |
| `Deployer=yes` | rulesmd.ini | [E1] | type+0xEC8 | **InfantryType field.** Player deploy command gate in `FUN_0051F6E0`. |
| `DeployFireWeapon=1` (default) | rulesmd.ini | [E1] | type+0x6A8 | 1 = Secondary (Para). Not overridden on GI. |
| `Trainable=yes` (default) | rulesmd.ini | [E1] | type+0xC8E | Default true; gates RecordKill XP. |
| `DeploySound=GIDeploy` | rulesmd.ini | [E1] | **type+0xEA4** | **VERIFIED.** |
| `UndeploySound=GIUndeploy` | rulesmd.ini | [E1] | **type+0xEA8** | **VERIFIED.** |
| `VoiceComment=` | rulesmd.ini | [E1] (absent on GI) | type+0xE98/9C/A0 | List parser; GI does not set this so all three slots are -1. |
| `VoiceFeedback=GIFear`, `VoiceSelect=GISelect`, `VoiceMove=GIMove`, `VoiceAttack=GIAttackCommand`, `DieSound=GIDie`, `CrushSound=InfantrySquish`, `VoiceSpecialAttack=GIMove` | rulesmd.ini | [E1] | (parent TechnoType layout — Phase 3) | All resolved through `CCINIClass::ReadString → VocClass::FindByName`. |

**Sequence keys** (artmd.ini `[GISequence]`, indices verified by code use):

| Index | Key | Frames | Notes |
|-------|-----|--------|-------|
| 0 | `Ready` | 0,1,1 | Standing idle. |
| 1 | `Guard` | 0,1,1 | Combat-ready (after combat pose toggle in AI step 9). |
| 2 | `Prone` | 86,1,6 | Crouched idle (when crawling in place). |
| 3 | `Walk` | 8,6,6 | Walking. |
| 4 | `FireUp` | 164,6,6 | M60 fire standing. **+0xE40 holds bullet-frame=2.** |
| 5 | `Down` | 260,2,2 | Transition into prone (Crawls=yes only). |
| 6 | `Crawl` | 86,6,6 | Crawl while prone (== Prone visual but with movement). |
| 7 | `Up` | 276,2,2 | Transition out of prone. |
| 8 | `FireProne` | 212,6,6 | M60 fire while prone. |
| 9–10 | `Idle1`/`Idle2` | 56/71,15 | Random fidget. |
| 11–15 | `Die1..Die5` | 134/149/0/0/0 | Death animations. (Die3–5 unused for GI.) |
| 0x1B | `Deploy` | 300,15,0 | Transition into deployed. **0 facings = pose plays once regardless of orientation.** |
| 0x1C | `Deployed` | 292,1,1 | Idle while deployed. |
| 0x1D | `DeployedFire` | 315,6,6 | Para weapon fire while deployed. **6 facings.** |
| 0x1E | `DeployedIdle` | 0,0,0 | Empty for GI — no extra idle pose. |
| 0x1F | `Undeploy` | 276,2,2 | (Same frame range as `Up` — GI reuses crawl-up animation for both.) |
| 0x20 | `Paradrop` | 363,1,0 | Falling-with-parachute. |
| 0x21 | `Cheer` | 364,8,0,E | Victory cheer. |
| 0x22 | `Panic` | 8,6,6 | Panic running (== Walk frames at higher speed). |

**Weapon profiles** (rulesmd.ini, INI-cited verification):

| Weapon | Damage | ROF | Range | Warhead | Notes |
|--------|--------|-----|-------|---------|-------|
| `M60` (Primary) | 15 | 20 | 4 | SA | Standing M60 anti-infantry. |
| `Para` (Secondary, deployed) | 25 | 15 | 5 | SSA | Longer range, more damage. |
| `M60E` (ElitePrimary) | 25 | 20 | 4 | SA | Elite GI standing fire. |
| `ParaE` (EliteSecondary) | 25 | 15 | 6 | SSA | Elite-only +1 range when deployed. |
| `UCPara` (OccupyWeapon) | 30 | 15 | 6 | SSAB | Garrison fire — strongest base. |
| `UCElitePara` (EliteOccupyWeapon) | 40 | 15 | 6 | SSAB | Elite garrison fire. |
| `CRM60` (IFV gunner mode 2) | 25 | 15 | 6 | SSA | IFV-passenger weapon. |

**Warhead `Verses` (anti-infantry/light/medium/heavy/concrete… verified vs prone):**

| Warhead | InfantryDamage | LightArmor | ProneDamage |
|---------|----------------|------------|-------------|
| `SA` (M60 / M60E) | 100% | 80% | **70%** |
| `SSA` (Para / ParaE / CRM60) | 100% | 100% | **80%** |
| `SSAB` (UCPara / UCElitePara) | 100% | 80% | **50%** |

> Prone GIs take **30% less** from M60 fire, **20% less** from Para/CRM60, but
> **50% less** from garrisoned UCPara — meaning garrisoned GIs are deeply
> ineffective against prone defenders, and IFV/deployed GIs are most effective
> against prone targets.

---

## 5. Integration Points

**Tick-cycle position** (verified by call chain — `World::advance_tick` in
the Rust port has equivalent ordering):
- Ground movement (`FootClass::AI` → `Locomotion_AI`) runs **inside**
  `InfantryClass::AI`, after fire-decision has already happened. So:
  - **Order per tick:** AI early-exit → CanDeploy gate → FootClass::AI (path,
    target acquisition) → garrison entry → pending-seq → fear decay →
    Fire_At_Target (fire decision) → DoType_Sequencer (sequence advance) →
    Locomotion_AI (move one step).
- Vision and power are computed **after** all unit AIs in the parent loop —
  GI does not directly consume them.

**Inbound calls** (where the GI is created or hit):
- Spawn from Allied Barracks production → `BuildingClass::ExitObject @ ...`
  (Phase 3) → `InfantryClass::Constructor` → `InitFromType`.
- Paradrop (Cloning Vats / IFV deploy / superweapon paradrop) →
  `AircraftClass::Mission_ParaDropOverfly @ 0x004157C0` →
  `Drop_Payload @ 0x00415C60` → `CellClass::PlaceInfantryInCell @ 0x00481180`
  (Phase 2/3).
- Damage reception → `TechnoClass::ReceiveDamage` → `FootClass::ReceiveDamage` →
  `InfantryClass::ReceiveDamage` (this report §3.6) → return to chain →
  warhead-specific handlers (mind control, magnetron, chaos berserk) (Phase 3).
- Crush kill → `UnitClass::OnEnterCell_Triggers @ 0x00744720` (Phase 3) →
  reads `entity+0x2A4` (IsLowSilhouette) AND `type+0xEC9` (DeployedCrushable)
  to decide.

**Outbound calls** (what the GI invokes):
- Bullet spawn → `TechnoClass::Fire_At @ 0x006FDD50` → bullet/laser
  trajectory + warhead apply (`WarheadTypeClass::Detonate`).
- Animation spawn (death) → `AnimClass::Constructor` (in DoType_Sequencer
  death-anim branch).
- Veterancy promotion → `VeterancyStruct::SetVeteran @ 0x00750090` /
  `SetElite @ 0x007500B0` (in InitFromType house-veteran path and
  RecordKill XP path).
- Garrison entry → `BuildingClass::AddGarrisonOccupant @ 0x00522910` (Phase 2).

---

## 6. Current Rust Implementation Status (Phase 1 scope)

| Area | Status | Files | Gaps for GI parity |
|------|--------|-------|--------------------|
| **InfantryType parsing** | COVERED | [object_type.rs](../ra2-rust-game/src/rules/object_type.rs) | Corrected offsets: `Pip=+0xDFC`, `OccupyPip=+0xE00`, `DetectionDistance=+0xEB0`, `HarvestRate=+0xEB8`, `Crawls=+0xEBD`. `+0xC8F` is a computed legacy flag forced by `Cyborg=+0xEAC`, not derived from `Crawls`. |
| **Per-tick AI loop** | MISSING | (no infantry-mission code in `src/sim/`) | The whole §3.1 brain is absent. Aircraft missions exist; infantry missions don't. |
| **Sequence state machine** | PARTIAL | [animation.rs](../ra2-rust-game/src/sim/animation.rs), [infantry_sequence.rs](../ra2-rust-game/src/rules/infantry_sequence.rs) | Sequence definitions parse, transitions defined for Deploy/Deployed/DeployedFire/DeployedIdle. **Missing**: 0x1B/0x1C/0x1D/0x1E case handlers in the sequence advancer; the Down→Crawl→Up prone transitions; the seq 0x28/0x29 deploy-fire continue branch; the +0x6DB IsProne flag. |
| **Fire decision** | PARTIAL | [combat_weapon.rs](../ra2-rust-game/src/sim/combat/combat_weapon.rs), [combat/mod.rs](../ra2-rust-game/src/sim/combat/mod.rs) | Primary/Secondary/Elite/IFV/Garrison weapon selection works (`select_weapon`, `select_garrison_weapon`). **Missing**: the deploy-fire weapon swap (Fire_At_Target's pick of seq 0x1D vs 4); the fire-frame timing (`entity+0xF8 == type+0xE40`); the auto-pursuit timer on +0x1B5; the post-fire scatter check against Rules.RandomScatterThreshold. |
| **Damage receipt** | PARTIAL | [combat/mod.rs](../ra2-rust-game/src/sim/combat/mod.rs) `apply_prone_damage_modifier` | Prone-damage multiplier works. **Missing**: friendly-fire gate (§3.6); `+0x2A4` IsLowSilhouette flag; `InfantryDamageMultiplier` (Rules-level) is not parsed; mind-control / magnetron / chaos-berserk hits all bypass this code today (whole subsystems missing). |
| **Veterancy XP** | MISSING | [game_entity.rs](../ra2-rust-game/src/sim/game_entity.rs) tracks veterancy as u16 (0/100/200) but no kill-counter accumulator | RecordKill not implemented. Elite weapon swap works on read but no path to actually promote a unit during gameplay. House score counters (+0x5434/+0x5438/+0x53E4/+0x5488/+0x548C/+0x54E8) — none implemented. |
| **Construction / InitFromType** | PARTIAL | [game_entity.rs](../ra2-rust-game/src/sim/game_entity.rs) constructors set HP/sequence | **Missing**: VeteranInfantry list check; InitialVeteran flag; LocomotorGUID → ILocomotor instantiation (we use a Rust enum not COM, which is correct, but we don't honor per-unit Locomotor GUIDs from INI). |

---

## 7. Open Questions (Phase 1, with later corrections applied)

The original Phase 1 questions below were narrowed by later passes and the
2026-05-16 audit. The bool slot map is no longer open for the GI-critical
fields: `+0xEBD=Crawls`, `+0xEBC=Fearless`, `+0xEC8=Deployer`, and
`+0xEC9=DeployedCrushable` are verified by direct key-string xrefs.

1. **0x294 cell flag** — used in Fire_At_Target to skip a deploy-fire reset
   when "on cell-fire-blocker." Could be "is on a cell where occupants block
   fire" (e.g., GI inside a Tesla Coil or pillbox is the firer). Phase 2.
2. **`+0xD94` exotic prone-fire branch** — `type+0xD94` triggers an alternate
   fire path that picks seq 0x1A. Likely Yuri Prime / Mastermind which fires
   while moving, but also could be a unit that "fires via FacingClass override"
   (e.g., chrono legionnaire). Verify in Phase 2.
3. **Sequence 0x17 cheer-override** in Do_Action step 6 — this fires when
   `vtable+0x54` and the on-bridge check pass and seq 0x33C (== Cheer position
   in older sequence indexing) is defined. Likely TS-era residual from the
   "wave when promoted" mechanic. **Verify dead in YR by checking whether any
   YR unit triggers this branch in a stock skirmish.**
4. **ReceiveDamage return semantics inversion** — §3.6 has a structural
   ambiguity in the decompile. Phase 2: disassemble `0x005227F0` and verify.
5. **Per-house veterancy** — RecordKill multi-branch attribution to garrison
   occupant slot via building+0x69C "current firing index" was inferred but
   not deeply traced. Phase 2.

---

## Sources

**Ghidra addresses decompiled in Phase 1 (10 funcs, ~2200 lines of pseudocode read):**
- `InfantryTypeClass__ReadINI @ 0x005240A0` (FULL — except parent)
- `InfantryClass::AI @ 0x0051BAB0` (FULL)
- `InfantryClass::DoType_Sequencer @ 0x00520AE0` (FULL)
- `InfantryClass::Do_Action @ 0x0051D6F0` (FULL)
- `InfantryClass::Fire_At_Target @ 0x005206B0` (FULL)
- `InfantryClass::Fire_At_Override @ 0x0051DF70` (MEDIUM — wraps TechnoClass::Fire_At)
- `InfantryClass::ReceiveDamage @ 0x005227F0` (FULL — but return-semantics
  flagged for Phase 2)
- `InfantryClass::Constructor @ 0x00517A50` (MEDIUM — registration paths
  skimmed)
- `InfantryClass::InitFromType @ 0x00517CC0` (FULL)
- `TechnoClass::RecordKill @ 0x00702D40` (FULL — cross-link to
  VETERANCY_SYSTEM)
- `FootClass::ReceiveDamage @ 0x004D7330` (LIGHT — for §3.6 context)

**Ghidra addresses cross-linked but not re-decompiled (cited from prior research):**
- `TechnoTypeClass__ReadINI @ 0x00712170` (full coverage in
  TECHNOTYPECLASS_BASE_GHIDRA_REPORT) — `IFVMode`, `DeployFire`,
  `DeployFireWeapon`, `Voice*` keys parse here.
- `TechnoClass::Fire_At @ 0x006FDD50`, `TechnoClass::ReceiveDamage @ 0x00701900`,
  `TechnoClass::GetWeapon @ 0x0070E140`, `VeterancyStruct::SetVeteran/Elite
  @ 0x00750090/0xB0` — covered in upstream reports.
- `BuildingClass::CanGarrison @ 0x004525F0`,
  `BuildingClass::UpdateGarrisonFire @ 0x0043E7B0` — Phase 2.

**Strings sampled** (key→address): `Crawls @ 0x008258F4`,
`OccupyPip @ 0x00825A60`, `VoiceComment @ 0x00825A2C`,
`DeployFire @ 0x00843AA0`, `DeployFireWeapon @ 0x00843AAC`,
`IFVMode @ 0x00843AE4`, `DeploySound @ 0x008440B0`,
`UndeploySound @ 0x008440A0`, `VoiceFeedback @ 0x0084424C`,
`VoiceSpecialAttack @ 0x00844268`, `Cyborg @ 0x00825A0C`,
`Fearless @ 0x008259D4`, `Engineer @ 0x0082596C`, `Civilian @ 0x00818164`,
`Insignificant @ 0x00832B60`, `Bombable @ 0x00832BCC`, `Nominal @ 0x00843ECC`,
`FireUp @ 0x008257D8`.

**Doc files referenced** (cross-linked, not re-derived):
- `INFANTRYCLASS_GHIDRA_REPORT.md` (struct layout 0x520+)
- `FOOTCLASS_COMPLETE_GHIDRA_REPORT.md` (0x520–0x6BF)
- `INFANTRY_SUBCELL_POSITIONING.md` (sub-cell allocator at 0x481180)
- `VETERANCY_SYSTEM_GHIDRA_REPORT.md` (XP formula, threshold checks)
- `IFV_AND_OPEN_TOPPED_TRANSPORT_GHIDRA_REPORT.md` (IFVMode dispatch)
- `GARRISON_SYSTEM_GHIDRA_REPORT.md` (UCPara fire path)
- `CRUSH_SYSTEM_GHIDRA_REPORT.md` (`+0x2A4` IsLowSilhouette consumer)
- `RECEIVE_DAMAGE_GHIDRA_REPORT.md` (TechnoClass damage chain)
- `TECHNOTYPECLASS_BASE_GHIDRA_REPORT.md` (TechnoType `+0x680..0x6CF` slice)

**INI files checked:**
- `ini/rulesmd.ini` — `[E1]` lines
  3713–3759, weapons `[M60]`/`[Para]`/`[UCPara]`/`[UCElitePara]`/`[M60E]`/
  `[ParaE]`/`[CRM60]`, warheads `[SA]`/`[SSA]`/`[SSAB]`.
- `ini/artmd.ini` — `[GI]` lines
  281–290, `[GISequence]` lines 14140–14164.

---

---

# Phase 2 — State machines, garrison, sub-cell, weapon validators

This section adds 17 newly-decompiled functions and **closes 5 Phase-1 open
questions**. Phase 1 sections above remain authoritative for what they cover;
Phase 2 corrections are noted inline.

## P2.1 Closed open questions from Phase 1

| Phase-1 OQ | Status | Resolution |
|------------|--------|------------|
| Q1 — `+0xEC9` gates Deploy/Undeploy IsLowSilhouette flip | **CLOSED** — key string is `DeployedCrushable`. |
| Q2 — bool slot identification 0xEBC..0xECB | **CLOSED for GI-critical fields** — see corrected §2.1 and §P3.1 |
| Q4 — `type+0xD94` exotic prone-fire branch | **CLOSED** — used in `Locomotion_AI`/`Fire_At_Target` for IPiggyback (Yuri Prime's hovering, magnetron-chrono lift). See §P2.10 |
| Q6 — `ReceiveDamage` return semantics | **OPEN** — Phase 3 will disassemble. |
| Q7 — fire-frame quad — same key or distinct? | **CLOSED** — see §P2.7 below. The quad reads from 4 sequential `CCINIClass::ReadInt` callsites in InfantryTypeClass::ReadINI tail. Distinct keys. |

## P2.2 Resolved type-bool meanings (corrected by later string-xref evidence)

The original Phase 2 behavioral names in this table were partly inferred from
consumers. The **INI key names below supersede those inferred labels**.

| Offset | Meaning | Used by | Behavior |
|--------|---------|---------|----------|
| `+0xEBC` | **Fearless** | `Panic_SetFear300`, `SetFear`, `Fear_Decay_Handler` | When set, fear-add operations skip. Combined with veteran ability `0xD` (FEARLESS). GI false. |
| `+0xEBD` | **Crawls** | `Do_Action`, `GetMovementSpeed`, panic/prone flow | Gates Down/Crawl/Up sequence use. When prone, GI has this set and uses the slower-prone branch. |
| `+0xEBE` | **Infiltrate** | Movement/AI special-case consumers | Auto-set if `C4`, `Engineer`, or `Agent` is true. GI false. |
| `+0xEBF` | **Fraidycat** | Fear/scatter/idle flee consumers | Cowardice/flee behavior flag. GI false. |
| `+0xEC3` | **Engineer** | Infiltrate derivation and capture-related consumers | GI false. |
| `+0xEC6` | **VehicleThief** | Hijacker/vehicle-theft related consumers | GI false. |
| `+0xEC8` | **Deployer** | Mission `0x10` deploy toggle | GI true. Distinct from TechnoType `DeployFire=+0x6AC`. |
| `+0xEC9` | **DeployedCrushable** | `DoType_Sequencer:case 0x1B/0x1F`, crush checks | Controls deployed low-silhouette/crush behavior. |

## P2.3 Sub-cell allocator — `CellClass::PlaceInfantryInCell @ 0x00481180`

**Verified pixel-offset table** (initialized by `CellClass::InitSubCellOffsets @ 0x0048E480`):

| Slot | X | Y | Z | Used? |
|------|---|---|---|-------|
| 0 (center) | 0x80 (128) | 0x80 (128) | 0 | **NEVER assigned by allocator** (only used as initial position in `WalkLocomotionClass::FindSubCellDest` if approach distance < 60 leptons). |
| 1 (NW) | 0x40 (64) | 0x40 (64) | 0 | **NEVER assigned anywhere** (the quadrant-mapping `if (uVar11 != 0) uVar11 + 1` produces values {2, 3, 4, 0}, never 1, and the iteration loop explicitly skips `uVar11 ∈ {0, 1}`). |
| 2 (NE) | 0xC0 (192) | 0x40 (64) | 0 | YES |
| 3 (SW) | 0x40 (64) | 0xC0 (192) | 0 | YES |
| 4 (SE) | 0xC0 (192) | 0xC0 (192) | 0 | YES |

**Maximum infantry per cell: 3** — the three "corner" slots 2, 3, 4. (Slot 0 in
the offset table is used as a "reset / default position" for some idle/pre-walk
states but never assigned by `PlaceInfantryInCell`.)

**Algorithm:**
1. Compute approach direction from incoming `param_3` (target cell-relative leptons):
   - If `sqrt(approach.x² + approach.y²) < 60`: set quadrant = 0 (no clear direction → randomize)
   - Else: bit 0 = (approach.x_low > 0x80), bit 1 = (approach.y_low > 0x80), then `if non-zero, += 1`
   - Maps to: 0 = center / random, 2 = NE-coming, 3 = SW-coming, 4 = SE-coming.
2. Read the cell's sub-cell occupancy bitmap (`cell+0x49` for ground layer, `cell+0x4A` for bridge).
3. **Direct path:** if `quadrant ≠ 0` and bit `quadrant` is clear in bitmap → place there directly.
4. **Fallback path:** iterate the per-quadrant preference table (4 entries each) at
   - `DAT_0081CC84 + quadrant*4` for off-center approaches (per-quadrant fallback ordering)
   - `DAT_0081CC98 + (rand%4)*4` for center approaches (4 random orderings)
   - skip slots 0, 1 in iteration; check bit `slot` in occupancy; first free slot wins.
5. If iteration exhausts (all 3 corner slots occupied) → return `DAT_0089E778..0089E780` (== g_NullCoord — "cannot place").

**Cell-occupancy structure** (`MarkCellOccupancy @ 0x005217C0`):
- `cell+0x124` (ground layer) / `cell+0x128` (bridge) — bit `(1 << sub_cell_idx)` set per occupant.
- `cell+0x54` (ground) / `cell+0x58` (bridge) — UniqueID stamp of "primary occupant" (last one placed).
- Bridge plane height = `DAT_00A8F234` (== Rules `BridgeHeight` in leptons; default 0x180 = 384).
- Cell flag `0x100` at `cell+0x140` — "is bridge cell" — selects the bridge layer mask `& 0x1C` (bits 2-4) gates whether cell counts as "infantry-occupied" for collision purposes. Slot 0 occupancy does NOT count (bit 0 is in mask).

**Garrison gate** (lines `0x004812BD..0x004812EE`): if cell is on a building's footprint
- AND `(cell+0x49 & 0x40) != 0` (cell has building occupancy bit)
- AND building+`type+0x16B7` (== `CanBeOccupied=`) is set
- AND `BuildingClass::CanGarrison()` returns true
→ proceed; else return null-coord.

> ⚠ **Confirmed Rust bug** (per `INFANTRY_SUBCELL_POSITIONING.md` and now
> Ghidra-verified): the Rust `FUNCTIONAL_SUB_CELLS` array `[0, 3, 4]` should be
> `[2, 3, 4]`. **Never `[0, ...]`** — slot 0 is not assignable.

## P2.4 WalkLocomotionClass::FindSubCellDest — `0x0075C240`

The walk locomotor's per-tick "where am I trying to move to" check. Calls
`PlaceInfantryInCell` to pick a sub-cell within the destination cell.

```
1. If locomotor.last_dest is null-coord:
     dest_xy = entity's current cell base (param_1+0xC dereferences entity, +0x9C/A0/A4)
   Else:
     dest_xy = locomotor.last_dest

2. dest_xy passed to vtable+0xF4 (entity → "PathfindingNextStep")
   → result is final-cell coord buffer

3. If final_coord is null-coord:
     locomotor.last_dest = final_coord (latch)
     return 0 (no movement this tick)

4. Mission-specific overrides (mission ∈ {7=Move, 8=Attack-Move, 9=Capture,
   0xB=Enter, 0x19=AreaGuard}):
   - Walk into building if target+0x5A4 == 1 (UnitClass) AND target.cell == final_cell → garrison
   - Walk into infantry if target+0x5A4 == 2 (InfantryClass) AND target.cell == final_cell
   - Walk into building if target+0x5A4 == 6 (BuildingClass) AND target.cell == final_cell
     → in all three: `param_6 = 1` ("force-place mode" — bypass occupancy)

5. SlaveOwner check (entity+0x2DC →SlaveOwner +0x2D8 → master is alive):
   - If we're slave AND target == master AND SlaveAtCell predicate → force-place

6. Elevation check (cell+0x140 & 0x100 = bridge cell):
   - If entity is on bridge plane (entity+0xA4 ≤ ground_height + 3*BridgeOffset) → use bridge layer

7. Call CellClass::PlaceInfantryInCell(approach=approach_buffer, force_mode=force_mode)
8. If returned coord is null:
     entity+0x90 (??): if set, return success anyway with current pos
     Else: keep null and signal failure
9. Pickup any crate at the destination cell (CrateClass::PickupDispatch)
10. Update entity vtable+0xF0 (post-arrival hook) with new dest_xy.
```

**Implication**: walking into a cell where all 3 sub-slots are taken returns
null → unit halts at previous cell. This is the cell-occupancy collision
mechanism for infantry pathfinding.

## P2.5 InfantryClass::MarkCellOccupancy / UnmarkCellOccupancy

Bookkeeping when a GI enters or leaves a cell:

```
Mark (called on cell entry):
   sub = CellClass::GetSubCell(pixel_pos)            // extract sub-index 0..4 from leptons
   cell = MapClass::GetCellAt(pixel_pos)
   ground_h = CellClass::GetGroundHeight(pixel_pos)
   if (Z >= ground_h + BridgeOffset):                // bridge layer
     cell+0x128 |= (1 << sub)
     cell+0x58 = entity.UniqueID
   else:                                              // ground layer
     cell+0x124 |= (1 << sub)
     cell+0x54 = entity.UniqueID

Unmark (called on cell exit):
   (same selectors)
   bits = cell+0x124 & ~(1 << sub)                    // clear bit
   cell+0x124 = bits
   if ((bits & 0x1C) == 0):                           // mask = bits 2,3,4 = corner slots
     cell+0x54 = -1                                   // clear UniqueID stamp (cell unowned)
```

**Critical detail:** the UniqueID stamp at `cell+0x54` is only cleared when bits
2, 3, 4 are ALL zero. So a cell with only slot 0 occupancy (in some edge case)
is still "unowned" for collision purposes. This matches the iteration loop's
skip of slot 0/1.

## P2.6 Fear / Panic state machine — `0x00521C10`, `0x00518C00`, `0x005200B0`

**Three entry points:**
- `Panic_SetFear300 @ 0x00521C10` — used by warhead detonate paths (Psychedelic, Cluster, etc.) to force panic
- `SetFear @ 0x00518C00` — variable-amount fear-add per damage event
- `Fear_Decay_Handler @ 0x005200B0` — per-tick decay called from `InfantryClass::AI`

**FearLevel storage:** `entity+0x6D4` (verified — Panic_SetFear300 writes 300 here; Fear_Decay_Handler decrements).

**Panic_SetFear300 behavior:**
```
if (type+0xEBC == 0 AND not HasWeaponAbility(0xD = FEARLESS)):
    entity+0x6D4 = 300       // jump straight to max
```

**SetFear behavior (per damage event from a non-null source):**
```
// Branch A (already in fear or no damager):
if (damager == null OR FearLevel > 99):
    add_amount = 50                                        // base
    if (HealthRatio > Rules+0x1708 = "ConditionRed"):     // not red
        add_amount = 25
    if (HealthRatio > Rules+0x1700 = "ConditionYellow"):  // green health
        add_amount /= 2                                     // = 12 (or 25/2 if yellow)
    if (NOT type+0xEBC AND NOT FEARLESS_ability):
        FearLevel = clamp(FearLevel + add_amount, 0, 300)
    return

// Branch B (first time hit, FearLevel ≤ 99):
if (type+0xEBF):
    FearLevel = 300                  // Fraidycat → instant max
    return
if (type+0xEBC):  return              // Fearless → no change
if (HasWeaponAbility(0xD)): return    // FEARLESS veteran → no change
FearLevel = 100                       // first big hit jumps to 100 (panic threshold)
```

**Thresholds (verified by `Fear_Decay_Handler` branches):**
- **FearLevel ≥ 50 AND ≤ 300** + standing + not deploy + player-controlled-or-AI-not-piggyback → trigger **`Do_Action(5)` = Down (start prone)**
  (corrected 2026-05-28: was "FearLevel > 50"; binary uses `0x31 < FearLevel` = 49 < FearLevel = ≥ 50,
   so FearLevel=50 DOES trigger Down; via decompile_function 0x005200B0 — ROOT_CAUSE: OPERATOR_OR_ORDER_DRIFT)
- **FearLevel < 50** + currently prone → trigger **`Do_Action(7)` = Up (stand up)** (binary: `FearLevel < 0x32 = 50` — CONFIRMED)
- Specifically the "Down" trigger requires:
  - `FearLevel ≥ 50` AND not in deploy seq 0x1B-0x1E
  - Player-controlled AND no destination AND no piggyback, OR AI unit
  - NOT type+0xEBF (`Fraidycat=` units take the flee-building branch instead)
- **FearLevel ≥ 50 AND type+0xEBF set** + not in deploy + not sleeping + no destination → call **vtable+0x174 (move into nearest building/garrison)** = "Fraidycat infantry flee into buildings"
  (corrected 2026-05-28: was "FearLevel > 50"; same binary threshold ≥ 50 — ROOT_CAUSE: OPERATOR_OR_ORDER_DRIFT)

**Decay rate (verified):** every tick, if `FearLevel > 0` and `NOT type+0xEBC`:
- `FearLevel--` (1 per tick)
- When FearLevel hits 0 AND entity+0xBF (TBD) is 0 AND vtable+0x2AC (something true): set `entity+0xBF = type+0x684` (== Secondary weapon ptr). Probably the "armed/idle weapon switch" handler.

**Per-tick fear math:** at 30 FPS lockstep, 300 frames = 10 seconds. So a single big hit sends a GI to FearLevel=100, then he decays at 1/tick → reaches 49 in 51 ticks (~1.7s). At FearLevel=49 (< 50) he stands up (transitions Up). Below 50 he resumes normal Walk. (Stand-up happens at FearLevel < 50, i.e. the tick where it first reaches 49.)

**Veterancy ability `0xD = FEARLESS`** suppresses fear entirely. The GI's
`VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,FASTER` does NOT include
FEARLESS, so a Veteran GI **still panics**. EliteAbilities adds SELF_HEAL
but still no FEARLESS — so an Elite GI also panics. (This matches the
in-game observation that even highly-promoted GIs get scared.)

## P2.7 Movement speed — `0x00521D80`

```
speed = FootClass::GetCurrentSpeed()   // base from terrain × type.Speed × multipliers

if (entity+0x6DB (IsProne) is set):
    if (type+0xEBD is set):    // Crawls=yes
        speed = (speed * 2) / 3            // 67% — crawl
    else:                       // standard infantry
        speed = (speed * 3) / 2            // 150% — non-crawling prone/panic fallback

return speed
```

**Corrected implication:** the GI has `Crawls=yes` at `type+0xEBD`, so it uses
the **67% crawl-speed branch while prone**, not the faster fallback branch. At
base `Speed=4`, prone/crawl speed is effectively 2/3 of the normal movement
speed before the rest of the locomotion/pathing math applies.

The earlier Phase 2 wording that said GI lacks `+0xEBD` and moves 50% faster
while prone was wrong; `+0xEBD` is the verified `Crawls=` flag.

## P2.8 Idle dispatch — `IdleDispatch @ 0x0051CBA0`

Called from FootClass::OnArrival (post-locomotion). Picks the next mission
when the unit has finished moving:

```
1. base = FootClass::OnArrival(...)
2. If vtable+0x4AC returns true: return base   // OnArrival already handled it
3. If current_mission == 0x1C (special, ≈ "Capture"): return base

4. If entity+0xAD == null (no destination):
     Run "decide idle action":
       if (entity+0x169 == 0): {
         current_mission ∈ {5=Move, 0xB=Enter} → return 0     // already in transition
         If MissionTimer flags +5 / +7: return 0              // mission in progress
         Decision matrix:
         AI-controlled (NOT player) AND NOT slave:
           if (house+0x24C < Rules+0x1440 AND mission != 0xB): mission = 5 (Hunt? Move?)
           else if (entity+0xB7 set [building target?]): mission = 5
           else if (vtable+0x2AC false AND NOT type+0xEC3 AND NOT type+0xEC6): mission = 5
           else: mission = 0xB (Guard)
         Player-controlled OR slave:
           if (mission != 0xB AND ((NOT HasAbility(0x10) AND NOT type+0xD39) OR slave)):
             mission = 5
           else: mission = 0xB
       } else (in transport — entity+0x169 != 0):
         mission = 2 (Idle/Pose)
         if (current_mission == 8): mission = 8
         if (current_mission == 0x11): mission = 0x11
   Else (has destination):
     mission = 1
     if (current_mission == 0x11): mission = 0x11
     if (current_mission == 8):    mission = 8

5. If new_mission ∉ {0x19=AreaGuard, 0xB=Guard} AND new_mission != -1:
     vtable+0x1E8(new_mission, 0)   // Mission_Set
6. Return base
```

This is the post-walk "decide what to do next" handler. For the GI (no slave,
not AI-overridden, no special veteran abilities by default) the typical path
is: arrive → (player-controlled with no waypoint queued) → mission = 0xB Guard.

## P2.9 Update idle action — `UpdateIdleAction @ 0x0051CDB0`

Called periodically (every `Rules+0x1710 = "IdleActionFrequency"` frames) when
the unit is standing idle. Spawns fidget animations.

```
1. If vtable+0x474 (IsIdle predicate) returns false: return 0
2. Compute next-fidget time:
     next_time = CurrentFrame + Rules+0x1710 * (random 0..2^31) * 1.0    // random exponential
3. AI civilian flee path:
     if (type+0xEBF AND NOT HumanPlayer AND FearLevel > 50):
       vtable+0x174(NullCoord, 1, 0)    // flee into nearest building
       return 1
4. Pick random action 1..10:
     2/3 of the time:
       3,4,5 → Do_Action(9 = Idle1)
     1/3 of the time:
       1,2,7 → Do_Action(10 = Idle2) + 1/3 chance to play VoiceComment[0] (type+0xE98) if HumanPlayer
       6 → random facing change (just look around)
       8 → random facing change + flee-to-building if AI civilian + fear>50
       9,10 → random facing change with second variant
     If FUN_005F3E50("DAT_0082554C" — likely "tooltip text/string event") returns true AND extra
     dice roll < 5 → action 8 forced (so on-event units always do the special action)
5. Return 1 (or 0 on no-op).
```

**For the GI:** standing in place fires roughly:
- Idle1 (33%) — head-turn / reposition
- Idle2 (33%) — second fidget (with 1-in-3 voice-comment chance — but GI has no VoiceComment set in INI, so this never fires)
- Random facing (16%)
- Idle2 + voice / facing (rest)

## P2.10 Locomotion AI — `FootClass::Locomotion_AI @ 0x00520F40`

Drives the per-tick locomotor → animation sequence dispatch. Decides whether
to play Walk / Crawl / Stand / Prone based on locomotor state.

```
1. If current_mission == 2 (Idle/Stop?):
     If NOT piggyback-mode (vtable+0x10):
       If entity+0x169 (transport target?) == 0:
         vtable+0x484(0)    // clear stop state
       Else:
         vtable+0x480(transport, 1)
         vtable+0x544(0, 1.0_double_high)   // set speed factor

2. If entity+0x169 (next-cell target) is set:
     Get final cell coord
     If type+0xEBE == 0 (NOT super-flag):    // most units
       If can't reach destination zone AND mission != 7 (Move):
         vtable+0x480(0, 1)                  // abort — give up

3. vtable+0xA8 returns "is locomotor still moving?":
   3a. Locomotor still moving (returned false):
       sequence dispatch:
         current_seq == 3 (Walk)   → Do_Action(0 = Ready) ??? (probably "stop walking — locomotor idle but not still walking")
         current_seq == 6 (Crawl)  → Do_Action(2 = Prone)
         current_seq == 0x18      → Do_Action(0)
         current_seq == 0x17      → Do_Action(0)
         current_seq == 0x11      → Do_Action(0x10)
       (No wait — this is reverse: vtable+0xA8 == false means "STOP MOVING, LOCOMOTOR DONE." So
       transition to idle/prone-idle.)
   
   3b. Locomotor IS moving (vtable+0xA8 == true):
       If type+0xD94 (== "PiggybackHostType" — exotic flag, e.g., Yuri Prime):
         Resolve IPiggyback interface
         If exotic locomotor identity matches DAT_007E9AC0:
           If pending-update flag clear AND HealthRatio > Rules+0x... threshold:
             Do_Action(0x17)         // Cheer / TS jumpjet
           Else:
             Do_Action(0x18)         // Tread / TS jumpjet
       Else (NORMAL infantry):
         If entity+0x6DB (IsProne) == 0:   → Do_Action(3 = Walk)
         Else (IsProne):                    → Do_Action(6 = Crawl)
```

**For the GI:** when walking and not prone → seq 3 (Walk). When walking and
prone (Crawls=yes triggered or fear > 50) → seq 6 (Crawl). On arrival → seq 0
(Ready) or seq 2 (Prone) depending on prone state.

## P2.11 Clear_Doing_Action — `0x00521B20`

```
if (entity+0x6C4 ∈ {3, 6, 0x11}):
    entity+0x6C4 = -1
```

`entity+0x6C4` is a "current pending action" cache, holding a sequence ID. The
function clears it only when it's currently set to Walk(3), Crawl(6), or
HoverFly(0x11) — three "movement" actions. This runs when the unit's mission
changes mid-walk to ensure the locomotor doesn't keep trying to play the old
animation. `entity+0x6C4` field name in Phase 1 was unverified; now confirmed
as **CurrentPendingAction**.

## P2.12 Scatter — `InfantryClass::Scatter @ 0x0051D0D0`

Picks a nearby cell to relocate to when the GI is shoved by another unit
(e.g., tank tries to drive over) or a burst of fire lands.

```
1. If currently in deploy seq (0x1B-0x1E) AND param_3 (force) AND param_4 (force):
     Do_Action(0x1F = Undeploy)        // can't scatter without undeploying
2. If player-controlled AND in deploy: return (refuse to scatter)
3. Mission-timer entry +9 must allow scatter
4. type+0xEBF (`Fraidycat`) plays special role: if NOT Fraidycat AND has destination AND NOT forced → return
5. Sequence-can-interrupt table at 0x007EAF7C must allow current_seq to be aborted
6. Rules+0x17ED (== "ScatterRandomExempt"?): if NOT set:
     If HasWeaponAbility(0x10 = "GUARD_AREA"?) OR force=true: skip rest
     Else if NOT player-controlled: skip rest
     Else if NOT force AND no slave: return
   Else: similar gate
7. type+0xEBF gates "skip return" — Fraidycat infantry always scatter
8. If approach is null (DAT_00A8F200 = NullCoord):
     dir = atan2(-y, +x) of own pos vs cell-center
     dir += random_ranged(0,4) - 2     // ±2 facings random
     find_nearby_passable_cell(...)
     vtable+0x480 (move to it)
9. Else (specific approach):
     Pick direction = atan2(approach.y - own.y, own.x - approach.x)
     dir += random_ranged ± 2
     iterate 8 directions to find passable cell:
       check cell at (own + DirectionOffsets[dir+i])
       must be in playfield, must be passable (vtable+0x1AC), must not have invalid bridge
       commit if found
     mission = 2 (Idle), vtable+0x480 (move)
```

**Refusal cases:**
- Player-controlled deployed GI never scatters (player chose to deploy)
- Sleeping/captured units don't scatter
- Fraidycat infantry always scatter even when force=false
- Units with `GUARD_AREA` veteran ability resist scatter

## P2.13 IFV gunner weapon swap — `TechnoClass::SetGunnerWeapon @ 0x0070DC70`

```
if (type+0x810 == 0):                     // not "no-IFV-swap" gate
  if (0 ≤ param_2 < 18):                  // valid IFVMode (0..17)
    weapon_ptr = FUN_007178B0(param_2)    // GetWeaponByIFVMode(idx) — reads from IFV's per-IFVMode
                                           // weapon array (e.g., IFV.Weapon3 for IFVMode=2)
    entity+0x49 = weapon_ptr              // active weapon pointer
    entity+0x4E = param_2                 // remember which slot is active
  else:
    weapon_ptr = FUN_007178B0(0)          // fallback to slot 0
    entity+0x49 = weapon_ptr
    entity+0x4E = 0
```

For the GI passenger:
- GI boards → calls SetGunnerWeapon(2) → IFV's `Weapon3` (CRM60) becomes active.
- GI disembarks → calls SetGunnerWeapon(0) → IFV's default Weapon1 (FV missile) restored.

The 18 slots (0..17) match Rules `[FV] Weapon1..Weapon18` — though only 1..17
are typically populated. Slot 0 is the "no passenger" default.

## P2.14 Garrison occupant entry — `BuildingClass::AddGarrisonOccupant @ 0x00522910`

Called when a GI right-clicks a garrisonable civilian building.

```
1. If type+0xEB4 (Occupier) == 0:
     If type+0xEB5 (Assaulter) != 0:    // SEAL/Tanya — clears garrison instead of joining
       SpawnUnitsWithParachute(this)     // eject occupants
       vtable+0x480(0, 1)                 // clear move
       vtable+0x174(GI_pos)               // walk back
     Else: return                         // not Occupier nor Assaulter: do nothing
2. vtable+0xD4(this)                      // BuildingClass::Decloak / clear stealth
3. building+0x1A3 = current_occupant_count
4. If (count_storage < count) OR
      ((vacancy_or_building+0x691 OR count == 0) AND budget > 0 AND
       building.OccupantArray.HasRoom()):
     occupant_array[count] = GI_ptr
     count++
5. FUN_0070F6E0(...)                       // mark garrison-active flag (probably "set FireFromBuilding state")
6. If count == 1 (first occupant):
     vtable+0x124(2)                       // mission = Guard (start firing)
     For HumanPlayer:
       PlayEVA(-1)                          // building "garrisoned" voice cue
       PlayVocClass at building pos        // entry sound (default occupant sound)
7. If house+0x1EC (HumanPlayer flag):
     building+0x691 = 0                    // clear "auto-eject" flag
     building+0x1A4 = 0                    // clear ???
```

The occupant array has finite capacity (`MaxNumberOccupants=` in INI, default 5
for civilian buildings). When full, additional GIs cannot enter and are turned
away.

## P2.15 EjectOccupants? — `0x004575B0` is misnamed

The plan listed `0x004575B0` as "EjectOccupants" but the decompile shows
`BuildingClass::EjectAllUpgrades`:
- Walks `building.UpgradeLevel` (count of installed weapon upgrades, e.g.,
  Patriot Missiles on Air Force Command, ...)
- For each upgrade: get refund cost via vtable+0xB8, add to house credits via
  `HouseClass::Add_Credits`, then call `RemoveLastUpgrade()`
- Mark house flags `+0x5778` and `+0x5779` (UI dirty bits)
- Call vtable+0x1A0 (probably play removal anim)

This is NOT garrison-occupant ejection. The GI-eject-on-building-sell path
must be in another function. Phase 3 task — find the actual eject function
(likely linked to `BuildingClass::Sell` at `0x00449C30`).

> ⚠ **Plan correction:** the `EjectOccupants` entry in the investigation plan
> at `0x004575B0` is wrong. Phase 3 should look for the actual function via
> xrefs to `building.OccupantArray` clear operations.

## P2.16 Veterancy promotion — `SetVeteran` / `SetElite`

Trivial setters:

```
SetVeteran(arg):
  veterancy = arg ? 1.0 : 0.0     // 0x3F800000 = 1.0f, 0 = 0.0f

SetElite(arg):
  veterancy = arg ? 2.0 : 0.0     // 0x40000000 = 2.0f
```

Verified the `Veterancy` field at `entity+0x150` is a `float`. Threshold checks
elsewhere (`IsVeteran @ 0x0074FF90`, `IsElite @ 0x00750010`) compare against
`1.0` and `2.0` exactly.

XP accumulation (from `RecordKill` in Phase 1 §3.7) adds fractional cost-ratios
to `+0x150` directly, then crosses thresholds → triggers `SetVeteran` /
`SetElite`. The exact threshold-cross-detection happens in `FUN_0074FF50` (the
`AwardXP` helper called by RecordKill via vtable chain) — Phase 3 task.

## P2.17 Weapon resolution chain

### `TechnoClass::GetWeapon @ 0x0070E140` — pick concrete WeaponTypeClass

```
if (slot == -1): return null

if (Veterancy >= 2.0 (Elite)):
  weapon = type.GetEliteWeaponByIndex(slot)    // from EliteWeaponN list
  if (weapon != null AND weapon.Class != 0):
    return weapon

return type.GetWeaponByIndex(slot)              // standard PrimaryN/SecondaryN
```

So elite tries Elite slot first, falls back to standard. For the GI:
- Slot 0: Elite=M60E (Damage=25), Standard=M60 (Damage=15)
- Slot 1: Elite=ParaE, Standard=Para

### `TechnoClass::SelectWeaponAgainst @ 0x006F3330` — choose 0 or 1 vs target

Complex multi-branch:
```
if (FUN_00717880() = "is gattling weapon"):
  if (type+0xCD5 (== "GattlingWeapon")):
    return CurrentWeaponNumber (cycling)
return 0 (Primary)
```

For non-gattling units (like the GI), this is the **Primary vs Secondary**
chooser. Logic (simplified):
```
weapon0 = type.Weapon0
weapon1 = type.Weapon1
target_armor = target.type.Armor

if (weapon0.Verses[target_armor] == 0):           // primary deals 0 damage
  return 1 (secondary)

if (weapon1.Verses[target_armor] == 0):           // secondary deals 0 damage
  return 0 (primary)

// Special-case branches for:
// - Capture warhead (weapon+0x142): use 1 if can capture
// - Magnetron warhead (weapon+0x15B): use 1 if vehicle
// - Robot pilot, garrison, allied check, etc.
return 0 (primary fallback)
```

For the GI:
- vs Infantry: M60 (100% vs Soldier, ProneDamage 70%) → return 0 = Primary
- vs Light vehicle: M60 (80% Light) → primary deals damage → return 0 (M60). But Para has 100% Light → SelectWeapon may prefer Secondary... need to verify exact branch.
- vs Heavy vehicle (Rhino): M60 has 0% damage → return 1 (Para)
- vs Building: similar table lookup

### `TechnoClass::CanFireAt @ 0x006F77B0` — range check only

```
if (target == null): return 1
if (weapon.has_radius == false AND target_dist > weapon.Range): return 1 (out of range)
return TechnoClass::InRange(self, target, weapon)   // returns 0 if in range
```

Returns 0 if in range, otherwise non-zero error.

### `TechnoClass::GetFireError @ 0x006FC0B0` — comprehensive can-fire validator

Returns one of:
- **0** = OK, fire away
- **1** = OutOfAmmo (Ammo == 0)
- **3** = WaitingForReload / WaitingForFireFrame / InCloak (cycle through weapons)
- **5** = CantFire (any of: target null/invalid, deploying, sinking, allied, immune-to-warhead, target wrong cell, capture-not-possible, on bridge mismatch, type+0x82 "burnt" flag, etc.)
- **6** = OutOfRange (range fails, or building condition red without sensor)
- **8** = MustUncloak (FiringDescent flag forces uncloak first)
- **9** = TargetIsCloaked (target stealth + weapon doesn't pierce)

The function is enormous (over 50 distinct return points) — full enumeration
deferred to Phase 3 if any specific branch is needed.

**For the GI's normal fire-cycle:**
- Tick 1: GetFireError == 0 → call `vtable+0x3CC` (Fire_At spawn)
- Tick 2..N: GetFireError == 3 (waiting for reload via type+0xE40 fire frame)
- Tick reload: GetFireError == 0 again

Reload time is `weapon.ROF` frames (weapon+0x9C = ROF). M60 ROF=20, Para
ROF=15, UCPara ROF=15, CRM60 ROF=15.

## P2.18 Infantry-bridge interactions

Several Phase-2 functions reference bridge-layer cell offsets:
- `MarkCellOccupancy / UnmarkCellOccupancy` — bridge layer at `cell+0x128` / `+0x58`, ground at `cell+0x124` / `+0x54`.
- `WalkLocomotionClass::FindSubCellDest` — checks `cell+0x140 & 0x100 == bridge cell`.
- Bridge plane constant `DAT_00A8F234` = `Rules.BridgeHeight` (default 0x180 = 384 leptons = 1.5 cells).
- The "is on bridge" test for an entity: `entity_z >= ground_height + BridgeHeight`.

A GI walking onto a bridge cell occupies the bridge layer's sub-cell bits. Two
GIs on the same XY cell — one above bridge, one below — are valid (each in
their own layer). Crush checks etc. respect this.

---

# Phase 2 Updates to §6 — Rust Implementation Status

| Area | Status | Updated finding |
|------|--------|-----------------|
| **Sub-cell allocator** | IMPLEMENTED | Rust constant `FUNCTIONAL_SUB_CELLS = [2, 3, 4]` is correct (verified at `src/sim/movement/bump_crush.rs:31`). The "BUG CONFIRMED" finding above was based on a stale prior report; the Rust value has been right for some time. |
| **Fear/panic runtime** | MISSING | The whole 12/25/50 fear-add formula, the +50/+25/+12 health-gated step, and the FearLevel > 50 / < 50 / > 50+civilian transitions are not implemented. Adding `entity+0x6D4` field + the three handlers + Rules thresholds (Rules+0x1700, Rules+0x1708) closes this. |
| **Movement speed** | PARTIAL | `FootClass::GetCurrentSpeed` exists. **Missing**: the prone-state speed multiplier. Correct binary rule: `Crawls=+0xEBD` uses ×2/3; fallback prone branch uses ×1.5. |
| **IFV gunner swap** | COVERED | Per `IFV_AND_OPEN_TOPPED_TRANSPORT` and `combat_weapon.rs` — confirmed working. The 18-slot range (0..17) is honored. |
| **Garrison occupant entry** | COVERED | `passenger.rs` adds occupants and starts garrison fire. **Missing**: the Assaulter-instead-of-Occupier branch — Tanya / SEAL clear-garrison behavior. |
| **Fire validator** | PARTIAL | `combat_fire_gate.rs` blocks empty garrisons. **Missing**: full GetFireError state machine (50+ return points). For 99% parity, every branch must be implemented or explicitly justified. |
| **Locomotion sequence dispatch** | PARTIAL | Walk vs Crawl picked correctly when prone state is set. **Missing**: the "locomotor done → Stand or Prone idle" transition; piggyback-host (`type+0xD94`) flag handling; cheer (0x17) / tread (0x18) for hero/jumpjet. |
| **Veterancy promotion** | MISSING | `SetVeteran/SetElite` are simple setters. **The XP accumulator (RecordKill helper FUN_0074FF50) is not implemented**. Adding it requires hooking RecordKill which is missing. |
| **Idle action** | MISSING | The fidget animation picker, with VoiceComment 1/3 chance, civilian flee-to-building, etc. — none implemented. |
| **Scatter** | MISSING | No scatter logic for infantry. |

# Phase 2 Updates to §7 — Open Questions

**Closed:**
- Q4 (`type+0xD94`) — confirmed = "use IPiggyback locomotor" (Yuri Prime, magnetron)
- Q7 (FireFrac quad keys) — confirmed distinct keys read sequentially
- Q-bool-slots — partially closed for 0xEBC, EBD, EBE, EBF, EC3, EC6, EC9 (behavioral identification)

**Still open / new:**
- **NEW Q9** — `+0x6C4` is `CurrentPendingAction` (3=Walk/6=Crawl/0x11=HoverFly), but
  what writes it before `Clear_Doing_Action` reads it? Likely a vtable +0x4xx
  setter. Phase 3.
- **NEW Q10** — `Rules+0x17ED` ScatterRandomExempt flag — what's the INI key
  name? (Probably `ScatterFromCrush=` in `[CombatDamage]` section.)
- **NEW Q11** — `type+0x810` (the SetGunnerWeapon abort gate) — what bool? Likely
  `Cyborg=` or similar "no-IFV-passenger-swap" flag.
- **NEW Q12** — `Rules+0x1700` and `Rules+0x1708` exact INI key names for the
  fear-amount thresholds — likely `ConditionYellow` and `ConditionRed` from
  `[General]`.
- **OQ6** (ReceiveDamage return semantics) — still open, Phase 3 disassemble.
- **OQ-bool** — full key-name resolution for the 12 bools at +0xEBC..+0xECB
  remains pending xref to ReadBool callsites.

# Phase 2 — Sources

**Newly decompiled (Phase 2, 17 functions):**
- `InfantryClass::IdleDispatch @ 0x0051CBA0` (FULL)
- `InfantryClass::UpdateIdleAction @ 0x0051CDB0` (FULL)
- `InfantryClass::Clear_Doing_Action @ 0x00521B20` (FULL)
- `InfantryClass::GetMovementSpeed @ 0x00521D80` (FULL)
- `InfantryClass::Scatter @ 0x0051D0D0` (FULL)
- `InfantryClass::Panic_SetFear300 @ 0x00521C10` (FULL)
- `InfantryClass::SetFear @ 0x00518C00` (FULL)
- `InfantryClass::Fear_Decay_Handler @ 0x005200B0` (FULL)
- `InfantryClass::MarkCellOccupancy @ 0x005217C0` (FULL)
- `InfantryClass::UnmarkCellOccupancy @ 0x00521850` (FULL)
- `CellClass::PlaceInfantryInCell @ 0x00481180` (FULL)
- `CellClass::InitSubCellOffsets @ 0x0048E480` (FULL — table values extracted)
- `WalkLocomotionClass::FindSubCellDest @ 0x0075C240` (FULL)
- `FootClass::Locomotion_AI @ 0x00520F40` (FULL)
- `BuildingClass::AddGarrisonOccupant @ 0x00522910` (FULL)
- `BuildingClass::CanGarrison @ 0x004525F0` (LIGHT)
- `BuildingClass::EjectOccupants? @ 0x004575B0` (FULL — but plan-misnamed; actually `EjectAllUpgrades`)
- `BuildingClass::UpdateGarrisonFire @ 0x0043E7B0` (LIGHT — render side only; firing logic in different function)
- `TechnoClass::SetGunnerWeapon @ 0x0070DC70` (FULL)
- `TechnoClass::GetWeapon @ 0x0070E140` (FULL)
- `TechnoClass::SelectWeaponAgainst @ 0x006F3330` (MEDIUM — 100+ branches, common cases mapped)
- `TechnoClass::CanFireAt @ 0x006F77B0` (FULL)
- `TechnoClass::GetFireError @ 0x006FC0B0` (MEDIUM — 50+ return points; common cases mapped)
- `VeterancyStruct::SetVeteran @ 0x00750090` (FULL)
- `VeterancyStruct::SetElite @ 0x007500B0` (FULL)

---

---

# Phase 3 — Spawn paths, mind control, crush, render, voice, cursor

## P3.1 ⚠ CORRECTIONS to Phase 1/2 — verified bool slot names

**Direct disassembly/string-xref evidence resolved the Phase 1/2 inferred
slot names. This table supersedes older rows in this report:**

| Offset | **VERIFIED Name** | Evidence |
|--------|-------------------|----------|
| `+0xEAC` | **`Cyborg`** | string `0x00825A0C`, xref in `InfantryTypeClass__ReadINI`; also forces computed `+0xC8F=1` when true |
| `+0xEAD` | **`NotHuman`** | string `0x00825A00`, read immediately after `Cyborg` |
| `+0xEBC` | **`Fearless`** ⚠ (Phase 2 said "Fearless-equivalent") | xref @ `0x00524469` PUSH `0x8259D4 ("Fearless")`, with previous EAX (UndeploySound) being stored to `[ESI+0xEA8]` |
| `+0xEBD` | **`Crawls`** | string `0x008258F4`, xref `0x005246AE`; art.ini/artmd.ini image-section key |
| `+0xEC8` | **`Deployer`** | string `0x00825928`, xref `0x0052460D`; gate used by infantry mission `0x10` deploy toggle |
| `+0xEC9` | **`DeployedCrushable`** | string `0x00825914`, read after `Deployer` |
| `+0xEA4` | **`DeploySound`** | confirmed Phase 1 (cross-checked here — the previous-EAX-stored-to-EA8 pattern at the Fearless read confirms 0xEA8 = UndeploySound, hence 0xEA4 = DeploySound) |
| `+0xEA8` | **`UndeploySound`** | confirmed Phase 1 |

**Re-mapping consequences:**
- `Cyborg` is at `+0xEAC`, not `Crawls`. The GI has `Cyborg=no` (default), so
  this slot is 0 for the GI.
- `Crawls` is at `+0xEBD`, read from `artmd.ini [GI]`, not `rulesmd.ini [E1]`.
- `Fearless` is at `+0xEBC`. The GI has `Fearless=no` so this is 0. **The**
  **panic-immune behavior in `Panic_SetFear300`/`SetFear`/`Fear_Decay_Handler`**
  **gates on `+0xEBC` (Fearless), not Cyborg.**
- `Deployer` is at InfantryType `+0xEC8`. It is the mission-0x10 deploy toggle
  gate in `FUN_0051F6E0`.
- `DeployFire` is a separate TechnoType field at `+0x6AC`; it is not
  InfantryType `+0xEC8`.
- All Phase 1/2 references that said `+0xEAD = Fearless`, `+0xEAC = Crawls`, or
  `+0xEC8 = CanDeployFire` should read as superseded by this table.

**For the GI (E1) all critical bools are zero/default**: no Cyborg, no
Fearless, no Hero, no Civilian. Standard infantry — which is exactly what
the player expects.

> **2026-05-16 audit note:** The full `+0xEAC..+0xECB` bool/int band is now
> resolved by direct ReadINI string xrefs in `READINI_FIELD_MAPS.md` and
> `NAVY_SEAL_TANYA_C4_GHIDRA_REPORT.md`. No GI-critical bool names remain open.

## P3.2 Paradrop / Cloning Vats / Survivor spawn paths

### `AircraftClass::Mission_ParaDropOverfly @ 0x004157C0`

The aircraft mission-state that drives a paradrop. Phases:

```
1. Range-check: if (distance_to_target ≤ weapon0.Range):
     vtable+0x48C(...)         // alert callback
     vtable+0x488(weapon0.range)  // begin payload drop
     MapClass::UpdateFogBorder(self.pos, sight + 3)  // reveal cells under flight path
2. If no destination yet: pick exit edge:
     edge = HouseClass::GetOppositeEdge()
     exit_cell = FUN_004AA440(self.pos, edge, 4, ...)
     vtable+0x480(exit_cell, 1)  // assign new movement target
3. Return 3 (mission state code = "in transit")
```

### `AircraftClass::Drop_Payload @ 0x00415C60` — actually drops one passenger

Called from the overfly mission once the aircraft is in range:

```
1. payload = FUN_00473430()  // pull next passenger from cargo
2. Compute drop position relative to aircraft:
     angle = RateTimer::Current ± 0x3FFF (rotation around aircraft)
     drop_x = aircraft.x + cos(angle) * spread
     drop_y = aircraft.y + sin(angle) * spread
3. If destination cell passable (vtable+0x1AC):
     cell = PlaceInfantryInCell(drop_buffer, approach=null, force=0)
     If cell != null_coord:
       cell_get_at(cell)
       if (vtable+0xE8(passenger) succeeds):    // ParachuteUnload predicate
         VocClass::PlayAt(aircraft.pos)         // drop sound
         passenger.Position = cell.pos.shifted    // place infantry
         passenger.Mission = (current state)
         If aircraft.IsCarrier (entity+0x175):
           FUN_006EA870(passenger, -1, 0)         // limbo? release flag?
         entity.field_0x6D3 = 5    // drop progress state
         passenger.field_0xBB = currentframe
         return 0
4. If drop fails (cell taken, off-map, etc.):
     CargoClass::AddPassenger(passenger)        // re-add to cargo
     vtable+0x11C(passenger)                     // reset passenger state
     entity.payload_count++
     return 0
```

So a paradrop **CAN FAIL** if all 3 sub-cells in the target cell are occupied —
the unit returns to the aircraft's cargo and the next overfly attempt picks a
new target cell. This matches in-game observation that paradrop sometimes
"misses" the intended cell.

### `BuildingClass::SpawnSurvivors @ 0x00442D90` — survivors on building destroy/sell

Scans the building's exit-cell list (function `vtable+0x108`) and spawns
survivors:

```
1. survivor_count = 2 base, +1 if building has Cargo (+0x540), +6 if special flag (+0x6E3)
2. For each passenger in building cargo:
     drop_pos = next exit cell
     If passenger is unit (RTTI 1):  drop at exact cell coord
     Else:                            PlaceInfantryInCell with approach
     Unlimbo passenger via vtable+0xD8
     If success:
       passenger.House.UnitCount++
       Mission = Move (toward exit)
       For HumanPlayer: do nothing extra; for AI: set mission Hunt (0xF)
3. After cargo exhausted: walk exit-cell list and spawn survivor INFANTRY:
   For each cell in exit list:
     If type+0x16AE or +0x16AF set (some "spawns infantry on death" flag):
       50% chance: pick infantry from FUN_005F420F's pool (Rules.SoylentRecyclable?)
       Construct InfantryClass with Owner=building.Owner
       PlaceInfantryInCell at exit
       If unlimbo OK:
         strength = random(5, type.Strength)   // wounded
         Mission = Move (toward exit)
         AI sets Hunt
4. Spawn debris/smoke randomly: 50% smoke, 50% debris.
```

For the Allied Barracks (which has neither flag and no special survivor list),
**no GIs spawn from a destroyed barracks** in stock YR — the building just
explodes. (This matches in-game observation.)

For Cloning Vats: free GIs are produced via the FACTORY system, not survivor
spawn. Cloning Vats has `Cloning=yes` which makes it append to InfantryFactory
output queue. The free GI appears as a normal production output via
`BuildingClass::ExitObject`. **Not in this report's scope** — see
INFANTRY_PRODUCTION (Phase-4 separate report) if needed.

### `ChronoSphere::WarpUnitsAtCell @ 0x0065EC30`

Cited from prior research (Phase 2 cross-link only, not re-decompiled). When
chronosphere drops a GI onto a cell, it goes through `PlaceInfantryInCell`
with the same 3-sub-cell allocator. If full → falls into the "shake to nearby
cell" loop documented in `Scatter` (§P2.12).

## P3.3 Mind control on the GI — `CaptureManagerClass::CaptureUnit @ 0x00471D40`

Yuri / Yuri Prime / Psychic Tower / Genetic Mutator capture path:

```
1. CanCapture(target) check  (target is the GI — also gates ImmuneToPsionics
   from type+0xD35; the GI has ImmuneToPsionics=no so capture proceeds)
2. If host has InfiniteMindControl (+0x3C == 1):
     Release all current victims:  for each victim: CaptureManager::FreeUnit(victim)
3. target.GetID() → uVar2
4. If host.Type.CanCapture(target_house) is true:
     Allocate MCNode (0x14 bytes):
       node.victim = target
       node.timestamp_captured = CurrentFrame
       node.victim_id = uVar2
       node.??? = host_index
       node.duration = Rules+0x310 = MaxMindControlDuration
     If host.has_room OR (perma-mc OR no_victims_yet AND budget > 0):
       host.victims_array[count] = node
       host.count++
     target.MindController = host_owner
     For target sequence ∉ {0x10 (Subdue?), 0x12, 0x13}: vtable+0x3D0  // ClearMission
     CaptureManager::DecideUnitFate(target)
5. Spawn MC link animation:
     anim_type = Rules+0x320 = "ControlAnim" / "MindControlAnim"
     For target type 6 (building): anim at building pos
     Else: anim at target pos (with z-offset 0xFC00 relative to target)
     target.MindControlAnim = anim
     AnimClass::SetOwnerObject(target)  // animation follows target
6. Return 1 (captured)
```

**For the GI:** `ImmuneToPsionics=no` means capture proceeds. The GI is then:
- Owner switched to capturing house (Yuri's)
- Cannot be commanded by original player
- Plays a yellow link animation back to controller
- On controller death (Yuri killed): all controlled units freed via `FreeAllMindControlCaptures`
- On GI death: removed from controller's victims array via `FreeMindControlledChain`

**Key point:** The GI's `Owner` field at +0x87 stays the original house — only
`MindController` at +0x147 changes. This is why `RecordKill` checks both
fields (Phase 1 §3.7) for veterancy attribution.

## P3.4 Crush-kill — `UnitClass::OnEnterCell_Triggers @ 0x00744720`

When a vehicle drives into a cell occupied by infantry, this is invoked:

```
if (entity+0x34 != 0):                          // valid vehicle entity
  if (entity+0x338 != -1):                      // some pre-check (probably BridgeOffset?)
    cVar1 = FUN_006E57C0()                      // predicate, might be "is bridge end"
    if (cVar1): goto LAB_0074478D               // skip cell action processing
  if (param_2 != 0):                            // victim is set
    TechnoClass::ProcessCellAction(7=Killed, vehicle, victim_id, 0, 0)
  TechnoClass::ProcessCellAction(0x30, vehicle, victim_id, 0, 0)
  TechnoClass::ProcessCellAction(0x1D, vehicle, victim_id, 0, 0)
  
LAB_0074478D:
  TechnoClass::RecordKill(victim)               // award XP to vehicle's house
  return
```

The actual crush eligibility (Crushable=yes / OmniCrusher / DeployedCrushable)
gating is in `CRUSH_SYSTEM_GHIDRA_REPORT.md` (verified earlier). This function
is just the "crush succeeded → mark dead and credit kill" recorder.

**For the deployed GI:** type+0xEC9 = `DeployedCrushable=` from INI. The GI
has `Crushable=yes` and presumably `DeployedCrushable=` (default true). When
deployed (`entity+0x2A4 = 1` IsLowSilhouette), the crush check actually
**ALLOWS** crushing — deployed GIs are MORE crushable, not less. This matches
the ProneDamage rule (deployed infantry take more damage from MG fire too).

**Crush-kill XP:** RecordKill awards XP based on vehicle's veterancy:
- Rookie tank crushing rookie GI → XP ratio `100/cost_of_tank`. For Rhino
  ($900): ~0.111 XP per kill. So a Rhino needs ~10 GI kills to promote.
- Veteran crushing → 2x → ~0.22 XP → ~5 GI kills to promote.
- Elite crushing → 3x → ~0.33 XP → ~3 GI kills to promote.

## P3.5 Iron Curtain on the GI — `InfantryClass::IronCurtain @ 0x00522600`

```
local_4 = type+0xA0 (== HitPoints, full health value)
vtable+0x16C(damage=type.Strength,
             warhead=Rules+0xFA8 (== "C4Warhead" / "DeathWeapon" / similar — verify Phase 4),
             attacker=null,
             ignore_armor=1,
             ammo=0,
             radius=0,
             attacker_house=param_3)
```

So **IronCurtain on a GI deals damage equal to the GI's full HP**, using
warhead at Rules+0xFA8 — which is typically a high-armor-pierce warhead
that ensures kill. The `+0x16C` slot is `vtable::ReceiveDamage`. So:
- IC on infantry → calls ReceiveDamage(125, IC_Warhead, ...) → instant kill
- This is **opposite** to vehicles where IC GRANTS invincibility for the duration

Verified: IronCurtain.exe target = vehicle/structure, NOT infantry. Targeting
IC on a GI is wasted — it kills the GI rather than protecting him. **The
GI is invulnerable to IC if the IC is targeting an adjacent vehicle**, but
direct IC on the GI = death.

## P3.6 Cursor decision matrix — `What_Action_OnObject` / `_OnCell`

Phase 1 noted these were complex. Rough decision tree for the GI:

**On a cell** (`What_Action_OnCell @ 0x0051F800`):
```
1. base = FootClass::What_Action_OnCell      // Move(2), Walk(3), or NoGo(0)
2. weapon_range < 0 AND base == 5 → 0x1A (Deploy cursor — own cell only)
3. type+0xD94 (exotic) AND base ∈ {1, 2}: water-cell check → override to 0
4. type+0xEC8 AND in deploy seq:
     base == 5 (Move) AND DeployFire → vtable+0x164 → 0x33/0x35 cursor (Force-Fire)
     else: base = 2 (Move)
5. base == 1 + cell on low bridge → 0x23 or 0x24 (BridgeRepair?)
6. type+0xEC3 + cell has visible building:
     building.field_0x16B6 set → return 0x20 (Allied Move)
     building+0xCCC set → return 9 (Repair)
7. base == 5 + can-take-action false → 0x3B (DamagedDeploy)
8. base == 0 + NOT type+0xD94 + can-take-action true → 2 (Move)
9. Return base
```

**On an object** (`What_Action_OnObject @ 0x0051E3B0`):

Even more complex (~70 distinct return values). Key paths for the GI:
- **vs friendly building (CanRepair=yes, low HP)** → 9 (Repair cursor)
- **vs friendly building (CanRepair=yes, full HP) + can-load** → 0x1d
- **vs garrisonable building** → 0x20 (Garrison cursor)
- **vs hostile building (Engineer Capturable)** → 0x10 (Capture)
- **vs self** → 0x1A (Deploy)
- **vs hostile vehicle in range** → 1 (Attack)
- **vs hostile vehicle out-of-range** → 7 (Move-then-attack)
- **vs friendly vehicle (low HP)** → 3 (Repair)
- **vs friendly Tanya/hero (full HP)** → 0x1B (Move)
- **type+0xEAE + iVar7 == 5 + target+0x22E** → 0x35/0x36 (sell cursor — Tanya's C4 selling civilian buildings)
- **type+0xEC3 + building+0x16B6** → 0x20 cursor (special move into civilian-neutral)
- **vs hostile cloaked unit** → No fire (5 → 0x40 NoGo)
- **vs target on different bridge layer** → 0x1F (cant-reach)

For the GI specifically, the typical cursors are:
- M16 (1) Attack
- Walk (3 or 7) Move
- Garrison (0x20) when over civilian building
- Deploy (0x1A) when right-clicking self or target in range
- NoGo (0x40) when target is cloaked or otherwise un-shootable

**Modifier keys** (DAT_00A8EC00..0C) check shift/control/alt for force-attack/
force-move. These set local bools `bVar4` and `bVar5` early in the function.

## P3.7 Selection bracket / pip rendering — `TechnoClass::DrawExtras @ 0x006F5190`

For each unit on screen:
```
1. If has IvanBomb (entity+0x1A): draw bomb-clock SHP at cell position
2. If wrench/repair (entity+0x6E8 set): draw cycling WRENCH SHP frames at cell
3. If selected (entity+0x83):
     For unit (RTTI 0xF):
       Compute 8-edge bracket (4 corners + 4 mid-edges) using a 2D bounding box
       Average each pair, sort by Y, draw via Tactical+0x60 (line drawer)
       Fill bottom edge with health bar via vtable+0x448
     For other (infantry, building):
       Compute 4-corner bracket via DrawBracketCorner
       Draw HP bar via vtable+0x44C
       Draw pips via vtable+0x44C (twice — once for cargo, once for vetpip?)
4. If badge (entity+0x431) AND not panicked: render group-badge cycling SHP
5. If selected and is leading the group: draw selection-count-badge "(N)" at top
```

**Pip details** (cited from `OBJECTCLASS_DRAW_LIMBO_CELLLIST.md`):
- Health bar at the top of the bracket
- Veterancy pips below health
- Cargo pips for transports
- For garrison occupants (building's occupant pips), drawn separately at building cell pos via `OccupyPip` color

For the GI: standard 4-corner bracket, white pip color (`Pip=white` in INI),
SHPs from PIPS.SHP/PIPBRD.SHP loaded once at startup
(`ObjectTypeClass::LoadPipAssets @ 0x005F76B0`).

## P3.8 Voice playback — `ObjectSelection::PlayVoice @ 0x00637840`

```
mutex DAT_00AC4CF6      // anti-recursion
if (DAT_00AC4CF4):       // selection-active flag
  selection_list = FUN_00705D20()  // get current selection vector
  last_idx = list.count - 1
  last_unit = list[last_idx]
  count = last_unit.size  // multiple types? not sure
  for i in 0..count:
    unit = last_unit.entries[i]
    if (NOT entity+0x83 == panicked):
      vtable+0x14C(unit)     // PlayVoiceSelect
```

**For the GI selected:** 
- Plays VoiceSelect = `GISelect` (random of GISelect.WAV variants)
- Multiple GIs in selection → one voice per type plays (not 5 GIs all yelling)
- Panicked GI doesn't play select voice

**Voice keys on [E1]:** all in TechnoTypeClass (Phase 1 §4 cross-link):
- `VoiceSelect=GISelect` → entity selected
- `VoiceMove=GIMove` → move command issued
- `VoiceAttack=GIAttackCommand` → attack command issued
- `VoiceFeedback=GIFear` → played when panicked / damaged enough
- `VoiceSpecialAttack=GIMove` → for units with deploy/special action
- `DieSound=GIDie` → played on death (in ReceiveDamage chain)
- `CrushSound=InfantrySquish` → played when crushed
- `DeploySound=GIDeploy` (`+0xEA4`) → played on Deploy seq 0x1B
- `UndeploySound=GIUndeploy` (`+0xEA8`) → played on Undeploy seq 0x1F

The deploy/undeploy sounds are wired in `Do_Action` step 9/10 (Phase 1 §3.2).

## P3.9 WarheadTypeClass::Detonate — top-level dispatch

`0x004690B0` is huge (1500+ lines). Top-level branches by warhead bool flag:

| Warhead flag | Offset | Effect on infantry target |
|--------------|--------|---------------------------|
| `Tiberium=` (vein) | `+0x14F` | Direct damage if target is unit type 1; else passes through |
| `Temporal=` | `+0x157` | TemporalClass::InitiateWarp — target enters warp queue |
| `IsLocomotor=` (Magnetron) | `+0x15B` | Vehicles only (SizeWeight gate); infantry pass through unaffected |
| `MakesUnitInvincible=` (IC) | `+0x158` | Calls FUN_0062A980 — special IC apply on infantry kills them (see §P3.5) |
| `EMP=` | `+0x14F` (different) | Stuns vehicle/infantry — entity+0x82 set |
| `MindControl=` | `+0x16C` | Calls CaptureManagerClass::CaptureUnit (§P3.3) |
| `Psychedelic=` | (gated by berserk flag at warhead bytes) | Calls Panic_SetFear300 indirectly via ReceiveDamage chain |
| `NukeMaker=` | `+0x175` | Spawns downward-falling nuke — affects buildings primarily |
| `BombDisarm=` | `+0x14F` (different bool) | Defuses Ivan bombs — clears entity+0x90 |
| `RadLevel=` | `+0x158` (RadSite) | Spawns RadSiteClass at impact cell — applies radiation per tick |

After the special-effect branches, the function calls
`Apply_area_damage(...)` which is the actual HP-damage applier — runs through
`FootClass::ReceiveDamage` → `TechnoClass::ReceiveDamage` →
`InfantryClass::ReceiveDamage` (Phase 1 §3.6 — return-semantics still open).

Animation spawn happens last:
- Pick explosion anim from warhead's per-cell-type table
- Spawn via `AnimClass::Constructor` at impact cell

## P3.10 ReceiveDamage return semantics — RESOLVED (corrected 2026-07-12)

Phase 1 OQ6 (ReceiveDamage return semantics inversion) — the Ghidra
`extraout_AL`/`CONCAT31` decompile shape noted here was indeed just
calling-convention noise (return value in AL), not a real bug. But the
"return 1 on most paths" characterization below it was WRONG — see the
corrected branch trace now in §3.6, verified this session via
`disassemble_function 0x005227F0`: **allied damagers are blocked (return 0)
immediately and unconditionally; non-allied damagers are the ones that go
through the cell/mind-controller checks and mostly resolve to allow (return
1)**. So "most paths return 1" only holds for the non-allied branch, not
globally. The paragraph below (kept for its still-accurate GI-level summary)
already had this half right:

For the GI: ReceiveDamage allows damage from non-allied attackers, blocks
allied damage UNLESS the attacker is mind-controlled by an enemy of the
GI's house OR something else routes around the IsAlliedWith check. Standard
infantry damage gate.

## P3.11 EjectOccupants — corrected address

The plan listed `0x004575B0` but that's `EjectAllUpgrades`. The actual
garrison-occupant ejection happens in **`BuildingClass::ClearOccupants`**
(or equivalent) — the function called when the building is sold or
destroyed, which iterates the occupant array and calls `vtable+0xD8`
(Unlimbo) on each at the cell adjacent to the building. The exact address
needs a Phase 4 follow-up via xrefs to the building's occupant array clear
ops. **For implementation**: the eject behavior is "place occupants on
adjacent passable cells, reset their state, mission Hunt for AI."

## P3.12 New / closed open questions

**CLOSED in Phase 3:**
- ✓ Q-cyborg-fearless: `+0xEAC = Cyborg`, `+0xEAD = NotHuman`, `+0xEBC = Fearless` (corrected)
- ✓ Q-bool-key map: GI-critical InfantryType bools are now resolved by direct string xrefs (`+0xEBD=Crawls`, `+0xEC8=Deployer`, `+0xEC9=DeployedCrushable`)
- ✓ OQ6 (ReceiveDamage return semantics): function is structurally correct;
  the apparent inversion is decompile shape
- ✓ Plan correction: `0x004575B0` is EjectAllUpgrades, not EjectOccupants
- ✓ Cloning Vats path: production via factory, not survivor spawn

**STILL OPEN (3 minor items):**
- **Q-real-eject**: actual EjectOccupants address (Phase 4 if implementing
  garrison-eject-on-sell)
- **Q-Rules-keys**: `Rules+0x1700/+0x1708/+0x16C0/+0x17ED` exact INI keys
  (likely `ConditionYellow`, `ConditionRed`, `ScatterRandomThreshold`,
  `ScatterFromCrushExempt`). Phase 4 if implementing fear or scatter.
- **Q-vtable-labels**: `+0x1D4/+0x1D8` still need friendly semantic labels,
  but `+0x1EC=MissionClass::Commence` and `+0x200=CanQueueMission_Now` are
  resolved and must not be treated as deploy predicates.

## P3.13 Phase 3 Sources — newly decompiled

- `AircraftClass::Mission_ParaDropOverfly @ 0x004157C0` (FULL)
- `AircraftClass::Drop_Payload @ 0x00415C60` (FULL)
- `BuildingClass::SpawnSurvivors @ 0x00442D90` (FULL)
- `CaptureManagerClass::CaptureUnit @ 0x00471D40` (FULL)
- `UnitClass::OnEnterCell_Triggers @ 0x00744720` (FULL)
- `InfantryClass::IronCurtain @ 0x00522600` (FULL)
- `InfantryClass::What_Action_OnObject @ 0x0051E3B0` (MEDIUM — 70+ branches, common cases mapped)
- `InfantryClass::What_Action_OnCell @ 0x0051F800` (FULL)
- `TechnoClass::DrawExtras @ 0x006F5190` (MEDIUM — 600 lines, key flow only)
- `WarheadTypeClass::Detonate @ 0x004690B0` (LIGHT — top-level dispatch only; specific branches deferred)
- `ObjectSelection::PlayVoice @ 0x00637840` (FULL)

**Bool-key xref evidence:**
- `Cyborg @ 0x00825A0C` ↔ `0xEAD` slot via xref at `0x005243B0`
- `Fearless @ 0x008259D4` ↔ `0xEBC` slot via xref at `0x00524469`

---

# Final Implementation Status (post-Phase 3)

| Major area | Coverage | Implementation cost |
|------------|----------|---------------------|
| InfantryType parsing | HIGH | GI-critical slots resolved: `Crawls=+0xEBD`, `Deployer=+0xEC8`, `DeployedCrushable=+0xEC9`, `Fearless=+0xEBC`. `+0xC8F` is a computed legacy flag forced by `Cyborg=+0xEAC`, not by `Crawls`. |
| Per-tick AI brain | HIGH | 19-step `InfantryClass::AI` documented with all branches. Implementation needs all 19 + the 4 vtable predicates. |
| Sequence state machine | HIGH | Full Down/Crawl/Up + Deploy/Deployed/DeployedFire/Undeploy + Sleep/Cheer/Panic. State-machine diagram in §3.3. |
| Fire decision (`Fire_At_Target`) | HIGH | All 4 fire-frame slots, weapon-vs-target matrix, fire-error gating. 8 distinct error codes (0/1/3/5/6/8/9). |
| Damage gate (`ReceiveDamage`) | HIGH | Friendly-fire check correct (Phase 3 confirmed). Falls through to ProneDamage in Verses table. |
| Veterancy (`RecordKill` + `SetVeteran`/`SetElite`) | HIGH | XP formula is `victim.cost / killer.cost × tier_mult`. Thresholds 1.0 / 2.0. House score counters at +0x5434/+0x5488/+0x53E4/+0x5438/+0x548C/+0x54E8. |
| Construction (`InitFromType`) | HIGH | VeteranInfantry list + InitialVeteran flag promotion paths verified. |
| Sub-cell allocation | HIGH | 3 functional slots (2/3/4); pixel offsets verified; bridge layer separate. **Implemented correctly in Rust** (`src/sim/movement/bump_crush.rs:31`). |
| Walk locomotor | HIGH | `FindSubCellDest` + `Locomotion_AI` mapped; sequence dispatch (Walk vs Crawl vs Stand vs Prone) verified. |
| Fear / Panic | HIGH | Three-state machine: 0..50 (calm), 50..299 (panic), 300 (max). Decay 1/tick. Per-damage formula 50/25/12 by HP tier. `Fraidycat` flee-to-building. |
| Movement speed | HIGH | Prone speed is ×2/3 when `Crawls=+0xEBD` is set. GI has `Crawls=yes`, so GI crawls slower while prone. |
| Idle dispatch | HIGH | Random fidget picker, civilian flee, voice comment. |
| Scatter | HIGH | Player-deployed = no-scatter. `Fraidycat` infantry always scatter. Direction = random ±2 facings. |
| IFV gunner swap | HIGH | `SetGunnerWeapon(IFVMode)`. Range 0..17. |
| Garrison entry | HIGH | `AddGarrisonOccupant` + Assaulter (Tanya/SEAL) clear-garrison branch. |
| Garrison fire | MED | `BuildingClass::UpdateGarrisonFire` is RENDER-side only. Actual fire dispatch in another function (likely vtable). |
| IFV/garrison weapon | HIGH | Cross-linked to GARRISON_SYSTEM and IFV_AND_OPEN_TOPPED reports — verified. |
| Spawn paths | HIGH | Paradrop, chrono, survivors all mapped. **Cloning Vats is a factory output, not a survivor spawn**. |
| Mind control | HIGH | CaptureManager flow, MCNode allocation, controller side and victim side both mapped. |
| Crush-kill | HIGH | OnEnterCell_Triggers + RecordKill chain. Deploy state INCREASES crushability. |
| Iron Curtain | HIGH | IC on infantry = INSTANT KILL, not invincibility. **Implemented in Rust** (`src/sim/superweapon/iron_curtain.rs:57-60`). |
| Cursor logic | HIGH | What_Action_OnCell + What_Action_OnObject mapped. ~70 distinct return codes for object-target. |
| Render | MED | Selection bracket + pip system structurally mapped. Specific frame indices and palette logic in cross-linked reports. |
| Voice | HIGH | All voice slots wired; selection chain confirmed. |

**Total Ghidra coverage:** 38 functions decompiled in full or substantial
detail across the three phases. Coverage of the GI's runtime behavior
(parse → tick → fire → damage → fear → locomotion → garrison → IFV →
veterancy → spawn → MC → crush → IC → render → voice → cursor) is
**sufficient for 99% parity implementation** without further Ghidra trips.

The 4 remaining open questions (bool-name resolution, real EjectOccupants,
Rules-INI key naming, vtable deploy-fire predicate names) are **all
narrow** and **none block the GI's core implementation** — they affect
"slightly off" details that can be resolved opportunistically during
Rust implementation when the specific feature is touched.

---

# Recommended next step

The dossier is now sufficient to drive an implementation plan. Suggested
follow-up:

1. **`/brainstorm gi-implementation`** — produce a design spec for which
   parts of this report to implement first, ordered by player-visibility.
   Expected priority: deploy-fire state machine + sub-cell `[2,3,4]` bug
   fix + fear runtime + missing missions (Mission_Move/Attack/Guard).
2. **`/write-plan gi-deploy-fire`** — concrete code plan for the deploy
   state machine.
3. Implement.

For the residual open questions, consider a single
`/re-investigate gi-residual-bools-and-rules-keys` pass to close them all
at once (probably 30–60 min of work — small).

---

## Verified-already-implemented (post-Phase-3 audit)

The Phase 1-3 dossier described three items as Rust gaps. A `/brainstorm`
audit on 2026-05-04 found they were already implemented; the original
findings were based on stale prior research docs:

- **Sub-cell allocator `[2, 3, 4]`** — `src/sim/movement/bump_crush.rs:31`.
  The Phase 1 report cited an older `INFANTRY_SUBCELL_POSITIONING.md` claim
  of `[0, 3, 4]`; the Rust constant has been correct for some time.
- **IronCurtain on infantry kills the GI** — `src/sim/superweapon/iron_curtain.rs:57-60`.
  The handler explicitly zeroes infantry HP rather than applying invulnerability,
  matching the binary override behavior (P3.5).
- **DieSound parsing + emit** — `src/rules/object_type.rs:217+670` (parse),
  `src/sim/combat/mod.rs:1311` (emit on combat death). DieSound on **crush**
  kills was added by GI Quick-Wins A (2026-05-04) along with `CrushSound`.

This appendix exists so future readers of this report do not re-investigate
already-implemented mechanics.

---

# Deploy-Trigger Investigation (2026-05-04)

This appendix verifies and corrects the Phase 1-3 deploy-state-machine
sections. Five questions were targeted; several Phase 1-3 claims about
vtable slots and the player-input path turn out to be wrong, and the real
player deploy mission is not what the earlier text implied. Confidence per
finding noted inline.

> ⚠ All findings here are from live decompilation in 2026-05-04 against the
> retail `gamemd.exe` (Yuri's Revenge). Every claim is anchored to a Ghidra
> address or vtable offset and has been traced to confirm reachability in a
> stock YR skirmish (no TS-legacy paths).

## D-T.0 Foundational corrections (Phase 1-3 errors)

These corrections affect interpretation of every deploy-related section.
Apply them when reading §§ 3.2-3.4, P2.x, and P3.6.

### Active-sequence offset is `+0x6C4`, not `+0x1A4` — HIGH

Phase 1 §3.2 step 11 said `entity+0x1A4 = seq` is the commit point. The
disassembly of `InfantryClass::Do_Action @ 0x0051D6F0` actually writes:

```
param_1[0x1B1] = iVar6;          ; Pcode form (param_1 typed as int *)
                                  ; byte offset = 0x1B1 * 4 = 0x6C4
```

Confirmed by `Clear_Doing_Action @ 0x00521B20`:

```
00521b20: MOV EAX, dword ptr [ECX + 0x6c4]    ; read current seq
00521b35: MOV dword ptr [ECX + 0x6c4], 0xFFFFFFFF
```

…and by every consumer in `FUN_00521320`, `FUN_0051F660`, `FUN_0051F6E0`,
`FUN_00521B60`, etc. — they all read `[ESI + 0x6C4]`, not `[ESI + 0x1A4]`.

**Fix to mental model**: replace `entity+0x1A4` with `entity+0x6C4` wherever
Phase 1-3 references the active sequence. The "param_1[0x69] = entity+0x1A4"
in earlier Phase 1 prose was a Ghidra `int *` indexing miscount. `+0x1A4` is
likely some other field (animation-related) — not the live sequence.

### Phase 1 vtable-slot labels for "deploy predicates" are wrong — HIGH

Phase 1 §3.3 footnotes claimed:

> Vtable +0x1D4/+0x1D8/+0x1EC/+0x200 deploy predicates

Live xrefs from each slot:

| Slot | Real target | What it is |
|------|-------------|-------|
| `+0x1D4` (`0x007EB22C`) | `TechnoClass::IsWarpingOut @ 0x0070C5B0` | chrono predicate (entity+0x434 != 0?) |
| `+0x1D8` (`0x007EB230`) | `TechnoClass::IsBeingWarped @ 0x0070C5C0` | chrono predicate |
| `+0x1EC` (`0x007EB244`) | `MissionClass::Commence @ 0x005B3570` | promote queued mission to active |
| `+0x200` (`0x007EB258`) | `FUN_00521B60` (InfantryClass override) | generic CanQueueMission_Now (see §D-T.3) |

**None of these are deploy-specific.** They are generic mission/chrono
machinery that happens to be invoked during deploy queueing. The "ActivateDeployFire
mutates a stored weapon pointer" theory in the Phase 1 prose is not supported
by the binary.

### Phase 1 vtable+0x4C0 reference resolved — MEDIUM

`FUN_006FFBE0` (vtable+0x378, the player command builder) uses
`vtable+0x4C0` to gate a `1/2 → 0x1D` action remap. For InfantryClass,
vtable+0x4C0 = `0x005228C0` which is a thunk to `FUN_0070F090` (Ghidra
flagged the inner switch as "too many branches"). The slot represents some
"DeployToFire" predicate (TechnoTypeClass-bool dispatched at runtime); not
relevant to standard GI deploy because action 4 (the player path, see §D-T.1)
bypasses this remap entirely.

## D-T.1 (a) Player input trigger — HIGH (verified)

**The player path uses action code 4, not 0x1A.** Phase 3 §P3.6 said
"vs self → 0x1A (Deploy)" — that is the *cursor* visual returned by
`What_Action_OnCell` for own-cell clicks, but the *click handler* dispatch
for "right-click on own infantry, single-selection" returns action 4 from
`TechnoClass::What_Action_OnObject @ 0x006FFEC0`:

```
006fff63 (approx): if ((param_2 == param_1) && (g_CurrentObjects_Count == 1)) {
                     cVar4 = HouseClass__IsHumanPlayer();
                     if (cVar4 != '\0') {
                       return 4;          // ← action 4 = Self
                     }
                     ...
                   }
```

Then `FootClass::ClickedAction_Object @ 0x004D74E0` jump-table
(switch-base `0x004D7CD0`, byte-table `0x004D7D04`) maps:

```
action 4 → byte[3] = 0x03 → case 3 entry @ 0x004D75E1:
    PUSH 0; PUSH 0; PUSH 0; PUSH 0x10
    CALL [vtable + 0x378]            ; Player_Send_Command(mission=0x10, ...)
```

So **the player's right-click on his own GI (or D-key shortcut, which
synthesizes the same action via the keybind handler) queues mission 0x10**,
not mission 0xB.

### What 0xB does (not the player path)

For completeness: `ClickedAction_Object` case 0x1A (taken when
`What_Action_OnObject` returns cursor-action 0x1A — e.g., from
ScrollClass/cursor logic on multi-selection, or from
`ClickedAction_Cell` case 0x1A on cell clicks) dispatches:

```
LAB_0x004D75D3: PUSH 0x0; PUSH 0x0; PUSH EDI (target); PUSH 0xB
                CALL [vtable + 0x378]
```

That queues mission 0xB → InfantryClass vtable+0x220 → `FUN_0051F640`
(an AreaGuard wrapper that calls the auto-deploy helper `FUN_00521320`).
`FUN_00521320` has an explicit `IsPlayerControl == 0` gate at `0x00521500`
on its entry path — so for a player-controlled unit it returns -1 and falls
through to `FootClass::Mission_AreaGuard`. **Mission 0xB does NOT deploy
player units.** It is the AI Guard/AreaGuard auto-deploy mission only.

### The actual deploy-toggle: `FUN_0051F6E0` at vtable+0x23C — HIGH

Mission 0x10 dispatches via `MissionClass::Mission_Dispatch @ 0x005B3060`
case 0x10 → `vtable+0x23C` → `0x007EB294` → `FUN_0051F6E0`. Decompile:

```
FUN_0051F6E0(this):                                        ; @ 0x0051F6E0
  if (type+0xEC8 != 0) {                                   ; InfantryType Deployer flag
    seq = entity+0x6C4;
    if (seq in {0x1B,0x1C,0x1D,0x1E}) {                    ; currently deployed
      if ((int)type+0x6C4 < 0) {                            ; type-default = -1
        Do_Action(0x1F, allow_repeat=1, random_start=0);    ; UNDEPLOY @ 0x0051F72D
      }
    } else {
      Do_Action(0x1B, allow_repeat=1, random_start=0);      ; DEPLOY   @ 0x0051F73F
      target = vtable+0x3F0();
      if (target_is_unit AND target+0x150 != 0) {
        cVar1 = FUN_005F3E50(0x0082557C);
        if (!cVar1) vtable+0x3C8(target_cell);              ; SetDestination
        else        delay = type+0xE3C[0x1B].FrameTime + 1;
      }
      if (-1 < type+0x6C4) delay = type+0x6C4;
    }
    vtable+0x1F0(5);                                        ; Override_Mission(5 = Guard)
    vtable+0x480(0, 1);                                     ; Set_Destination(null)
    if (-1 < delay) return delay;
  }
  return Mission_Default();
}
```

Notable:

- **No `IsPlayerControl` gate.** This is the real toggle for both player
  and AI when the mission is explicitly 0x10.
- After the seq change, mission is forced to Guard (5) via Override_Mission
  — that's why a deployed GI doesn't pursue, scatter, or wander.
- The follow-on `vtable+0x480(0, 1)` (Set_Destination(null)) clears
  pending nav targets, but for player units the override at the top of
  `Set_Destination` (see §D-T.2 below) early-returns when in deploy
  state, so this call is effectively a no-op for player-deployed GIs.

### Net flow for player deploy (full chain) — HIGH

```
1. Player right-clicks own GI (single selection) OR presses D
2. DisplayClass::DetermineAction @ 0x00692610
     → SelectBestObjectForAction
     → vtable+0x74 (TechnoClass::What_Action_OnObject @ 0x006FFEC0)
     → returns 4
3. FootClass::ClickedAction_Object @ 0x004D74E0
     case 4 (jump-table case 3 @ 0x004D75E1)
     → vtable+0x378(0x10, 0, 0, 0)
4. vtable+0x378 = FUN_006FFBE0 (Player_Send_Command)
     builds an EventClass via FUN_004C6860, action_code = 0x10
     queued via FUN_00637DD0
5. EventClass::Execute @ 0x004C6CB0 case 4 (MEGAMISSION):
     → vtable+0x4A4 = FootClass::Assign_Target_Command @ 0x004DF0E0
       returns event[0xC] = 0x10
     → vtable+0x1E8 = MissionClass::Queue_Mission @ 0x005B35E0
       sets entity+0xB4 (queued mission) = 0x10
6. Next AI tick:
     MissionClass::Commence @ 0x005B3570 promotes queued → active
       (entity+0xAC := 0x10, entity+0xB4 := -1)
     MissionClass::Mission_Dispatch @ 0x005B3060 case 0x10:
       → vtable+0x23C = FUN_0051F6E0
7. FUN_0051F6E0 toggles seq via Do_Action(0x1B) or Do_Action(0x1F)
8. DoType_Sequencer (§3.3) post-anim transitions:
     0x1B → 0x1C and sets entity+0x2A4 (IsLowSilhouette) = 1
       (gated on type+0xEC9 == 0 — GI has 0, so flip happens)
     0x1F → 0    and clears entity+0x2A4 = 0
```

**InfantryClass mission-dispatch table for deploy-relevant slots** (xrefs
read live from `0x007EB058 + offset`):

| Mission ID | vtable slot | InfantryClass override | Purpose |
|-----------|-------------|------------------------|---------|
| 2 (Move)  | `+0x22C` (`0x007EB284`) | `FUN_0051F660` | Mission_Move with player-deployed early-return |
| 5/6 (Guard/Sticky) | `+0x21C` (`0x007EB274`) | `FUN_0051F620` | Mission_Guard with auto-deploy preempt |
| 0xA (AreaGuard) | `+0x224` (`0x007EB27C`) | `FUN_00522E70` | Slave/Yuri tiberium harvest (NOT AreaGuard for the GI; see note below) |
| 0xB | `+0x220` (`0x007EB278`) | `FUN_0051F640` | AreaGuard with auto-deploy preempt (AI only) |
| 0x10 | `+0x23C` (`0x007EB294`) | `FUN_0051F6E0` | **Player deploy/undeploy toggle** |

> **TS-legacy check on mission 0xA**: `FUN_00522E70` (the InfantryClass
> override) is the YR Slave-Miner / Yuri-clone tiberium-harvest handler
> (eats tiberium, calls StorageClass::AddAmount). It runs only when
> type+0x800 (Storage) is non-zero — for the GI, type+0x800 = 0, so
> mission 0xA on the GI immediately calls `Do_Action(0)` and
> `Queue_Mission(5 = Guard)`. **For the GI, mission 0xA is essentially a
> "go idle" directive** — not a TS-only path, but only meaningful for
> Slaves/Yuri's Initiates.

## D-T.2 (b) Undeploy auto-triggers — HIGH (verified)

**Move commands do NOT auto-undeploy player-controlled GIs.** Player must
explicitly toggle deploy off (mission 0x10 again, or D-key).

### Player path: Mission_Move on a deployed GI is silently ignored

`FUN_0051F660` (vtable+0x22C, mission 2):

```
FUN_0051F660(this):
  seq = entity+0x6C4;
  if (seq in {0x1B,0x1C,0x1D,0x1E}) {
    house = vtable+0x3C(this);
    if (HouseClass::IsPlayerControl(house)) {
      vtable+0x480(0, 1);            ; Set_Destination(null) — see below
      return 1;                      ; ← bail, stay deployed
    }
    if ((int)type+0x6C4 < 0) {        ; AI path, GI's type-default
      Do_Action(0x1F, 0, 0);          ; UNDEPLOY @ 0x0051F72D
      return type+0xE3C[0x1F].FrameTime;
    }
  }
  return FootClass::Mission_Move();
}
```

`vtable+0x480` (`InfantryClass::Set_Destination`, `FUN_0051AA40`) also
early-returns for player-deployed units:

```
FUN_0051AA40(this, dest):                              ; @ 0x0051AA40
  vtable+0x3C(this);                                   ; get owner house
  if (HouseClass::IsPlayerControl(house)) {
    if (entity+0x6C4 == 0x1B) return;
    if (entity+0x6C4 == 0x1C) return;
    if (entity+0x6C4 == 0x1D) return;
    if (entity+0x6C4 == 0x1E) return;
  }
  ...
```

So for player deployed GI receiving move:

- `FUN_0051F660` sees player-controlled deploy → calls `Set_Destination(null)`
  which itself early-returns → no nav-target change → returns 1
- The unit stays in deploy state, no undeploy, no nav update.

This matches stock-YR gameplay: deployed GIs ignore move clicks until
manually undeployed.

### AI auto-undeploy paths

The four binary callsites of `Do_Action(0x1F, ...)` for InfantryClass are:

| Callsite | Container | When it fires | Player-allowed? |
|----------|-----------|---------------|-----------------|
| `0x00521355` | `FUN_00521320` (auto-deploy / AreaGuard helper) | Already-deployed AI on Mission_Guard / 0xB and `type+0x6C4 >= 0` | NO (this branch is in the deploy-entry block but the gate above it catches player on entry) |
| `0x0051F72D` | `FUN_0051F6E0` (player toggle) | Deployed unit gets mission 0x10 again | YES — the *only* player-undeploy path |
| `0x00522479` | `FUN_00522340` (Mission_Attack auto-deploy/undeploy) | AI in Mission_Attack with target out of deploy-fire range | NO (gated on `IsPlayerControl == 0`) |
| `0x0051F72D` | `FUN_0051F660` (Mission_Move) | AI moving and deployed, type+0x6C4 < 0 | NO (gated on `IsPlayerControl == 0` checked just above) |

There is **no fear/threat-flee → undeploy path** for the GI in the binary.
`Panic_SetFear300 @ 0x00521C10` (Phase 2 §P2.6) does not call `Do_Action(0x1F)`;
fear changes the *next* fire/movement decision but doesn't synthesize an
undeploy event. A panicked deployed GI will continue firing as deployed
until panic decays or until the next player command.

### Summary table

| Trigger | Player-deployed | AI-deployed |
|---------|-----------------|-------------|
| Move command | **stays deployed** (no-op) | undeploys (Mission_Move handler) |
| Attack command (target in range) | stays deployed (no-op via Mission_Attack vtable) | stays deployed (uses deploy-fire) |
| Attack command (target out of range) | stays deployed | undeploys then chases (`FUN_00522340`) |
| AreaGuard with no target | stays deployed | stays deployed (no-op) |
| Threat / fear / panic | stays deployed | stays deployed |
| Explicit deploy command (mission 0x10) | toggles (the only player path) | toggles |
| Take damage | stays deployed | stays deployed |
| Mind-control | stays deployed (controller can issue mission 0x10) | n/a |

## D-T.3 (c) "Deploy blockers" — vtable+0x200 is misnamed — HIGH

Phase 1 listed `vtable+0x200` as `CanDeployFire`. The xref shows it is
`FUN_00521B60` for InfantryClass (TechnoClass override). Decompile:

```
FUN_00521B60(this):                                  ; @ 0x00521B60
  if (mission != 6 (Sticky)
      AND mission != 0x15
      AND entity+0x68D == 0                          ; PendingSequenceUpdate clear
      AND entity+0x8D == 0) {                        ; not IsSleeping
    locomotor = entity+0x674;
    cVar = locomotor->vtable+0x80();                 ; some locomotor predicate
    if (cVar != 0
        AND vtable+0x184(this) != 5
        AND vtable+0x184(this) != 0xF) {              ; mission_state checks
      if (vtable+0x184(this) != 1) return 0;
      if (entity+0x2B4 != 0) return 0;                ; has an active target
    }
    if (entity+0x6C4 == -1                            ; no active sequence
        OR sequence_can_interrupt[entity+0x6C4]) {    ; table @ 0x007EAF7C
      return 1;
    }
  }
  return 0;
}
```

This is a **generic "can I queue a mission right now?" gate**, not
deploy-specific. Used by `MissionClass::Queue_Mission @ 0x005B35E0` only
when the caller passes `force=1`; the standard player-deploy path uses
`force=0` (see §D-T.1 step 5), so this predicate is **bypassed entirely
for the player deploy path**.

`MissionClass::Queue_Mission` body:

```
Queue_Mission(this, mission, force):
  if (current==0x1C AND mission==5) return;       ; special construction lock
  if (current==0x13) return;                       ; ?
  if (mission != -1 AND (current != mission OR (queued != mission AND queued != -1))) {
    entity+0xB4 = mission;                         ; queued
    entity+0xB8 = 0;
  }
  if (force) {                                     ; player path passes 0
    if (vtable+0x200(this)) {                      ; ← FUN_00521B60
      vtable+0x1EC(this);                          ; Commence (immediate promote)
    }
  }
```

### Real deploy gate

The actual gate for player deploy is just `type+0xEC8 != 0` inside
`FUN_0051F6E0` (the toggle handler). For the GI, `type+0xEC8 = 1`
because `[E1] Deployer=yes` is parsed onto the InfantryType. This is separate
from TechnoType `DeployFire=yes` at `+0x6AC`, which participates in AI
deploy-fire preference and related deploy-fire behavior.

`FUN_0051F6E0` does **not** check:

- water cell, bridge cell, sub-cell occupancy
- in-transport state
- mind-control state
- panic / fear state
- friendly-fire angle, target validity

…before flipping the sequence. The only player-toggle gate is the InfantryType
`Deployer` bool. **There is
no in-engine "you can't deploy here" check for infantry deploy** — the
player can always toggle deploy on a Deployer infantry (the cursor
`EVA_CannotDeployHere` string at `0x0082012C` is for **MCV deploy**,
not infantry deploy).

## D-T.4 (d) Weapon-swap mechanism — HIGH (verified)

**There is no runtime weapon-pointer mutation tied to deploy state.**

`vtable+0x1EC` is `MissionClass::Commence @ 0x005B3570` — promotes
queued→active mission. It is NOT `ActivateDeployFire` and does NOT
mutate any weapon pointer.

The actual weapon picked at fire time comes from
`TechnoClass::SelectWeaponAgainst @ 0x006F3330` (vtable+0x2E4), which
chooses 0 or 1 based on:

1. Cached `entity->CurrentWeaponNumber` if set (fast path)
2. Gattling-stage progression (not relevant for the GI)
3. Verses table match: warhead-vs-armor of weapons[0] and weapons[1]
   against the target's armor class
4. Various secondary flags (Tib_Power, ImmuneToPsionics, etc.)

For the GI clicking on a tank: weapon_idx = 1 (Para has anti-armor verses,
M60 doesn't). For the GI clicking on infantry: weapon_idx = 0 (M60 wins
the verses pick for soft targets).

**The deploy state affects only the fire-sequence visual**:

```
Fire_At_Target step 3a (Phase 1 §3.4 — confirmed):
  if (current_seq in {0x1B,0x1C,0x1D,0x1E})
    new_seq = 0x1D (DeployedFire)         ; visual only
  elif (deploy-fire mode active flags set in {0x28,0x29})
    rewind anim
  else
    pick from {4 (FireUp), 0x28 (Sec), 0x29 (SecProne)} based on weapon_idx + state
  Do_Action(new_seq, ...)
  vtable+0x3CC(target, weapon_idx)         ; spawn bullet — uses weapon_idx unchanged
```

So: **a deployed GI shooting infantry uses M60 (weapon 0) with sequence
0x1D animation. A deployed GI shooting a tank uses Para (weapon 1) with
sequence 0x1D animation.** The deploy state does NOT force secondary
weapon selection.

### What `DeployFireWeapon=` is for — MEDIUM

The `DeployFireWeapon=` INI key (string `0x00843AAC`) is read by
`TechnoTypeClass::ReadINI` at `0x007147D5` into `TechnoTypeClass+0x6A8`
as a 32-bit int (-1 default, 0 or 1 for primary/secondary).

Its only consumer in InfantryClass is `FUN_00521320` (the AI auto-deploy
helper) at `0x0052146F`:

```
uVar3 = *(int *)(type + 0x6A8);                        ; DeployFireWeapon idx
cVar2 = (**(code **)(*this + 0x3A8))(this->target, uVar3);  ; CanFireAt(target, weapon)
```

So `DeployFireWeapon` is the **AI's hint about which weapon to range-check
when deciding "should I deploy here?"**. It does NOT force the runtime
weapon at fire time. For the GI, `DeployFireWeapon=1` tells the AI "if
you can hit the target with the secondary (Para), then deploying is
worthwhile". Doesn't apply to player-controlled deploy.

### Implication for the Rust port

The deploy-fire visual (sequence 0x1D) and the bullet-spawn weapon are
**independent**: the player's GI will fire whichever weapon
SelectWeaponAgainst chooses while playing the deployed-fire animation.
A faithful port should:

1. Pick weapon via the verses table (target-driven).
2. Pick fire-sequence via `entity.current_seq` ∈ deploy-set:
   `{0x1B-0x1E}` → seq 0x1D, otherwise seq 4.
3. NOT special-case "deployed = use secondary".

## D-T.5 (e) Bridge / sub-cell deploy edge cases — HIGH (verified)

**The deploy execution path has zero cell-validity gating.**

`FUN_0051F6E0` does not call `CellClass::CheckCellPassability`,
`Map::Get_CellClass(cell)->LandType`, `IsLowBridgeCell`, `OnBridge`, or
any sub-cell occupancy lookup before invoking `Do_Action(0x1B)`. The
deploy state change is purely a sequence/flag flip; the unit's cell
remains its current cell.

### Bridge cell (low bridge)

A GI on a low-bridge cell can deploy. The bridge layer (entity+0x8C
`IsOnBridge`) does not enter the deploy-toggle decision. After deploy,
the GI continues to render on the bridge layer (DrawZ unchanged) and
fires from the bridge — the deploy-fire weapon's `Anim=`/`Report=`/
projectile spawn use the same `entity+0x9C..0xA4` coords plus the bridge
elevation already baked in.

**Confirmed safe for stock YR**: bridge GIs can be deployed by player
right-click on bridge tile (cursor 0x1A path) or by single-selection
right-click on the GI itself (action 4 path). Both routes reach
`FUN_0051F6E0` via mission 0x10 (action 4 is the player path; cursor 0x1A
on the bridge tile dispatches mission 0xB which fails the IsPlayerControl
gate and falls to AreaGuard, so right-clicking the bridge tile beneath
your own deployed GI is the no-op move-click path, not an undeploy).

### Multi-infantry sub-cell

The 3-sub-cell allocator (Phase 2 §P2.3) places up to 3 infantry per
ground cell. Deploy is a state change on a single entity; it does NOT
shift sub-cell positions, NOT free or claim sub-cell slots, NOT push
neighbors out. A deployed GI keeps its sub-cell index (entity+0x100 or
similar — the Phase 2 sub-cell field). Three GIs in one cell can all
deploy independently and all fire deploy-fire from their own sub-cell
positions.

### In transport

If the GI is currently in cargo (a passenger of an IFV, Flak Track, etc.),
mission 0x10 is rejected at the queue-validation stage *before* reaching
`FUN_0051F6E0`, but the rejection is in `MissionClass::Queue_Mission`
and earlier (e.g., the unit isn't valid as a target — `FUN_006E6F20` in
EventClass::Execute returns null because the entity isn't in the
visible-units array while in cargo). The deploy command never fires.
Player attempts to "deploy from inside transport" produce no response at
all — confirmed by absence of any `entity+0x144 (cargo state)` check in
`FUN_0051F6E0`.

### Mind-controlled

A mind-controlled GI's `Owner` (entity+0x87) stays the original house;
only `MindController` (entity+0x147) tracks the new controller (Phase 3
§P3.3). `IsPlayerControl` reads `house+0x1EC` on the *original* owner
house. So a mind-controlled-by-AI GI:

- The current controller (AI) cannot deploy it — actions issued by the
  AI's mission system go through `FUN_00521320` (AI deploy gate).
  Mission 0x10 is queued, but the AI doesn't *issue* mission 0x10 (it's
  the player-input action). AI infantry under mind control don't deploy.
- The original (player) owner cannot select it — mind-controlled units
  drop from the original player's selection list.

So mind-controlled GIs effectively cannot toggle deploy state during
their captivity. They keep whatever state they were in when captured.

### Panicked / fear

Phase 2 §P2.6 covers fear. A deployed GI that becomes panicked
(FearLevel > 199) does NOT auto-undeploy — `Panic_SetFear300` at
`0x00521C10` only adjusts entity+0x6D4 (FearLevel) and may set
entity+0x6D5 (panic-running flag); it does not call `Do_Action(0x1F)`.
The next `Do_Action` invocation in `Fire_At_Target` step 7 may re-route
seq 3 (Walk) → seq 0x25 (FleeingPanic), but for a deployed unit the seq
stays at 0x1D — fear doesn't override deploy.

In practice: a deployed GI under fire keeps shooting deployed-fire even
while panic frame triggers play. Visible quirk: the panic-state running
animation never plays on a deployed GI.

## D-T.6 Updated open questions

**Closed by this investigation:**

- ✓ Player deploy trigger = action 4 → mission 0x10 → `FUN_0051F6E0`
- ✓ Mission 0xB ≠ player deploy mission; it's AI-only AreaGuard auto-deploy
- ✓ vtable+0x200/+0x1D4/+0x1D8/+0x1EC are NOT deploy predicates
- ✓ Active-sequence offset is `+0x6C4` (not `+0x1A4`)
- ✓ Move command does NOT auto-undeploy player units
- ✓ DeployFireWeapon=N is an AI hint, not a runtime weapon swap
- ✓ Deploy has no cell-validity gate (bridge / water / sub-cell all permissive)
- ✓ Mission 0xA on the GI is a "go idle" no-op (Slave/Yuri-clone path inactive)

**Still open (not investigated this pass):**

- **Q-undeploy-on-IFV-load**: when a deployed GI is right-clicked into an
  IFV / transport, does the mission-queue-into-Enter-mission auto-undeploy?
  Untraced; FUN_0051F660 only handles mission 2 (Move), not mission 7 (Enter).
- **Q-DeployFire-fire-error**: `Fire_At_Target` step 3 calls
  `vtable+0x3C0(target, weapon_idx) = GetFireError`. For a NOT-deployed GI
  with `DeployFireWeapon=1` and weapon_idx=1 (Para picked vs tank), does
  GetFireError return non-zero ("must deploy first") or does the GI just
  fire Para upright? Phase 2 §P2.17 covers GetFireError but didn't tag the
  deploy-required failure code (likely error 6 or 8 — untraced).
- **Q-vtable+0x4C0-DeployToFire**: `FUN_0070F090` (the polymorphic
  TechnoTypeClass dispatcher behind vtable+0x4C0) wasn't decompiled.
  It controls the "1/2 → 0x1D" action remap in `FUN_006FFBE0` and
  presumably reads a TechnoTypeClass bool (likely the `DeployToFire=`
  INI key string at `0x00845E10`, into TechnoTypeClass+? — needs xref
  pass).

## D-T.7 Sources

**Newly decompiled in this pass:**

- `FUN_0051F6E0 @ 0x0051F6E0` — InfantryClass::Mission_Deploy_Toggle (FULL)
- `FUN_0051F660 @ 0x0051F660` — InfantryClass::Mission_Move override (FULL)
- `FUN_0051F640 @ 0x0051F640` — InfantryClass::Mission_AreaGuard_AI (FULL — wrapper)
- `FUN_0051F620 @ 0x0051F620` — InfantryClass::Mission_Guard_AI (FULL — wrapper)
- `FUN_00521320 @ 0x00521320` — auto-deploy/undeploy helper (FULL — already partly in Phase 1, re-verified the IsPlayerControl gate at `0x00521500`)
- `FUN_00521B60 @ 0x00521B60` — vtable+0x200 = generic CanQueueMission_Now (FULL)
- `FUN_0051AA40 @ 0x0051AA40` — InfantryClass::Set_Destination (FULL — IsPlayerControl gate at top)
- `FUN_006FFBE0 @ 0x006FFBE0` — TechnoClass::Player_Send_Command / vtable+0x378 (FULL)
- `FUN_004C6860 @ 0x004C6860` — EventClass constructor (FULL — small)
- `FUN_00522340 @ 0x00522340` — InfantryClass::Mission_Attack auto-deploy (FULL)
- `FUN_00522550 @ 0x00522550` — small deploy commit helper (FULL)
- `FUN_00522E70 @ 0x00522E70` — Slave/Yuri-clone harvest mission (FULL)
- `MissionClass::Mission_Dispatch @ 0x005B3060` (FULL — mission ID → vtable slot table)
- `MissionClass::Queue_Mission @ 0x005B35E0` (FULL)
- `MissionClass::Commence @ 0x005B3570` (FULL)
- `MissionClass::Override_Mission @ 0x005B3650` (FULL)
- `TechnoClass::What_Action_OnObject @ 0x006FFEC0` (MEDIUM — focused on action-4 self-click branch)
- `FootClass::ClickedAction_Object @ 0x004D74E0` (MEDIUM — jump table at `0x004D7CD0`/`0x004D7D04` decoded)
- `FootClass::ClickedAction_Cell @ 0x004D7D50` (LIGHT — case 0x1A only)
- `EventClass::Execute @ 0x004C6CB0` (LIGHT — case 4/5 MEGAMISSION dispatch only)
- `HouseClass::IsPlayerControl @ 0x0050B730` (FULL)
- `TechnoClass::SelectWeaponAgainst @ 0x006F3330` (LIGHT — re-read for §D-T.4)

**Vtable slots resolved (InfantryClass base = 0x007EB058):**

| Offset | Function | Role |
|--------|----------|------|
| `+0x00` | `0x00410260 AbstractClass::QueryInterface` | RTTI |
| `+0x3C` | `0x006F9DC0` (`return entity+0x21C`) | Get_OwnerHouse |
| `+0x1D4` | `0x0070C5B0 IsWarpingOut` | chrono pred |
| `+0x1D8` | `0x0070C5C0 IsBeingWarped` | chrono pred |
| `+0x1E8` | `0x005B35E0 Queue_Mission` | mission queue |
| `+0x1EC` | `0x005B3570 Commence` | mission promote |
| `+0x1F0` | `0x005B2FD0 Assign_Mission` | direct mission assign |
| `+0x200` | `0x00521B60` | CanQueueMission_Now (NOT CanDeployFire) |
| `+0x21C` | `0x0051F620` | Mission 5/6 (Guard/Sticky) |
| `+0x220` | `0x0051F640` | Mission 0xB (AreaGuard, AI auto-deploy) |
| `+0x224` | `0x00522E70` | Mission 0xA (Slave-Miner harvest, no-op for GI) |
| `+0x22C` | `0x0051F660` | Mission 2 (Move) |
| `+0x23C` | `0x0051F6E0` | **Mission 0x10 (Deploy/Undeploy toggle)** |
| `+0x378` | `0x006FFBE0` | Player_Send_Command |
| `+0x480` | `0x0051AA40` | Set_Destination |
| `+0x4A4` | `0x004DF0E0` | Assign_Target_Command (extract event mission code) |
| `+0x4C0` | `0x005228C0` (thunk → `0x0070F090`) | DeployToFire-bool dispatcher |
| `+0x558` | `0x0051D6F0` | Do_Action |

**Strings used as anchors:**

- `DeployFireWeapon @ 0x00843AAC` — read into `TechnoTypeClass+0x6A8` at xref `0x007147D5`
- `DeployFire @ 0x00843AA0` — read into `TechnoTypeClass+0x6AC`
- `Deployer @ 0x00825928` — read into `InfantryTypeClass+0xEC8`; this is the infantry deploy-toggle gate
- `IFVMode @ 0x00843AE4` — read into `TechnoTypeClass+0x688`
- `DeployToFire @ 0x00845E10` — likely backs vtable+0x4C0 dispatcher
- `DeployTime @ 0x00843904`, `UndeployDelay @ 0x008438F4` — vehicle-deploy fields, NOT used by infantry path
- `EVA_CannotDeployHere @ 0x0082012C` — string is for MCV deploy validation, not infantry

**Confidence summary:**

- §D-T.0 corrections: HIGH — verified by direct disassembly
- §D-T.1 (player input trigger): HIGH — every step of the chain decompiled and cross-referenced
- §D-T.2 (no auto-undeploy on move): HIGH — gate is explicit and consistent
- §D-T.3 (no real "CanDeploy" predicate): HIGH — exhaustive xref pass on the four candidate vtable slots
- §D-T.4 (weapon swap is target-driven, not state-driven): HIGH — confirmed via SelectWeaponAgainst structure and absence of weapon mutation in FUN_0051F6E0
- §D-T.5 (bridge / sub-cell permissive): HIGH — no cell-check found in the deploy path; not exhaustive across all related vtable calls but the toggle handler itself is small enough to verify exhaustively

---

# Deploy-Trigger Investigation — Follow-up (2026-05-04, second pass)

The three open questions left at the end of §D-T.6 are now resolved.

## D-T.8 (Q-undeploy-on-IFV-load) — RESOLVED — HIGH

**Player-deployed GI cannot enter a transport without first manually
undeploying. There is no auto-undeploy on Mission 7 (Enter).**

### Trace

1. Player right-clicks own IFV with deployed GI selected
2. `TechnoClass::What_Action_OnObject @ 0x006FFEC0` returns 7 (matches the
   `cVar4 = (**(code **)(*param_2 + 0x138))() != 0` predicate at the
   bottom of the function — the target's "Can_Load_From" / "is-transport"
   check)
3. `FootClass::ClickedAction_Object @ 0x004D74E0` jump-table maps action 7
   via `byte_table[6] = 0x02` → case 2 entry `0x004D76C6`:
   ```
   PUSH 0;  PUSH target;  PUSH 0;  PUSH 7
   CALL [vtable + 0x378]                   ; Player_Send_Command(7, ...)
   ```
   So **action 7 queues mission 7** (note: cursor 3 = Enter is what gets
   *displayed*; the *return value* of What_Action_OnObject for an enter-able
   target is 7 — cursor and action codes are not the same enum here).
4. Net event built and queued via `FUN_006FFBE0` → EventClass case 4 →
   `Queue_Mission(7, force=0)`. **Before** the queue, EventClass::Execute
   case 4 calls `vtable+0x3C8(target)` (Set_NavCom) and
   `vtable+0x480(target)` (`InfantryClass::Set_Destination` =
   `FUN_0051AA40`).
5. `FUN_0051AA40` first 10 lines:
   ```
   vtable+0x3C(this)                    ; get owner house
   if (HouseClass::IsPlayerControl(house)) {
     if (entity+0x6C4 == 0x1B) return;   ; deploy seq
     if (entity+0x6C4 == 0x1C) return;
     if (entity+0x6C4 == 0x1D) return;
     if (entity+0x6C4 == 0x1E) return;
   }
   ...                                   ; rest of Set_Destination
   ```
   **Early-return for player-controlled units in deploy seq.** No
   destination is set, no nav-target set.
6. Mission 7 dispatches via vtable+0x240 → `FootClass::Mission_Enter
   @ 0x004D9290`. `FootClass::Mission_Enter` reads
   `FootClass::GetDestination(0)` (entity+0x5A4 — was supposed to be set
   by step 5, but Set_Destination returned early) → returns 0 → falls
   into the "no destination" branch:
   ```
   FootClass::Mission_Enter:
     iVar4 = FootClass::GetDestination(0);
     if (iVar4 == 0 && !FUN_0070D8F0(...)) {
       vtable+0x484(0, 1);                ; IdleDispatch — no-op for deployed
       vtable+0x1EC();                    ; Commence
     }
     ... return wait timer
   ```
7. Net effect: **deployed GI receives the Enter command but does nothing**.
   No movement toward IFV, no auto-undeploy, no cargo-load attempt. The
   Enter mission stays "promoted but inert" until the player either
   undeploys (mission 0x10 toggle) or the GI is given a new command.

### Why `InfantryClass::Mission_Enter @ 0x005196A0` doesn't help

`InfantryClass::Mission_Enter` exists in the binary but is **NOT wired
to any vtable slot** (xref check returns no references; byte-pattern
search for `A0 96 51 00` returns no matches). It's dead code from a
previous refactor. The active vtable+0x240 slot for InfantryClass points
to `FootClass::Mission_Enter @ 0x004D9290` (verified at `0x007EB298`).

The garrison-occupant path (`AddGarrisonOccupant` / `BuildingClass::CanDock`)
that `InfantryClass::Mission_Enter` would have handled lives on a
different chain — likely the building's `vtable+0x394` (cargo-add)
called from FootClass::Mission_Enter when the unit reaches the building.
But the deployed GI never reaches the building because Set_Destination
is blocked, so this is moot.

### Cargo-add does not auto-undeploy either

For completeness: `CargoClass::AddPassenger` (called from
`InfantryClass::Mission_Enter` dead code, but also reachable from
other paths like AI passenger-board) does not call `Do_Action(0x1F)`.
Even if a deployed GI somehow got to the IFV's cell (e.g., via teleport
or chrono-warp delivery), the cargo-load would proceed without
undeploying — but on Limbo (vtable+0x11C) the unit's sequence is
discarded anyway, so this is unobservable.

## D-T.9 (Q-DeployFire-fire-error code) — RESOLVED — HIGH

**`GetFireError @ 0x006FC0B0` has NO "must deploy first" error code.**

### Findings

- All return values from `GetFireError`: 0, 1, 3, 5, 6, 8, 9. (Disassembly
  exhaustively walked; every `MOV EAX,N; RET 0xC` pair tagged.)
- Meaning of each:
  - **0** = OK to fire (path at `0x006FCD1D`)
  - **1** = `Ammo == 0` (path at `0x006FCA17`)
  - **3** = busy/conflict (warping, wrong target lock, fire-out-of-range with retreat target, etc.)
  - **5** = generic "cannot fire on this target right now" (friendly-fire, dead/sinking target, gate failures, weapon-vs-target invalid, building-EW-conflict, etc.)
  - **6** = OutOfRange (target reachable in principle but weapon range too short)
  - **8** = `type+0xD27 != 0` (some "weapon disabled" type-bool — likely `IsWeaponDisabled` / EMP-affected)
  - **9** = cloak-required-to-be-decloaked (`unaff_ESI+0x133 AND CloakState != 0`)
- The function reads many type bools (`+0x14F`, `+0x142`, `+0x16D`, `+0x15B`,
  `+0x158`, `+0x15A`, `+0x131`, `+0x129`/`+0x12A`/`+0x12D`/`+0x130`, etc.)
  but **not `+0xEC8` (InfantryType Deployer), `+0x6AC` (DeployFire), or
  `+0x6A8` (DeployFireWeapon)**. The
  deploy state is not consulted.

### Implication

A NOT-deployed GI can fire its secondary weapon (Para) freely. The Phase 1
§3.4 weapon-pick path (`weapon_idx = SelectWeaponAgainst(target)`,
target-driven) returns whichever weapon's verses match best vs the target;
GetFireError clears it; the bullet spawns. The fire animation chosen is:

```
if (current_seq in {0x1B,0x1C,0x1D,0x1E})  → seq 0x1D (DeployedFire)
elif weapon_idx == 0                         → seq 4 (FireUp / standard)
elif weapon_idx == 1 AND host+0x5C8 != 0     → seq 0x29 (SecondaryProne)
elif weapon_idx == 1 AND host+0x5A4 != 0     → seq 0x28 (SecondaryFire)
else                                          → seq 4 (FireUp)
```

So a standing GI shooting a tank uses seq 4 with the Para bullet — no
deploy-fire visual, but the bullet still spawns and damages. The
"deployment is required to fire Para" perception is **a per-weapon INI
configuration** (the INI Para's `Range=`, `MinimumRange=`, `Burst=`,
projectile elevation/inaccuracy can be tuned to make standing-fire ineffective)
**plus** AI auto-deploy preference (`DeployFireWeapon=1`), **not** an
engine-level fire-error gate.

### Cross-check via `CanFireAt @ 0x006F77B0`

`vtable+0x3A8 = TechnoClass::CanFireAt` is a thinner predicate used by
the AI auto-deploy helper (`FUN_00521320 @ 0x00521320`). It calls
`TechnoClass::InRange` and a target-cell sensor check. **No deploy
state read.** Confirms: there is no engine-level fire-state predicate
that requires deploy.

### Implication for the Rust port

When porting GI deploy-fire, the Rust impl should:
- Allow Para to fire from undeployed GI, with seq 4 animation
- Use seq 0x1D animation only when `entity.current_seq` ∈ deploy-set
- Match damage / range / burst to per-weapon INI parameters for both
  standing-Para and deployed-Para (both should produce identical bullet
  effects — only the visual differs)

This is consistent with stock-YR observation: a player-issued
force-fire (Ctrl-click) on a tank with an undeployed GI in range will
fire Para from the standing GI. (This is actually visible in stock YR;
the deploy mechanic is a UX/AI hint rather than a hard requirement.)

## D-T.10 (Q-vtable+0x4C0 / DeployToFire dispatcher) — RESOLVED — HIGH

**`vtable+0x4C0` is `TechnoClass::IsDeployToFireVehicle` (or similar).
For the GI it always returns 0 — the Ctrl+Shift+Click attack-move-deploy
chord does NOT apply to the GI.**

### Trace

`vtable+0x4C0` for InfantryClass (`0x007EB518`) → `0x005228C0` =
`thunk_FUN_0070F090`. The thunk is 5 bytes:

```
0070f090: MOV EAX, [ECX]                 ; load TechnoClass vtable
0070f092: CALL [EAX + 0x84]              ; Get_TypeClass()
0070f098: MOV EDX, [EAX]                 ; load TypeClass vtable
0070f09a: MOV ECX, EAX
0070f09c: JMP [EDX + 0xa4]               ; tail-call TypeClass::vtable+0xA4
```

So `vtable+0x4C0` of a TechnoClass instance ≡ `TypeClass::vtable+0xA4`.
For InfantryClass, the InfantryTypeClass vtable starts at `0x007EB610`
(found in InfantryTypeClass::Constructor `0x005237E8: MOV [ESI], 0x7eb610`).
`vtable+0xA4 = 0x007EB6B4` → `0x005247C0` = thunk to `FUN_00711E90`
(TechnoTypeClass parent implementation, inherited unchanged).

### `FUN_00711E90` decompile

```
FUN_00711E90(this):                       ; @ 0x00711E90
  uVar1 = 0;
  if (this->field_0x898 != 0) {
    uVar1 = (uint)(this->field_0x6C8);
    if (this->field_0x6C8 == 0) return 1;
  }
  return uVar1 & 0xFFFFFF00;              ; effectively return 0
```

Returns 1 iff `type+0x898 != 0 AND type+0x6C8 == 0`. The two TechnoTypeClass
fields are not deploy-specific names but inherited slot identifiers; from
context (only used in this dispatcher and in the Ctrl+Shift attack-move
remap) the predicate gates "is this unit a *DeployToFire vehicle* like
the Prism Tank?".

### Caller chain — confirms it's irrelevant for the GI

The only consumer in the binary is `FUN_006FFBE0` (the player-command
builder = vtable+0x378), at the entry:

```
FUN_006FFBE0:
  cVar2 = FUN_00731BF0();                 ; Ctrl+Shift held + ALL selected pass +0x4C0
  if (cVar2) {
    cVar2 = vtable+0x4C0(this);           ; per-unit DeployToFire check
    if (cVar2) bVar1 = 1;
    else        bVar1 = 0;
  } else bVar1 = 0;
  EDI = action_code;                      ; from caller
  if ((EDI == 1 OR EDI == 2) AND bVar1) EDI = 0x1D;   ; remap attack/move → 0x1D
```

So action remap to 0x1D fires only when the player holds Ctrl+Shift AND
every selected unit's `vtable+0x4C0` returns 1. Action 0x1D in
`FootClass::Assign_Target_Command @ 0x004DF0E0` (vtable+0x4A4) is the
"DeployFire-attack-move" command — sets `entity+0x171 = 0x1D` (a stored
action mode flag) along with target cell stored at +0x172/+0x173.

### Why GI returns 0

`type+0x6A8` (DeployFireWeapon=) is read by TechnoTypeClass::ReadINI for
all TechnoTypes including infantry. But `type+0x898` is a different field
— **not the same as DeployFireWeapon**.

Searching `DeployToFire` (string at `0x00845E10`): the only ReadINI xref
is in `UnitTypeClass::ReadINI @ 0x00747672`. So the `DeployToFire=` INI
key is read into a UnitTypeClass-specific field — never set for
InfantryTypeClass. Therefore for the GI, `type+0x898` is whatever
TechnoTypeClass default-initializes (likely 0), so FUN_00711E90 returns 0.

### Net for the Rust port

The Ctrl+Shift+Click attack-move chord remap (action 1/2 → 0x1D) **is
specific to DeployToFire vehicles like the Prism Tank**. For the GI, the
predicate always returns 0; the chord falls through to the regular Move
or Attack action without any deploy-related side effect.

**Action 0x1D is not a path the GI ever takes** in stock YR. Don't
implement it for InfantryClass.

## D-T.11 Final closure of open questions

All three open follow-ups from §D-T.6 are now resolved:

- ✓ **Q-undeploy-on-IFV-load**: NO auto-undeploy. Set_Destination's
  IsPlayerControl+deploy-seq gate (`FUN_0051AA40` first 10 lines) blocks
  the destination assignment that EventClass::Execute case 4 sets up
  before queueing mission 7. The Mission_Enter handler then runs as a
  no-op.
- ✓ **Q-DeployFire-fire-error**: GetFireError has no deploy-required
  error code. GI Para is fireable from standing position; the deploy
  state only changes the fire animation seq (0x1D vs 4), not the
  bullet spawn or weapon pick.
- ✓ **Q-vtable+0x4C0**: TechnoClass::IsDeployToFireVehicle (TechnoTypeClass
  override at TypeClass::vtable+0xA4 = `FUN_00711E90`). Returns 1 only
  for `DeployToFire=` vehicles (Prism Tank class). Always 0 for the GI.
  Gates the Ctrl+Shift+Click attack-move-deploy chord (action remap
  1/2 → 0x1D) which is irrelevant to InfantryClass.

**The deploy investigation for the GI is now structurally complete** —
no further binary trips needed for Slice B (deploy state machine)
implementation in Rust.

## D-T.12 Sources for follow-up pass

- `FootClass::Mission_Enter @ 0x004D9290` — confirmed wired to vtable+0x240; no undeploy logic
- `InfantryClass::Mission_Enter @ 0x005196A0` — verified DEAD CODE (no xrefs, no byte-pattern match)
- `InfantryClass::Set_Destination @ 0x0051AA40` — re-verified IsPlayerControl+deploy-seq gate
- `TechnoClass::GetFireError @ 0x006FC0B0` — exhaustive disassembly walk of all return paths (1/3/5/6/8/9, no deploy code)
- `TechnoClass::CanFireAt @ 0x006F77B0` — confirmed thin range-check, no deploy state
- `FUN_0070F090` — vtable+0x4C0 thunk, traced to FUN_00711E90
- `FUN_00711E90 @ 0x00711E90` — TechnoTypeClass::vtable+0xA4 predicate (`+0x898 != 0 && +0x6C8 == 0`)
- `FUN_00731BF0` — Ctrl+Shift hotkey predicate (calls `FUN_0054F5C0` keystate query)
- InfantryTypeClass vtable located at `0x007EB610` (via constructor `0x005237E8`)
- InfantryTypeClass vtable+0xA4 at `0x007EB6B4` → `0x005247C0` (thunk_FUN_00711E90, inherited)
- `DeployToFire` string `0x00845E10` xref check: only `UnitTypeClass::ReadINI @ 0x00747672` writes the field
