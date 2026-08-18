# Chrono Miner Inbound Warp — Which Caller Arms It (Ghidra Report)

**Date:** 2026-07-19
**Status:** RESOLVED — closes the "OPEN — which caller supplies the DockUnload-building NavCom that arms Teleport" thread in `CHRONO_MINER_SYSTEM_OVERVIEW.md` §3.
**Method:** 4-lane read-only binary-trace swarm + synthesis + adversarial verifier (all Opus, ~850k tokens). Every headline claim re-checked at disassembly level; the adversarial pass attacked each load-bearing step and could not refute it. Independently re-verified by the parent session (callers of `0x0065AD30`, `Transmit_Radio_Impl` body).
**Confidence:** HIGH on the arming caller, the field identities, and the §3.1 correction (all byte-verified). MEDIUM on one runtime-only detail (gate (d), below).

---

## 1. Verdict

The open question conflated **two distinct roles**. Separating them resolves it:

- **Contact SOURCE** (what puts the refinery into the miner's `Contacts[0]`): the **miner's own HELLO (radio `0x02`)** sent from `UnitClass::Mission_Harvest` state 2 (`0x0073E5E0`, at `LAB_0073ee51`). On ROGER, `RadioClass::Transmit_Radio_Impl` (`0x0065A970`) writes `refinery → miner.Contacts[0]`.
- **ARMING CALL** (what flips the locomotor Drive→Teleport): **`FootClass::Receive_Radio` case `0x12` (MOVE_TO_CELL), call site `0x004D91EB`** = `Set_Destination(dockCell, 1)`. This is the `Set_Destination` invocation whose Gate-1 block (in `0x00741970`) sees `Contacts[0]` = the `DockUnload` refinery and `param_2` = the dock cell, and keeps Teleport active.

There is also a **redundant re-arm** at `Mission_Enter` Site C (`0x004D941D`) that nets the same single warp (see §5).

### The full arming chain (all byte-verified)

```
UnitClass::Mission_Harvest state 2  (0x0073E5E0)
  refinery within ChronoHarvTooFarDistance (50 cells)?
    → Transmit(HELLO 0x02, refinery)   @ LAB_0073ee51
        → ROGER → Transmit_Radio_Impl writes refinery into miner.Contacts[0]
    → set substate 3
  substate 3 → Queue_Mission(MISSION_ENTER=7)          (vtable+0x1e8 = 0x005B35E0)
FootClass::Mission_Enter  (0x004D9290)
  → Transmit(CAN_DOCK 0x0E, Contacts[0]=refinery)      (vtable+0x278)
      → BuildingClass::Receive_Radio case 0x0E  (0x0043C2D0)
          dock cell = building NW anchor + (3,1)        (CONCAT22(y+1, x+3))
          → Transmit(MOVE_TO_CELL 0x12, &dockCell, miner)
              → FootClass::Receive_Radio case 0x12  (0x004D8FB0)
                  → Set_Destination(dockCell, 1)        @ call site 0x004D91EB
                      → TechnoClass::Set_Destination (UnitClass override, 0x00741970)
                          GATE 1 arming block (0x7423CD..):
                            (a) Contacts[0] (= Contact_With_Whom(0)) is a Building (What_Am_I==6)   ✓ refinery
                            (b) building.Type + 0x16B3 (DockUnload=) set                            ✓
                            (c) param_2 (dockCell) is a CellClass (What_Am_I==0xB)                  ✓
                            (d) CellClass::FindFirstUnit(dockCell) (0x0047EBA0) == NULL             ✓ (pad empty)
                          → keep TeleportLocomotion active
                      → tail Set_Destination_Internal → HeadToCoord on Teleport → warp arms next
                        tick in TeleportLocomotionClass::StateMachineTick (0x7192F0)
```

---

## 2. Field identity — `+0xE4` (Contacts) vs `+0x5A4` (NavCom): the crux

The arming gate reads **`Contact_With_Whom(0)` = `Contacts[0]` = `+0xE4[0]` (the radio contact)**, **not** the committed-destination pointer `+0x5A4` (`param_1[0x169]`). These are different fields:

- **`0x0065AD30`** body: `return *(*(this+0xE4) + index*4)`. Disasm: `MOV EAX,[ECX+0xE4]; MOV ECX,[ESP+4]; MOV EAX,[EAX+ECX*4]; RET 0x4`.
- **`+0xE4` is the RadioClass `Contacts[]` array**: `Transmit_Radio_Impl` (`0x0065A970`) uses `param_1[0x39]` (=`+0xE4`) as the contact array and `param_1[0x3a]` (=`+0xE8`) as its count/capacity — writing `Contacts[slot]=target` on HELLO-ROGER and clearing it on BREAK.
- **Proven distinct from `+0x5A4`**: `FootClass::Receive_Radio` case `0x17` does `CMP [this+0x5A4], EAX` where `EAX = Contact_With_Whom(0)` — a no-op if they were the same field.

### Label correction applied (Ghidra)

`0x0065AD30` was mislabeled **`FootClass__GetDestination`**. It is **not** a destination getter — its callers include `BuildingClass::Receive_Radio`, `BuildingClass::Destroy`, `SuperClass::Launch`, `WarheadTypeClass::Detonate`, `TemporalClass::CanWarpTarget` (none of which have a NavCom). Renamed to **`RadioClass__Contact_With_Whom`** (2026-07-19), with a plate comment recording the old label and the `+0xE4`/`+0x5A4` distinction.

---

## 3. Correction to `CHRONO_MINER_SYSTEM_OVERVIEW.md` §3.1

§3.1 argued the far path "fails Gate 1 because `param_1[0x169]` (`+0x5A4`) == NULL." **Wrong field, right conclusion.** Gate 1 never reads `+0x5A4`; it reads `Contacts[0]` (`+0xE4[0]`). The far path *does* fail Gate 1 — but because **`Contacts[0]` is NULL** (no HELLO/ROGER link has been formed yet on the far path), so the "current radio contact is a `DockUnload` building" test (a) fails. Both fields happen to be NULL on the far path, but the *causal* one is `Contacts[0]`. (Overview patched to match.)

---

## 4. Close-vs-far: normal case is a FULL ore→refinery jump

Governed by `[General] ChronoHarvTooFarDistance` = **50 cells** (`Rules+0xD7C`; non-chrono harvesters use `HarvesterTooFarDistance`=5 at `Rules+0xD78`). State 2's gate: `dist ≤ Rules+0xD7C << 8` (leptons), at `0x0073ee40`.

- **Refinery within 50 cells (the normal case):** the miner links (HELLO) *from the ore position*, so the arming chain above fires with `Contacts[0]`=refinery and warps in **one hop from the ore patch to the dock cell** (anchor+(3,1)). This matches the in-game observable and the WarpOut-at-depart-cell behavior.
- **Refinery beyond 50 cells (huge/opposite-side map):** **no HELLO is sent** → `Contacts[0]` stays NULL → Gate 1 fails → the miner **DRIVES** to a staging cell (refinery cell + `BuildingType+0x1618/+0x161C` = art `QueueingCell`), and only warps the final approach once it is within 50 cells and re-links. The stock INI comment states this intent ("...they will stay on their side of the map").

So the observable inbound warp is a **full jump** in the normal case — **not** merely a short final-approach hop. (Drive-first is the >50-cell fallback only.) This supersedes the "UNVERIFIED — structurally plausible that the real warp is a SHORT final-approach hop" speculation in overview §3.

---

## 5. Ruled-out / secondary callers

- **`Mission_Enter` Sites A (`0x4D945C`) and D (`0x4D92E2`)** use `vtable+0x484` (= `0x00738970` = `UnitClass::Scatter_Force`), **not** `Set_Destination` (`vtable+0x480` = `0x741970`), and both pass `param_2=0`. **Do not arm.**
- **`Mission_Enter` Site B** (`Set_Destination(NavList[0], 0)`) is reached only when `+0x5A4`==0; a successful `DockUnload` CAN_DOCK sets `+0x5A4` first, so the stock flow routes to Site C instead. **Not the miner-dock arming path.**
- **`Mission_Enter` Site C (`0x004D941D`)** *does* satisfy all four gates (Contacts[0]=refinery, param_2=dock cell), but it is a **redundant re-issue** of the same cell that `Receive_Radio` case `0x12` already set synchronously inside the same `Transmit(0x0E)` call. Net observable: one warp either way.
- **The internal "Dock= type-list re-target" blocks in `0x741970`** (transport/carrier queue at `+0xD2C`; dockable-building-type list at `+0x3EC/+0x3F8`) only *condition `param_2` into a cell*; they do not independently arm (the flip still requires `Contacts[0]`=DockUnload building) and are skipped in the stock miner flow.

---

## 6. Honest open gaps (no invented closure)

- **Gate (d)** — `FindFirstUnit(dockCell)==NULL` — is assumed for a normally-empty pad. A contested/occupied dock cell makes the miner drive the last stretch instead. Not statically certifiable; needs a live trace.
- **Warp origin cell** — the byte-proof shows the HELLO is *sent* from the harvest position, but does not prove zero drive occurs between the state-3 HELLO and the case-`0x12` arm (1–2 ticks; miner is ~stationary). "Origin ≈ ore cell" is empirically supported by the in-game WarpOut-at-depart observation, not statically proven.
- **Site C** no-op-vs-refire depends on same-tick ordering; nets one warp regardless.

---

## 7. Sources (all live against `gamemd.exe`, project `testProsjekt`)

- `RadioClass__Contact_With_Whom` (renamed from `FootClass__GetDestination`) `0x0065AD30` — body reads `+0xE4[index]`; callers include Building/Super/Warhead classes.
- `RadioClass__Transmit_Radio_Impl` `0x0065A970` — `+0xE4`=Contacts[], `+0xE8`=count; HELLO adds, BREAK removes.
- `TechnoClass::Set_Destination` (UnitClass override) `0x00741970` — Gate-1 arming block; `PUSH 0x0; CALL 0x0065ad30` at `0x0074240F`; four-condition AND chain (`CMP EAX,0x6`; `[type+0x16B3]` DockUnload; `SUB EAX,0xB` cell; `CALL 0x0047EBA0` FindFirstUnit).
- `FootClass::Receive_Radio` `0x004D8FB0` — case `0x12` call site `0x004D91EB` = `Set_Destination(*param_4, 1)`; case `0x17` compares `Contact_With_Whom(0)` vs `+0x5A4`.
- `BuildingClass::Receive_Radio` `0x0043C2D0` — case `0x0E` builds dock cell NW+(3,1), sends `0x12`.
- `FootClass::Mission_Enter` `0x004D9290` — four Set_Destination-family sites (A/D use vtable+0x484; B unreachable; C redundant arm).
- `UnitClass::Mission_Harvest` `0x0073E5E0` — state 2 HELLO at `LAB_0073ee51`; threshold gate at `0x0073ee40` (`Rules+0xD7C`).
- `Find_Docking_Bay(arg3=1)` `0x004DF040`→`0x004DEE80` — permissive candidate query, does not establish a radio link (touches neither `+0xE4` nor `+0x5A4`).
- INI: `ini/rulesmd.ini:294 ChronoHarvTooFarDistance=50`, `HarvesterTooFarDistance=5`.
- Lane / synthesis / verdict scratch: `scratchpad/chrono-warp-caller/{L1..L4,SYNTHESIS,VERDICT}.md`.
