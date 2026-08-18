# C4 on Bridge Repair Hut (CABHUT) — Upstream Gate Investigation

**Date:** 2026-05-12
**Trigger:** `project_c4_bridge_hut_followup` — Rust port observation that
right-clicking CABHUT with a SEAL/Tanya selected "does nothing observable."
Hypothesized upstream cause was an `Immune=yes` gate in gamemd.
**Scope:** Verify whether gamemd.exe contains an upstream gate (cursor /
order / mission-assign / on-arrival) that rejects C4 placement on
`BridgeRepairHut=yes` buildings.
**Verdict:** **No such upstream gate exists in gamemd.** The hypothesis
in `BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md` §15.2 (and in the
memory entry) is **refuted by direct binary evidence**. In vanilla YR,
Tanya/SEAL C4 placement on CABHUT proceeds normally; the hut survives
the explosion and the bridge collapses via the `BuildingClass::Update`
BridgeRepairHut branch already documented in §3.2 / §14.

This investigation also corrects a stale offset claim in §15.1 of the
bridge report: `Immune` is at **ObjectTypeClass + 0x233**, not at
`+0xC4D`.

---

## 0. TL;DR

For SEAL/Tanya hovering a CABHUT in gamemd:

1. `InfantryClass::What_Action_OnObject` (0x51E3B0) reaches a C4-plant
   block keyed on `InfantryType.C4` (`+0xec2`).
2. The block returns **action `0x10`** (the Sabotage/C4 cursor) because
   CABHUT satisfies `CanC4` (default-`true`, verified in the constructor
   at `0x45E063`) and does **not** set `InvisibleInGame` (`+0x1701`).
3. **`return 0x10` is a direct return** — it bypasses the
   `Immune`-downgrade tail at `0x51F171` that converts `ACTION_ATTACK`
   (5) → `ACTION_NOMOVE` (2).
4. The click dispatches `Mission_Sabotage` (mission code `0x11`). On
   arrival, `InfantryClass::PerCellProcess` Mission_Sabotage branch
   gates on `vtable[0x160] (IsIronCurtainActive)` and
   `BuildingClass.field_0x6df` — **not on `Immune`**.
5. When the C4 timer expires, `BuildingClass::Update` (0x43FB20) routes
   through the `BridgeRepairHut` branch (per bridge report §14.2):
   bridge is destroyed; **the hut does not take damage** — so
   `Immune=yes` is moot in the damage path too.

**Net:** the "C4 click rejected upstream" symptom observed in the Rust
port is **not** a faithful reproduction of any gamemd gate. If parity is
the bar, the Rust port should let C4 placement on CABHUT proceed
normally, with the bridge collapse cascading as the existing
ignored-test `c4_on_cabhut_destroys_bridge_when_upstream_immune_lifted`
already expects.

---

## 1. Symptom reframe

**Reported (memory `project_c4_bridge_hut_followup`):**
> "right-clicking CABHUT (Bridge repair hut) with a SEAL selected does
> nothing observable" — in the Rust port, as of dev branch May 2026.

**Hypothesis under investigation:** there is an "upstream Immune gate"
in gamemd between the click and the damage path that rejects C4-on-CABHUT
silently, and the Rust port (which doesn't check `Immune` in its C4
plant code) needs the gate ported in.

**What "upstream gate" could mean (candidate hypotheses, ranked):**

| H | Location | Mechanism | Initial confidence |
|---|----------|-----------|--------------------|
| H1 | `InfantryClass::What_Action_OnObject` | Cursor downgrades to MOVE/NONE for `Type.Immune` targets | HIGH (Phase 2 hypothesis) |
| H2 | Click→Mission dispatch | Action 0x10 mapped to `Mission_None` for Immune | MED |
| H3 | `PerCellProcess` Mission_Sabotage | On-arrival predicate rejects Immune | MED |
| H4 | `BuildingType.+0x1701` or similar | A different type-flag silently gates | LOW |
| H5 | `BridgeRepairHut=yes` itself short-circuits | Type-flag gate, not Immune | LOW |
| H6 | No gate exists in gamemd | The Rust-port "click does nothing" is a port bug, not parity work | (initially unranked — surfaced after evidence) |

Per `feedback_brainstorm_verification_preflight`: the bridge report
§15.2 prediction that vtable[0x160] is the gate was already refuted in
Phase 2 — leaving H1 / H2 / H3 as candidates. This investigation
**verifies them directly** rather than building on §15.2's residual
hypothesis.

---

## 2. Verified type-flag offsets (with confidence axes)

Per `feedback_research_confidence_axes`, each offset claim is qualified
on three axes: **content** (what bytes are there), **identity** (what
the field is named), **binding** (where it's consumed).

| Field | Class | Offset | Content | Identity | Binding | Verified at |
|-------|-------|--------|---------|----------|---------|-------------|
| `Immune` | `ObjectTypeClass` | `+0x233` | HIGH | **HIGH** | HIGH | `ObjectTypeClass::ReadINI @ 0x5F9510` (`MOV [EBX+0x233], AL` after `ReadBool("Immune", ...)`) — **see §3 for the correction to bridge report §15.1's +0xC4D claim** |
| `LegalTarget` | `ObjectTypeClass` | `+0x231` | HIGH | HIGH | HIGH | Same function, `MOV [EBX+0x231], AL` after `ReadBool("LegalTarget", ...)` |
| `Insignificant` | `ObjectTypeClass` | `+0x232` | HIGH | HIGH | — | Same function, `MOV [EBX+0x232], AL` after `ReadBool("Insignificant", ...)` |
| `Bombable` | `ObjectTypeClass` | `+0x22e` | HIGH | HIGH | — | Same function, `MOV [EBX+0x22e], AL` after `ReadBool("Bombable", ...)` |
| `BridgeRepairHut` | `BuildingTypeClass` | `+0x16b6` | HIGH | HIGH | HIGH | `BuildingTypeClass::ReadINI @ 0x460E9A` (`MOV [EBP+0x16b6], AL` after `ReadBool("BridgeRepairHut", ...)`) |
| `Capturable` | `BuildingTypeClass` | `+0x1572` | HIGH | HIGH | HIGH | `ReadINI` chain: `ReadBool("Capturable", ...) → MOV [param_1+0x1572], AL` |
| `Spyable` | `BuildingTypeClass` | `+0x1576` | HIGH | HIGH | — | Same chain |
| `CanC4` | `BuildingTypeClass` | `+0x1577` | HIGH | HIGH | HIGH | `BuildingTypeClass::ReadINI` chain; default initialized to `1` in constructor at **`0x45E063`** (`C6 86 77 15 00 00 01` = `MOV byte ptr [ESI+0x1577], 1`) — **confirms CanC4 defaults to TRUE for all buildings** |
| `InvisibleInGame` | `BuildingTypeClass` | `+0x1701` | HIGH | HIGH | HIGH | `BuildingTypeClass::ReadINI`: `ReadBool("InvisibleInGame", ...) → MOV [param_1+0x1701], cVar5` |
| `Engineer` | `InfantryTypeClass` | `+0xec3` | HIGH | HIGH | HIGH | `InfantryTypeClass::ReadINI @ 0x524584` (`MOV [ESI+0xec3], AL` after `ReadBool("Engineer", ...)`) — **matches the predicate in `InfantryClass::What_Action_OnObject` engineer-block** |
| `C4` | `InfantryTypeClass` | `+0xec2` | HIGH | HIGH | HIGH | `InfantryTypeClass::ReadINI @ 0x524559` (`MOV [ESI+0xec2], AL` after `ReadBool("C4", ...)`) — **the gate flag for action 0x10** |

Vtable binding (verified by `read_memory` per
`feedback_vtable_binding_verification`):

| Slot | Class | Address | Function | Verified |
|------|-------|---------|----------|----------|
| `vtable[0x88]` (slot 34) | `BuildingClass` | `0x459EE0` | `GetType()` (returns `*(this + 0x520)`) | Read of `0x7E3EBC + 0x88` returned `E0 9E 45 00`; decompiled body returns `*(param_1 + 0x520)` — i.e., simple getter for `BuildingClass.Type` pointer |
| `vtable[0x7c]` (slot 31) | `BuildingClass` | `0x5F6C10` | `ObjectClass::IsAboveGround` | Read of `0x7E3EBC + 0x7C` returned `10 6C 5F 00`; named function in Ghidra |
| `vtable[0x160]` (slot 88) | `BuildingClass` | `0x41BF40` | `TechnoClass::IsIronCurtainActive` | (already verified in bridge report §15.1 Phase 2) |

---

## 3. Correction to bridge report §15.1: `Immune` is at `+0x233`, not `+0xC4D`

Bridge report §15.2 states:

> "there is likely a `WeaponClass::Can_Attack` or `Mission::Can_Sabotage`
> (or similar) predicate that consults the target's `Type[+0xC4D]`
> (Immune)."

This **+0xC4D claim is wrong**. Direct verification inside
`ObjectTypeClass::ReadINI @ 0x005F92D0` (verified via
`get_function_by_address 0x005F92D0`; `0x5F94F4` cited below is the PUSH of
the "Immune\0" string within the body, not the function entry):

```
005f94e2: CALL  0x005276d0                ; (prior read setup)
005f94e7: MOV   CL, byte ptr [EBX+0x233]  ; default for Immune
005f94f4: PUSH  0x832b70                  ; "Immune\0"
005f94f9: PUSH  EBP                       ; section ptr
005f94fc: CALL  CCINIClass::ReadBool
005f9510: MOV   byte ptr [EBX+0x233], AL  ; store Immune result to +0x233
```

The "Immune\0Strength\0\0\0\0LegalTarget\0" string block at
`0x832B70`–`0x832B84` is read in tight sequence with the `+0x231`/
`+0x232`/`+0x233` reads. There is no Immune field at `+0xC4D` — the
+0xC4D number appears nowhere in the binary in this context. The bridge
report's §15.1 hypothesis section was written under an incorrect offset
assumption; this doc supersedes that claim for offset purposes.

The bridge report's §15.3 lesson ("vtable bindings must be verified by
`read_memory`") applies symmetrically to **field offsets** — both the
TS-source-code conventions and YRpp/Ares-derived constants can be wrong
when ported forward. This investigation re-verified every offset by
finding the `ReadBool(literal-key-string, default-byte-at-offset)`
sequence in the corresponding `ReadINI`.

---

## 4. The Immune-downgrade gate exists — but does **not** fire for Tanya/SEAL on CABHUT

There **is** an Immune-based action downgrade in gamemd, in **three**
locations (one per movement-class):

| Site | Class | Function | Pattern |
|------|-------|----------|---------|
| `0x51F171` | InfantryClass | `What_Action_OnObject` | If resolved action `== 5` (ATTACK), and `target.GetType().Immune != 0`, **set action `= 2`** (NOMOVE) |
| `0x417F5B` | AircraftClass | `What_Action` | same shape |
| `0x740492` | UnitClass | `What_Action_OnObject` | same shape |

Assembly at `0x51F160` (verified by `read_memory`):

```
83 fd 05                ; CMP   EBP, 5                ; iVar7 == 5 (ACTION_ATTACK)?
75 19                   ; JNZ   +0x19                  ; if not, skip downgrade
8b 06                   ; MOV   EAX, [ESI]             ; load target.vtable
8b ce                   ; MOV   ECX, ESI               ; ECX = target (this)
ff 90 88 00 00 00       ; CALL  [EAX+0x88]             ; vtable[0x88] = GetType
8a 88 33 02 00 00       ; MOV   CL, byte ptr [EAX+0x233] ; load Immune
84 c9                   ; TEST  CL, CL
74 05                   ; JZ    +5                     ; if !Immune, keep ATTACK
bd 02 00 00 00          ; MOV   EBP, 2                 ; ACTION = NOMOVE
[epilogue]
```

**This gate fires only when `iVar7 == 5` at the bottom of the function.**
For Tanya/SEAL on CABHUT, `iVar7` is **not** 5 at this point — it is
already replaced by a direct `return 0x10` from the C4-plant block
earlier in the function (see §5). The Immune downgrade is therefore
**bypassed** for the SEAL/Tanya-on-CABHUT case.

Confidence: HIGH on content, HIGH on identity, HIGH on binding
(vtable[0x88] verified to be a trivial `GetType` returning `+0x520`).

---

## 5. The C4-plant block at `What_Action_OnObject` returns `0x10` for CABHUT — direct return, bypassing Immune

Inside `InfantryClass::What_Action_OnObject` (well before the bottom
Immune-downgrade tail), there is a block specifically for
`InfantryType.C4`-bearing infantry against capturable-style buildings:

```c
// pseudocode reconstructed from decompilation
if (IsHumanPlayer()
    && (attacker.Type.C4 /* +0xec2 */ != 0
        || TechnoClass::HasWeaponAbility(0xe))
    && iVar7 == 5 /* ACTION_ATTACK so far */
    && target.GetRTTI() == 6 /* Building */
    && !target.vtable[0x80]() /* not IsDestroyed */)
{
    if (target != null
        && target.GetRTTI() == 6
        && target.Type.CanC4 /* +0x1577 */ != 0
        && target.Type.InvisibleInGame /* +0x1701 */ == 0)
    {
        return 0x10;   // direct return — C4-plant cursor
    }
    return 5;          // direct return — fall back to ATTACK
}
```

For SEAL/Tanya on CABHUT, every conjunct holds:

| Condition | Value for CABHUT case | Source |
|-----------|----------------------|--------|
| `IsHumanPlayer()` | yes (player click) | runtime |
| `attacker.Type.C4` | `1` for both SEAL and TANY | [ini/rulesmd.ini:4027,4078](../../ra2-rust-game/ini/rulesmd.ini) |
| `iVar7 == 5` | yes (CABHUT has `LegalTarget=yes`, so `TechnoClass::What_Action_OnObject` returns 5) | [ini/rulesmd.ini:16341](../../ra2-rust-game/ini/rulesmd.ini) + decompile of `TechnoClass::What_Action_OnObject @ 0x6FFEC0` |
| `RTTI == 6` | Building | runtime |
| `!IsDestroyed` | yes (CABHUT alive) | runtime |
| `CanC4` | **`1`** — constructor default at `0x45E063` initializes `+0x1577 = 1`; CABHUT does not override | binary + INI |
| `InvisibleInGame` | `0` — CABHUT does not set `InvisibleInGame=yes` | [ini/rulesmd.ini:16336-16352](../../ra2-rust-game/ini/rulesmd.ini) |

**Therefore the function returns `0x10` directly.** No further iVar7
manipulation; no Immune check.

Confidence: HIGH on content, HIGH on identity, HIGH on binding.

---

## 6. Action `0x10` dispatches `Mission_Sabotage` (0x11), and its on-arrival branch does **not** consult `Immune`

`InfantryClass::PerCellProcess` (0x519630) contains an explicit
Mission_Sabotage branch (mission code `0x11`):

```c
iVar4 = infantry.vtable[0x184]();  // get current Mission
if (iVar4 == 0x11 && infantry.Type.C4 /* +0xec2 */ != 0) {
    target_building = Look_up_building_in_cell();
    if (target_building != null && target_building == infantry.NavTarget) {
        // Pre-attach gates:
        target_mission = target_building.vtable[0x184]();
        if (target_mission != 0x13               /* not Construction */
            && target_building.vtable[0x160]() == 0  /* !IsIronCurtainActive */)
        {
            if (target_building.field_0x6df != 0) {
                // C4 already planted (or Crewed-survivor cooldown active per §14.2);
                // walk to building, no plant
                ...
                return;
            }
            target_building.field_0x6df = 1;   // mark C4 attached
            // ... allocate BombClass, attach to target_building.field_0x150 / +0x14b / etc.
        }
        ...
        return;
    }
}
```

**Gates consulted in this branch:**
- `target_building.GetMission() != 0x13` (not in Construction)
- `target_building.IsIronCurtainActive() == 0` (vtable[0x160] — verified
  in bridge report §15.1 to be the IronCurtain check, not Immune)
- `target_building.field_0x6df == 0` (no prior C4; flag is dual-purpose
  per bridge report §14.1, but for CABHUT only the C4-plant interpretation
  applies because `Immune=yes` prevents the Crewed-survivor path from
  ever setting it — per §14.2)

**Not consulted:** `Immune` (`+0x233`). No `MOV CL, byte ptr [...+0x233]`
read appears in the Mission_Sabotage branch — verified by re-decompile.

CABHUT-specific evaluation:
- CABHUT is `Mission_Standby` or `Mission_Guard` at idle, never `0x13`
  (Construction) at hover time — passes.
- CABHUT, un-curtained, has `IsIronCurtainActive() == 0` — passes.
- CABHUT, fresh, has `field_0x6df == 0` — passes.

So the C4 attach proceeds. The infantry calls something equivalent to
`BombClass::Attach`-for-building (a different code path from the
RTTI==0xf path in `BombClass::Attach @ 0x438E70`, which only handles a
specific RTTI category), the building's destruction-timer field is
populated from `RulesClass.C4Delay` (`g_RulesClass + 0xFD0`), and the
infantry is despawned via `vtable[0xF8]` (`Limbo`).

Confidence: HIGH on content (direct decomp + assembly inspection),
HIGH on identity of the missions and flags, MEDIUM-HIGH on binding
(`vtable[0x160]` is verified; `vtable[0x184]` mission-getter is a
strong inference but not yet `read_memory`-verified — could be added in
a follow-up).

---

## 7. Damage path (C4 timer expiry) also bypasses `Immune` for CABHUT

Per bridge report §3.2 and §14.2 (already accepted), when the C4 timer
expires:

`BuildingClass::Update` (`0x43FB20`) checks `field_0x6df == 1` and
expiry condition, then **branches on `Type.BridgeRepairHut` (`+0x16b6`)**:

- If `BridgeRepairHut`: dispatches `DestroyBridge_*_MapInit` on
  neighboring bridge cells; **does NOT call `vtable[0x16C]` (area
  damage)** on the hut itself. The hut keeps its HP; the bridge
  collapses; `field_0x6df` is cleared.
- Otherwise: applies area damage via `vtable[0x16C]` with the C4
  warhead.

**Implication for Immune:** for CABHUT, the damage path is never
entered — `vtable[0x16C]` (and therefore `ReceiveDamage`'s `Immune`
early-out at `0x701F45`) is bypassed structurally. The hut "survives"
the C4 not because Immune absorbed the damage, but because the
BridgeRepairHut branch never asks for damage.

(The bridge report's §14.2 already documents this; reproducing here for
the full upstream-to-downstream trace.)

---

## 8. Asymmetry with the engineer path — confirms the right mental model

The bridge report §3.1 documented that the engineer-on-CABHUT path lives
in the `Mission ∈ {8, 0xB, 0x19}` (Capture / Enter / similar) branch of
`PerCellProcess` and is gated by `Type.Engineer (+0xec3)`. The same
function also has an **explicit BridgeRepairHut early-return at the top
of `What_Action_OnObject`** for engineers (gated by `Type.Engineer` and
`BuildingType.Capturable` at `+0xccc`):

```c
if (attacker.Type.Engineer /* +0xec3 */
    && target.RTTI == 6
    && IsHumanPlayer()
    && !target.vtable[0x80]()  /* not destroyed */
    && target.Type.+0xccc /* (Capturable-class flag, TBD identity) */)
{
    if (target.Type.BridgeRepairHut /* +0x16b6 */) {
        // engineer-on-CABHUT short-circuit → return 0x1d or 0x20
        ...
    }
    // (rest of engineer-capture handling)
}
```

The fact that the engineer block has an **explicit short-circuit for
BridgeRepairHut** (returning a dedicated cursor) is a deliberate
developer choice. The fact that the **InfantryType.C4 block has NO
analogous short-circuit** is also deliberate: the C4 cursor (0x10) is
shown normally, and the C4-plant-on-CABHUT path is supported
end-to-end, with the BuildingClass::Update branch routing destruction
to the bridge.

Strong support for H6: **no upstream gate is supposed to exist** here.
The engineer path and the Tanya/SEAL path are symmetric in design —
both succeed against CABHUT, via different downstream cascades.

---

## 9. Verdict on the original hypotheses

| H | Verdict | Evidence |
|---|---------|----------|
| H1 — `Immune` downgrades cursor for SEAL/Tanya on CABHUT | **REFUTED** | C4-plant block returns `0x10` directly; Immune downgrade only fires when `iVar7 == 5` at the function tail (`0x51F171`) — not reached |
| H2 — Click→Mission dispatch rejects action `0x10` on Immune | **REFUTED** (by inference) | No Immune read in the Mission_Sabotage branch of `PerCellProcess`; no other dispatcher between What_Action and PerCellProcess in the click→mission→tick chain has a known Immune gate |
| H3 — `PerCellProcess` Mission_Sabotage rejects Immune target | **REFUTED** | Direct decompile of `0x519630` Mission `0x11` branch shows the gates are `target.GetMission() != 0x13`, `vtable[0x160]() == 0`, and `field_0x6df == 0` — no Immune read |
| H4 — `BuildingType.+0x1701` (`InvisibleInGame`) gates | **NOT APPLICABLE** | CABHUT doesn't set `InvisibleInGame=yes`; this flag does exist as a C4-plant gate but it doesn't apply here |
| H5 — `BridgeRepairHut=yes` short-circuits the C4 path | **REFUTED** | The C4-plant block at `What_Action_OnObject` doesn't consult `+0x16b6` (only the engineer block does); CABHUT passes the C4 block normally |
| H6 — No gate exists; the Rust port symptom is a port bug | **SUPPORTED** | All other hypotheses refuted; gamemd's design clearly supports C4-on-CABHUT (the hut-survives-bridge-collapses behavior is the entire point of the BridgeRepairHut branch in `BuildingClass::Update`) |

---

## 10. Implications for the Rust port

The `#[ignore]` test
`c4_on_cabhut_destroys_bridge_when_upstream_immune_lifted` at
[src/sim/world/world_orders_bridge_repair_tests.rs:265](../../ra2-rust-game/src/sim/world/world_orders_bridge_repair_tests.rs#L265)
should be re-titled and lit up: **there is no "upstream Immune gate to
lift" because gamemd doesn't have one**. The expected vanilla-YR
behavior the test should assert is:

1. SEAL/Tanya clicks CABHUT with C4 cursor → mission `Sabotage` is
   assigned (or whatever the Rust analog is).
2. Infantry walks to CABHUT.
3. On arrival, C4 is attached (assuming no IronCurtain on the hut and
   no prior C4 plant in cooldown).
4. After C4Delay, the bridge collapses; the hut survives at full HP.

If the Rust port's `apply_c4_damage_to_building` is what the user
believed was "no-op'ing on Immune," that's not the gamemd model. The
correct model is: **the bridge collapse cascade should fire WITHOUT
applying damage to the hut**, exactly as bridge report §14.2 describes.

The "right-click does nothing observable" symptom likely traces to one
of:

- The Rust port's cursor/hover code returning no valid action for
  CABHUT (its cursor-resolution may be checking `Immune` defensively
  where gamemd does not).
- The Rust port's order dispatch refusing to schedule a C4 plant for
  `Immune=yes` targets.
- The Rust port's `apply_c4_damage_to_building` checking `Immune` and
  refusing to consume the C4 or trigger the bridge cascade.

None of those would match gamemd. Suggested next investigation in the
Rust port codebase (separate from this RE doc):
- Find the cursor-resolution path for Tanya/SEAL on a hovered building
  and verify it does **not** consult `obj.immune`.
- Find `Command::PlantC4` dispatch and verify Immune is not checked.
- Find `apply_c4_damage_to_building` and verify the BridgeRepairHut
  branch fires before any Immune-gated damage application.

---

## 11. Confidence summary and open questions

**HIGH confidence (verified by binary + INI):**
- `Immune` is at `ObjectTypeClass + 0x233` (bridge report §15.1's
  +0xC4D claim is **wrong** and should be corrected).
- The Immune-downgrade gate at `0x51F171` (and analogues in Aircraft/
  Unit `What_Action_OnObject`) only converts `ACTION_ATTACK` → `ACTION_
  NOMOVE`; it does not affect action `0x10`.
- CABHUT defaults to `CanC4 = true` (constructor at `0x45E063`).
- `What_Action_OnObject` returns `0x10` for Tanya/SEAL on CABHUT.
- `PerCellProcess` Mission_Sabotage branch does not consult `Immune`.
- `BuildingClass::Update` C4-timer-expiry routes CABHUT through the
  BridgeRepairHut branch (no damage to hut).

**MEDIUM confidence:**
- The identity of `vtable[0x184]` as "current mission getter" — strong
  inference but not yet verified by `read_memory` on the BuildingClass
  vtable at slot 0x184. This is a useful follow-up if anyone needs to
  cite it definitively.
- The action enum mapping (`0x10` = SABOTAGE cursor, `0x47` = some
  other demolish action, etc.) — inferred from context; the canonical
  enum is not yet extracted from the binary.

**Open questions:**
- What is `BuildingType.+0xccc` (the engineer-block prerequisite flag)?
  Probably `Capturable` or a Capturable-superset; not investigated
  because not load-bearing for this scan.
- What does `WeaponType.+0x139` vs `WeaponType.+0x13a` distinguish
  (returns 0x40 vs 0x47 in the alternate weapon-resolution block of
  `What_Action_OnObject`)? Possibly demo-truck vs Tanya/SEAL split.
  Not investigated; not load-bearing here because the InfantryType.C4
  block returns first and bypasses the weapon-resolution block for
  Tanya/SEAL.
- Has the "Tanya C4 on CABHUT" behavior actually been observed in
  vanilla gamemd? This investigation is static-analysis-only. A
  10-minute in-game playtest with a custom map placing Tanya next to
  a CABHUT would close the loop empirically; the static-analysis
  prediction is "yes, it works, bridge falls, hut stays."

---

## 12. Sources

- `gamemd.exe` (RA2 v1.001 / YR), Ghidra MCP — live decompile +
  `read_memory` verification throughout
- [BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md](BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md)
  §3.1, §3.2, §14, §15
- [ini/rulesmd.ini](../../ra2-rust-game/ini/rulesmd.ini) — `[CABHUT]`,
  `[GHOST]`, `[TANY]`, `[Sapper]`
- [src/sim/world/world_orders_bridge_repair_tests.rs](../../ra2-rust-game/src/sim/world/world_orders_bridge_repair_tests.rs) —
  the ignored test that should be re-titled
- [docs/plans/2026-05-12-bridge-repair-and-hut-death-plan.md](../../../plans/2026-05-12-bridge-repair-and-hut-death-plan.md) —
  downstream cascade plan (now landed)

### Key function addresses verified

| Address | Function | Role in this investigation |
|---------|----------|---------------------------|
| `0x51E3B0` | `InfantryClass::What_Action_OnObject` | Cursor resolution; contains both the C4-plant block (returns 0x10) and the Immune-downgrade tail (0x51F171) |
| `0x51F171` | `What_Action_OnObject` Immune-downgrade site | Verified by `read_memory` of the byte pattern `8a 88 33 02 00 00 84 c9 74 05 bd 02 00 00 00` |
| `0x417F5B` | `AircraftClass::What_Action` Immune-downgrade analogue | Same byte pattern, verified by `read_memory` |
| `0x740492` | `UnitClass::What_Action_OnObject` Immune-downgrade analogue | Same byte pattern, verified by `read_memory` |
| `0x6FFEC0` | `TechnoClass::What_Action_OnObject` | Upstream of FootClass/InfantryClass overrides; consults `LegalTarget (+0x231)` but not `Immune` |
| `0x4DDED0` | `FootClass::What_Action_OnObject` | Thin wrapper around TechnoClass with shroud handling |
| `0x519630` | `InfantryClass::PerCellProcess` | Mission_Sabotage branch verified to not consult Immune |
| `0x43FB20` | `BuildingClass::Update` | C4-timer-expiry; routes to BridgeRepairHut branch (per §14.2) |
| `0x459EE0` | `BuildingClass::GetType` (vtable[0x88]) | Verified by `read_memory(0x7E3EBC + 0x88)`; decompile confirms `return *(this + 0x520)` |
| `0x45E063` | `BuildingTypeClass::Constructor` site initializing `CanC4=1` | Verified by `read_memory(0x45E060)` returning `c6 86 77 15 00 00 01` (`MOV byte ptr [ESI+0x1577], 1`) |
| `0x005F92D0` | `ObjectTypeClass::ReadINI` (function entry) | Verified via `get_function_by_address 0x005F92D0`; Immune read at body offset `0x5F94F4` (PUSH "Immune\0" @ `0x832B70`) and store `MOV [EBX+0x233], AL` @ `0x5F9510` (`read_memory 0x5F9510` = `88 83 33 02 00 00`) |
| `0x460E8D` | `BuildingTypeClass::ReadINI` BridgeRepairHut read | Verified by xref from string "BridgeRepairHut\0" at `0x81A898`, followed by `MOV [EBP+0x16b6], AL` |
| `0x524571` | `InfantryTypeClass::ReadINI` Engineer read | Verified by xref from string "Engineer\0" at `0x82596C`, followed by `MOV [ESI+0xec3], AL` |
| `0x524545` | `InfantryTypeClass::ReadINI` C4 read | Verified by xref through PUSH "C4\0" at `0x825978`, with `MOV [ESI+0xec2], AL` storing the result |
