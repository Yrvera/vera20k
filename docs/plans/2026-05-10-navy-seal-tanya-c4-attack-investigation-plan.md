# Navy SEAL / Tanya C4 Demolition Attack — Investigation Plan

> **For Claude:** This plan scopes a `/re-investigate` pass on the SEAL/Tanya C4
> mechanic. Execute by running `/re-investigate navy-seal-tanya-c4-attack` with
> this plan loaded as context. The mechanic is tightly bounded — single-session
> execution recommended.

**Topic:** Navy SEAL and Tanya C4 demolition attack (the instant/delayed
building-destruction infantry attack), gated by `CanC4=yes` on InfantryTypeClass
and the `Sapper` secondary weapon.
**Scope Size:** Medium — ~22 functions, ~14 INI keys, 1 file format change (none).
**Est. Effort:** ~5-7 hours of `/re-investigate` work
- Phase 1 Core: ~2-3h (FULL on 7 functions: parser → firing dispatch → sequence)
- Phase 2 Depth: ~2-3h (FULL/MEDIUM on ~9 callees: detonation, building gate, weapon flag)
- Phase 3 Edges: ~1-2h (LIGHT on ~6 callers/integration: Mission_Attack, AI, cloak, cursor)

**Prior Research:**
- HIGH-confidence (out of scope here, used as foundation):
  - `BOMB_CLASS_GHIDRA_REPORT.md` — Crazy Ivan IvanBomb (timed-fuse mechanic). Different from C4. Confirmed during scoping.
  - `INFANTRYCLASS_GHIDRA_REPORT.md` — vtable, sequences, fear system. Useful for cross-ref of `0x005206b0` Fire_At_Target dispatch but does NOT cover the C4 firing path.
  - `WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md` — WeaponTypeClass struct. Useful for `IsAttackBldgsOnly` flag at `+0x142` and `DecloakToFire` parsing.
  - `BUILDING_DAMAGE_DESTRUCTION_GHIDRA_REPORT.md` — Mentions `C4Warhead` (Rules+0xFA8) applied to docked units; does NOT cover SEAL/Tanya delivery.
  - `ENGINEER_CAPTURE_GHIDRA_REPORT.md` — Engineer Mission_Capture (`0x005202F0`). Similar special infantry-vs-building mechanic, useful as a parallel example. Not C4.
  - `FIRE_AT_PIPELINE_GHIDRA_REPORT.md` — `TechnoClass__Fire_At @ 0x006FDD50`. Covers generic fire dispatch but NOT the C4 sub-branch.
  - `CLOAKING_INTERACTIONS_REPORT.md` — SEAL stealth and DecloakToFire context.
  - `GI_GHIDRA_REPORT.md` — mentions Assaulter flag (+0xEB5) and IsHero (+0xEBE) without expanding C4.

- **Confirmed gaps** (in scope):
  - **No** dedicated SEAL/Tanya C4 attack report
  - **No** doc tracing the firing path: `Sapper` weapon → InfantryClass anim sequence → detonation function → building damage
  - **No** doc pinning the `CanC4` field offset on InfantryTypeClass
  - **No** doc on the `0x1B/0x1E/0x28/0x29` action-sequence cases (the "kneel and plant" animation)
  - **No** doc on `field_0x1bb` (InfantryClass "currently planting C4" state) or `field_0x68e` (FootClass delayed-fire flag)
  - **No** doc on per-building `CanC4=no` immunity check (Defense Bureau, Tesla Reactor, Refinery, Power Plant)
  - **No** doc on whether C4 uses BombClass internally or has its own delay machinery
  - **No** doc on `Sapper` weapon's `IsAttackBldgsOnly` semantics
  - **No** doc on `FakeC4` (Chrono Commando) and how it differs

**Expected Output:** research document at
`docs/research/SEAL_TANYA_C4_DEMOLITION_GHIDRA_REPORT.md`

**Next Pipeline Step:** `/brainstorm` then `/write-plan` for implementation —
Rust currently has no infantry→building C4 demolition path; the warhead is parsed
but only used for bridge-collapse ground kills. After this investigation, the
user decides whether to design the Rust feature.

---

## 1. Goal

When this investigation finishes, the report must answer:

1. **What is the complete firing chain** when SEAL or Tanya attacks a building?
   From cursor / order issuance → `Mission_Attack` → weapon select → fire
   dispatch → animation sequence → detonation → building damage → cleanup.
2. **Is C4 timer-based, instant, or fuse-based?** `C4Delay=0.03` minutes
   (~1.8 seconds in INI scoping) suggests delayed detonation. Does it use
   `BombClass` (the IvanBomb timer infrastructure) or a separate delay mechanism?
   What state lives on the InfantryClass during the delay (`field_0x1bb`)?
3. **What gates the attack?** `CanC4=yes` on attacker, `CanC4=no` on certain
   target buildings — what is the exact check ordering, what offsets, and what
   is the failure mode (cursor change, error, silent ignore)?
4. **What constants determine the damage?** Sapper weapon `Damage=2500` with
   `Warhead=Mechanical` (100% vs buildings) — is the explosion the Sapper's
   own warhead, or does it switch to the global `C4Warhead=Super` (absolute
   damage)? Or both in sequence?
5. **What is the Rust parity status?** With the binary's behavior fully traced,
   compare to the (currently absent) Rust implementation; flag every divergence
   for follow-up.

## 2. Prior Research Inventory

| Report | Scope | Confidence | Known Gaps for THIS topic |
|--------|-------|------------|------------|
| `BOMB_CLASS_GHIDRA_REPORT.md` | IvanBomb (timed bomb-on-Techno) | HIGH | Different mechanic — C4 may or may not share `BombClass` internals; **must verify** |
| `INFANTRYCLASS_GHIDRA_REPORT.md` | InfantryClass AI / sequences / fear | HIGH | No C4 firing/sequence cases documented |
| `WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md` | WeaponType struct | HIGH | `IsAttackBldgsOnly` semantics + DecloakToFire-on-Sapper not traced into firing |
| `BUILDING_DAMAGE_DESTRUCTION_GHIDRA_REPORT.md` | Building death | HIGH | `C4Warhead` referenced for docked-unit kill only |
| `ENGINEER_CAPTURE_GHIDRA_REPORT.md` | Engineer Mission_Capture | HIGH | Parallel example; not C4 |
| `FIRE_AT_PIPELINE_GHIDRA_REPORT.md` | `TechnoClass__Fire_At @ 0x006FDD50` | HIGH | No C4 sub-branch |
| `CLOAKING_INTERACTIONS_REPORT.md` | Cloak / DecloakToFire | HIGH | SEAL-specific decloak interaction with C4 not documented |
| `GI_GHIDRA_REPORT.md` | GI dossier | HIGH | Mentions `Assaulter +0xEB5`, `IsHero +0xEBE` but doesn't expand C4 |

**Conflicts between reports:** none surfaced during scoping.

## 3. Function Inventory

| # | Phase | Address | Current Name | Scope Reason | Depth Target | TS-Legacy Risk |
|---|-------|---------|--------------|--------------|--------------|----------------|
| 1 | 1 | `0x005240A0` | `InfantryTypeClass__ReadINI` | Reads `CanC4` boolean into one of the bool fields in the `0xEAC..0xECB` block. Pin the exact offset. | FULL — pin offset and check sibling bools (Cyborg, IsBomber, Fearless, etc.) | LOW — `CanC4` is YR-active (SEAL/Tanya use it) |
| 2 | 1 | `0x005206B0` | `InfantryClass__Fire_At_Target` | Main InfantryClass firing dispatch. `field_0x1BB` ("planting C4 active") gates anim 0x28/0x29 vs normal. | FULL | LOW |
| 3 | 1 | `0x00520AE0` | `InfantryClass__DoType_Sequencer` | Switch on action enum. **Cases `0x1B` / `0x1E` / `0x28` / `0x29` are the C4 plant/idle sequence.** Case `0x1B` calls `FUN_0070F770` (detonation candidate). | FULL — extract every case, every constant, every helper called | MEDIUM — sequence enum may have TS-legacy values |
| 4 | 1 | `FUN_0070F770` | (unnamed) | **Detonation candidate** — called from sequence case 0x1B. Likely applies the actual building damage (or attaches a BombClass). Verify and rename. | FULL | LOW |
| 5 | 1 | `0x006FDD50` | `TechnoClass__Fire_At` | Generic fire pipeline. Confirm whether C4 enters here normally or via an override (#7). Existing report covers but not C4-specific. | MEDIUM (re-read with C4 lens) | LOW |
| 6 | 1 | `0x006FC0B0` | `TechnoClass__GetFireError` | Gates firing — checks `WeaponType+0x142` (`IsAttackBldgsOnly`) and target's `+0x5EF` flag (bombable). **Where `CanC4=no` is enforced on the building side.** | FULL | LOW |
| 7 | 1 | `0x0051DF70` | `InfantryClass__Fire_At_Override` | Existing reports document Fraidycat fear-trigger only. Verify whether C4 also routes through this override. | FULL — every branch | LOW |
| 8 | 2 | `0x004DE770` | `TechnoClass__Fire` | Early entry to firing pipeline; suspect C4 enters here. | MEDIUM | LOW |
| 9 | 2 | `0x004DE760` | `TechnoClass__WeaponFire` | Twin entry; verify split between #5 and #8/9. | LIGHT | LOW |
| 10 | 2 | `0x00521B20` | `InfantryClass__Clear_Doing_Action` | Clears `field_0x1BB` (C4 active flag). Confirms when the planting state ends. | MEDIUM | LOW |
| 11 | 2 | `0x004D4DC0` | `FootClass__Mission_Attack` | Mission state that drives unit to fire. `field_0x68E` ("delayed fire") triggers `vtable+0x3C8` — likely the C4-plant trigger. | MEDIUM | LOW |
| 12 | 2 | `0x00772121` | `WeaponTypeClass__ReadINI` | Reads `IsAttackBldgsOnly` (`+0x142`) and `DecloakToFire` (`+0x133` or similar) used by Sapper. | MEDIUM | MEDIUM — `IsAttackBldgsOnly` may be TS-era; check shipping non-Sapper weapons that set it |
| 13 | 2 | `0x0051E3B0` | `InfantryClass__What_Action_OnObject` | Cursor / target-pick. **The `SabotageCursor=yes` on Sapper changes cursor here for valid C4 targets.** | FULL | LOW |
| 14 | 2 | `0x0066C31F` (in `RulesClass__ReadCombatDamage`) | (inline read) | Reads `C4Warhead=Super` global into `Rules+0xFA8`. | LIGHT — confirm offset | LOW |
| 15 | 2 | `0x0066CD9A` (same fn) | (inline read) | Reads `C4Delay=0.03` global. **Pin the offset and confirm units (minutes? frames? game seconds?).** | LIGHT — confirm offset and unit conversion (typically `frame = minutes * 900`) | LOW |
| 16 | 2 | (TBD) | BuildingTypeClass `CanC4` parser (in `BuildingTypeClass__ReadINI`) | Sets the per-building immunity flag (Defense Bureau, Tesla Reactor, Refinery, Power Plant set `CanC4=no`). Pin the offset. | MEDIUM | LOW |
| 17 | 3 | `0x0051BAB0` | `InfantryClass__AI` | Calls `Fire_At_Target`. Confirm tick ordering: where does the C4 plant tick during `advance_tick`? | LIGHT | LOW |
| 18 | 3 | `0x0084951C` (string addr) | `DecloakToFire` xref | Trace from string into the cloak-state check during firing. Cloak-on-fire is generic, not C4-specific. | LIGHT | LOW |
| 19 | 3 | `0x00521C10` | `InfantryClass__Panic_SetFear300` | SEAL/Tanya may have fear immunity adjacent to C4 logic; sanity-check no cross-coupling. | LIGHT | LOW |
| 20 | 3 | (TBD via xref of `[Sapper]` string) | Sapper weapon definition consumer | Find the `Sapper` string in binary, follow to consumer (likely just the weapon-name lookup at scenario load). | LIGHT | LOW |
| 21 | 3 | (TBD) | `FakeC4` consumer (Chrono Commando) | Verify whether `FakeC4` warhead/weapon goes through the same path as real Sapper but with different damage, or a separate code path entirely. | LIGHT | **HIGH — possible TS legacy.** `FakeC4` may be a Yuri's-Revenge-only Chrono-Commando feature, OR a TS holdover. Confirm. |
| 22 | 3 | (TBD) | Anim spawn for C4 plant | Identify whether the planting animation comes from the InfantryType sequence list (offset `+0xE54`/`+0xE60`) or from a fixed `[NADEPT_C4]`-style INI animation. | LIGHT | LOW |

**Phase 1 checkpoint rule:** the executor must pause after functions #1-#7 and
write a "what we know about the firing chain so far" summary before continuing
to Phase 2. If Phase 1 reveals the chain branches differently than mapped
(e.g., C4 doesn't go through `InfantryClass__Fire_At_Target` at all), the plan
is revised.

**Sizing check:** 22 functions in the inventory, fits the "Medium" band. Phase 1
(7 functions) covers the entry chain end-to-end. Phase 2 (9 functions) fills
helpers, gates, and global-rule reads. Phase 3 (6 functions, mostly LIGHT) is
context and edge cases.

## 4. Detail Checklist

The executor must extract these specifics during research:

### Magic numbers and constants
- **Sapper damage** = 2500 — confirm the binary actually applies this raw or scales it
- **C4Delay** = 0.03 (minutes per INI). Convert to frames: `0.03 × 900 = 27 frames` if standard. Confirm.
- **Mechanical warhead Verses** = `0%,0%,0%,100%,100%,100%,0%,0%,0%,100%,100%` (against armor types) — confirm parser reads in the right order
- **`C4Warhead=Super`** global default — verify
- **Sequence enum cases** `0x1B`, `0x1E`, `0x28`, `0x29` — pin every case's behavior
- **InfantryClass** `field_0x1BB` (C4 active) — pin offset and check it's not aliased
- **FootClass** `field_0x68E` (delayed-fire) — pin offset
- **WeaponType** `+0x142` (`IsAttackBldgsOnly`) — confirm
- **Target building** `+0x5EF` (bombable flag) — pin and verify it's the gate

### Bit flags and masks
- `IsAttackBldgsOnly` — single bit, confirm bit position within byte at `+0x142`
- `DecloakToFire` — same, on WeaponType
- Building's `CanC4` immunity bool — single bit, confirm offset (likely in BuildingTypeClass bool block)
- InfantryType `CanC4` bool — pin offset (in `0xEAC..0xECB` range)
- Verify whether `IsBomber` / `IsHero` / `Assaulter` / `Cyborg` flags interact with C4

### State machine states / sequence enum
- The InfantryClass action enum (used in `DoType_Sequencer`):
  - Case `0x1B` — DETONATE (calls `FUN_0070F770`)
  - Case `0x1E` — IDLE during plant (?)
  - Case `0x28` — KNEEL/PLANT pose
  - Case `0x29` — adjacent pose
  - Confirm transitions: `1B → 1E → 28 → 29 → done` or some subset
- Door-state of target building during plant: does plant abort if building is destroyed mid-plant?

### INI keys to verify
- See Section 5 (full table). Highlights: `CanC4` (InfantryType + BuildingType), `C4Warhead`, `C4Delay`, `Sapper.Damage/Warhead/Range`, `Sapper.IsAttackBldgsOnly`, `[Mechanical].Verses`.

### Struct offsets to extract
- **InfantryTypeClass** (param_1 type: `int *` per existing READINI_FIELD_MAPS — verify, multiply by 4!): `CanC4` offset (one of `0xEAC..0xECB`)
- **InfantryClass**: `field_0x1BB` (C4-active state)
- **FootClass**: `field_0x68E` (delayed-fire trigger)
- **WeaponTypeClass**: `+0x142` IsAttackBldgsOnly, `+0x144` (next bool, suspected `IsTemporal`?)
- **BuildingTypeClass**: `CanC4` bool offset, `+0x5EF` (bombable flag — verify name)
- **RulesClass**: `+0xFA8` (C4Warhead pointer), `C4Delay` offset (TBD)
- **BombClass** (only if confirmed used by C4): see `BOMB_CLASS_GHIDRA_REPORT.md` — 0x5C bytes

### Clamps, rounding, off-by-ones
- C4Delay frame conversion: minutes → frames (`× 900`?). Verify rounding direction (truncate vs round-up).
- Damage clamp on Sapper-vs-non-building (Mechanical.Verses says 0% vs infantry — confirm zero damage is short-circuited or applied as 0)
- Range check on Sapper (1.5 cells) — fixed-point or float? Diagonal vs orthogonal handling?

### Edge cases to test
- **Plant interrupted by death**: SEAL killed mid-plant — does C4 still detonate? Does `field_0x1BB` get cleared? Does building take damage?
- **Plant on building that's destroyed mid-plant**: target dies — what happens to the planter? Does animation play out or abort?
- **Plant on `CanC4=no` building**: cursor blocked, or is the order accepted then silently fails?
- **Plant on garrisoned building**: does C4 still work? Does it kill the garrison too? (cross-ref `Assaulter` flag at `+0xEB5`)
- **Plant on chronosphered building**: target frozen — does C4 work?
- **Plant under Iron Curtain**: target invulnerable — does C4 bypass? (Likely no but verify)
- **Multiple SEALs on same building simultaneously**: do their C4s stack? Each places its own?
- **Plant on a captured-but-not-yet-owned building** (during engineer capture animation)
- **Plant on a building that's selling** (sell animation playing)
- **Cloaked SEAL plants**: does the plant decloak? At what frame?
- **C4 vs walls**: does `Mechanical` warhead destroy walls, or only buildings?
- **Allied C4 on Allied building**: friendly-fire prevention, or works?

### Timing / ordering
- Where in `advance_tick` does the C4 plant tick (turret fire vs combat vs movement)?
- Does the detonation happen at end-of-frame or inline mid-tick?
- Does the building damage hit before or after building repair tick (could a building repair tick mid-detonation save it)?

### TS-legacy flags
- See Section 7

### Vtable dispatches
- `vtable+0x3C8` called from `Mission_Attack` when `field_0x68E` is set — resolve the concrete method (likely InfantryClass override)
- Sequence-related vtable on InfantryTypeClass — used in `DoType_Sequencer` to resolve which animation list

## 5. INI Keys in Scope

| Key | Section | Default | Suspected Purpose | Currently Parsed in Rust? |
|-----|---------|---------|-------------------|----------------------------|
| `CanC4=` | per InfantryType (`[SEAL]`, `[TANY]`) | no | Master flag enabling C4 attack on this infantry | **No** (only `engineer` and `sabotage_cursor` flags are parsed; `CanC4` not surfaced) |
| `CanC4=` | per BuildingType (e.g., `[NADBRC]`, `[NATSLA]`, `[NAREFN]`, `[NAPOWR]`) | yes | Per-building immunity to C4 | **No** |
| `Primary=MP5` | `[SEAL]` | n/a | SEAL primary anti-infantry weapon | Yes (generic Primary parsing) |
| `Secondary=Sapper` | `[SEAL]`, `[TANY]` | n/a | The C4 weapon | Yes (generic Secondary parsing) |
| `[Sapper]` | weapons section | n/a | C4 plant weapon: `Damage=2500, ROF=100, Range=1.5, Warhead=Mechanical` | Generic weapon parser exists |
| `[Sapper] IsAttackBldgsOnly=` | weapon flag | yes (on Sapper) | Restricts firing to buildings | **No** |
| `[Sapper] SabotageCursor=` | weapon flag | yes (on Sapper) | Changes cursor to "sabotage" UI | Field exists in `object_type.rs:546` (`sabotage_cursor`) but no logic |
| `[Sapper] Warhead=Mechanical` | weapon | n/a | Damages buildings only | Yes (warhead reference parsing) |
| `[Mechanical] Verses=0%,0%,0%,100%,100%,100%,...` | warhead | n/a | Anti-armor multiplier table | Yes (warhead parser) |
| `C4Warhead=Super` | `[CombatDamage]` | `Super` | Global override warhead applied at detonation (absolute damage) | Yes — parsed into `bridge_warheads.rs:23-50` (defaults to `"Super"`); only used for bridge collapse currently |
| `C4Delay=0.03` | `[CombatDamage]` | 0.03 (minutes ≈ 27 frames) | Time between plant and detonation | **No** |
| `[Super] Verses=100%,...` | warhead | n/a | The "absolute" warhead used at C4 detonation | Yes (warhead parser) |
| `[FakeC4]` | weapon | n/a | Chrono Commando sapper variant | **No / Unknown** — verify if YR-active |
| `[FakeC4WH]` | warhead | n/a | Fake C4 warhead | **No / Unknown** |
| `[NADEPT_C4]` | `artmd.ini` | n/a | Soviet Repair Bay C4-damaged appearance | **Unknown** |

## 6. Caller & Integration Map

| Caller | Calls Into | When Invoked | Should Executor Decompile? |
|--------|------------|--------------|----------------------------|
| `0x0051BAB0` `InfantryClass__AI` | `InfantryClass__Fire_At_Target` (#2) | Per-tick during `advance_tick` foot phase | LIGHT — confirm tick ordering only |
| `0x004D4DC0` `FootClass__Mission_Attack` | `Fire_At` chain | When Mission is `MISSION_ATTACK` | YES — `field_0x68E` is the C4 trigger gate |
| Player order issuance (`OrderClass`) | `Mission_Attack` | On right-click target building | LIGHT — order issuance is generic |
| AI script (TaskForce attack) | `Mission_Attack` | AI controlling SEAL | NO — AI logic out of scope |
| `What_Action_OnObject` (#13) | (none — UI cursor only) | On hover with SEAL selected | YES — the cursor decision is the gate visible to the player |

**Where this hooks into Rust today:**
- InfantryType parser: [src/rules/object_type.rs:417-648](src/rules/object_type.rs#L417-L648). Has `engineer` (line 421), `sabotage_cursor` (line 546). **Missing: `CanC4` field.**
- Warhead parser: [src/rules/warhead_type.rs:29-180](src/rules/warhead_type.rs#L29-L180). Has `IvanBomb=` (line 162) flag.
- C4Warhead reference: [src/rules/bridge_warheads.rs:23-50](src/rules/bridge_warheads.rs#L23-L50). Stores the section name; resolved in `ruleset.rs:1182-1445`.
- Combat: [src/sim/combat/mod.rs](src/sim/combat/mod.rs) and submodules. **No C4 branch.**
- C4Warhead is currently used only for bridge-collapse ground kill cascade ([src/sim/bridge_state/mod.rs:248](src/sim/bridge_state/mod.rs#L248)).

**Callers explicitly NOT investigated (with justification):**
- Player order issuance — generic, well-understood, not C4-specific
- AI script TaskForce attack — out of scope, AI work is on hold per memory
- `BombClass` infrastructure — already covered HIGH-confidence in `BOMB_CLASS_GHIDRA_REPORT.md`. Only re-touch if Phase 1 confirms C4 actually uses it

## 7. TS-Legacy Risk Register

- **`CanC4` is YR-active** — confirmed used by SEAL and Tanya in shipping
  rulesmd.ini. Not TS legacy. **No risk.**
- **`C4Delay` and `C4Warhead` globals** — present in YR rulesmd.ini's
  `[CombatDamage]` block. **Active.**
- **`IsAttackBldgsOnly` (WeaponType+0x142)** — only Sapper sets it in vanilla
  YR. Verify no other shipping weapon has it set as a "TS leftover" that
  would trigger the same code path.
- **`FakeC4` weapon and `FakeC4WH` warhead** — Chrono Commando-only. Chrono
  Commando is a YR campaign-only unit (not skirmish). **Verify whether the
  FakeC4 code path is reachable in standard YR skirmish — likely not, but
  if active, document it; if dormant, mark as TS-adjacent dead code.**
- **`[NADEPT_C4]` art entry** — Soviet Repair Bay's C4-damaged appearance.
  Verify whether this anim plays in standard YR (not TS) when a Repair Bay
  is C4'd.
- **`Assaulter +0xEB5`** — flag for "can clear garrisoned building" (SEAL has
  this per existing GI report). Cross-check: does `Assaulter` interact with
  the C4 path or is it independent?
- **`IsHero +0xEBE`** — flag for Tanya/Yuri/Boris (per existing GI report).
  Cross-check: does `IsHero` modify C4 behavior, or is `CanC4` the only gate?
  **Risk:** if `IsHero` is the actual gate and `CanC4` is just metadata, the
  scoping conclusion would be wrong.
- **Mission/trigger script types referencing C4** — not investigated in
  scoping; flag as "do not implement TS-only mission scripts that reference
  C4 unless verified active".
- **TS-era SEAL-equivalent units** — Tiberian Sun had no SEAL. No TS-side
  callers should exist for this code. If any are found, treat as suspicious.

## 8. Current Rust Implementation Surface

The Rust side has **no infantry-vs-building C4 demolition path**. Per scoping:

| System | File(s) | Status |
|--------|---------|--------|
| InfantryType parser | [src/rules/object_type.rs:417-648](src/rules/object_type.rs#L417-L648) | Implemented for generic fields (Strength, Speed, Cost, Primary, Secondary). **`CanC4` not parsed.** |
| Warhead parser | [src/rules/warhead_type.rs:29-180](src/rules/warhead_type.rs#L29-L180) | Implemented including `IvanBomb=` flag |
| C4Warhead reference | [src/rules/bridge_warheads.rs:23-50](src/rules/bridge_warheads.rs#L23-L50) | Section name stored, default `"Super"`, resolved at world init |
| C4 attack logic | (none) | **Missing — no infantry→building C4 demolition path** |
| Engineer capture | [src/sim/world/world_orders.rs](src/sim/world/world_orders.rs), [src/sim/production/](src/sim/production/) | Partial — `engineer` flag parsed; capture path incomplete |
| IvanBomb attach/detonate | (warhead parsed only) | **Missing — no BombClass-equivalent in Rust** |
| Building demolition | [src/sim/bridge_state/](src/sim/bridge_state/) | Bridge collapse uses C4Warhead for ground-kill cascade |
| Combat infrastructure | [src/sim/combat/](src/sim/combat/) | Generic weapon dispatch only. **No special-case branches** for engineer / Tanya / SEAL / Ivan |

**Implication:** when this investigation produces its report, the natural
follow-up is `/brainstorm` on whether to design a generic
"infantry-special-attack-on-building" module, or a SEAL/Tanya-specific path,
or piggyback on a future BombClass implementation. **Do not pre-decide that
during research.** Report the binary's behavior; design comes later.

## 9. Deferred Open Questions

The scoping pass surfaced these but couldn't answer them. The executor must
explicitly resolve each or mark as unresolved:

1. **Does C4 use `BombClass` under the hood?** `C4Delay=0.03` minutes (~27
   frames) is suspiciously similar to a BombClass fuse, but the Sapper
   weapon's damage is `2500` with `Mechanical` warhead — that doesn't look
   like a BombClass attach. Possibilities:
   - C4 = inline delay (state on InfantryClass `field_0x1BB`), no BombClass
   - C4 = BombClass attach with very short fuse (using `C4Warhead=Super`
     instead of `IvanWarhead`)
   - Two-stage: Sapper damage applied immediately + delayed Super warhead
     applied after `C4Delay`
   **Decide via decompilation of `FUN_0070F770` (#4) and trace from there.**

2. **What is the `field_0x1BB` semantics?** Is it a tick countdown, a boolean,
   or a target-building pointer?

3. **What is the exact `CanC4` field offset on InfantryTypeClass?** The
   `0xEAC..0xECB` range has many bool fields; pin the order from
   `InfantryTypeClass__ReadINI` (#1) by listing every ReadBool call in
   sequence. Cross-check with existing READINI_FIELD_MAPS doc.

4. **What is the exact `CanC4` field offset on BuildingTypeClass?** Find the
   parser that reads it from `[NADBRC]`/`[NATSLA]`/etc. (the buildings that
   set it to `no`) and pin the offset. Probably in
   `BuildingTypeClass__ReadINI`.

5. **What is `+0x5EF` on the building (the "bombable" flag mentioned in
   `TechnoClass__GetFireError`)?** Is this `CanC4` directly, or another
   flag like "can be sabotaged" / "is constructed"?

6. **Sapper (`Mechanical`) damage vs C4Warhead (`Super`) damage — which is
   actually applied?** The INI suggests Sapper applies `Damage=2500` with
   Mechanical warhead. But the global `C4Warhead=Super` exists. Possibilities:
   - Sapper applies the immediate damage; C4Warhead is unused for SEAL
     (only used for bridge-collapse cascade as in Rust today)
   - Sapper triggers the timer; on detonation, C4Warhead is applied with
     some damage value (2500? Rules-default?)
   - Both apply (Sapper as a "plant impact" and C4Warhead as the "boom")
   **Resolve from `FUN_0070F770` decompilation.**

7. **Is the C4 plant interruptible?** What happens if the planting SEAL is
   killed mid-plant? Mid-detonation? Does `field_0x1BB` get cleared? Does
   the building still take damage?

8. **Cloaked SEAL → C4 → decloak**: at what exact frame does decloak happen?
   On order? On plant start? On detonation? Does `DecloakToFire` on the
   Sapper weapon route through the same path as a non-C4 weapon's decloak?

9. **Multiple SEALs targeting the same building**: are simultaneous plants
   stackable (each does its damage), or does the second SEAL queue / fail?

10. **Allied-vs-Allied C4 (friendly fire)**: gates this at order issuance
    (cursor blocks), at firing (GetFireError), or at damage application?

11. **`FakeC4` (Chrono Commando)**: does it follow the same code path with
    a different warhead, or a separate path? Is it active in YR skirmish?

12. **Anim source**: is the planting animation triggered from
    InfantryTypeClass sequence list (offset `+0xE54`/`+0xE60`), or from a
    fixed `[NADEPT_C4]`-style art entry? Check both `DoType_Sequencer` (#3)
    and any AnimType lookup near the detonation function.

## 10. Execution Strategy

**Recommended:** **Single-session `/re-investigate`.**

This scope is well-bounded (22 functions, ~14 INI keys, 1 self-contained
mechanic). A single deep dive should cover it without subagent batching.
Phase 1 → Phase 2 → Phase 3 in one pass.

**Checkpoint discipline:**
- After Phase 1 (functions #1-#7), the executor MUST write a "what we know
  about the firing chain so far" summary before starting Phase 2. If Phase 1
  reveals the chain is fundamentally different (e.g., C4 doesn't go through
  `Fire_At_Target`), revise this plan before continuing.
- After Phase 2 (functions #8-#16), a second checkpoint: "delay mechanism
  resolved? BombClass involved or not?". The answer to deferred Q#1 must be
  in this checkpoint.

**Subagent batching not recommended** — the call chain is sequential, each
function's findings inform the next. Parallel research would lose the chain
context.

## 11. Success Criteria

The executed research document must:

- [ ] Answer every question in Section 1 with a clear yes/no/specific value.
- [ ] Include every function from Section 3, OR explicitly justify omission
      with "covered in <existing report>" or "not reachable in YR skirmish".
- [ ] Resolve every deferred question from Section 9, OR re-document each
      as unresolved with what was tried.
- [ ] State **"Active in YR: Yes / No / Conditional (which flag)"** for every
      finding.
- [ ] Cite Ghidra addresses for every HIGH-confidence claim.
- [ ] Include a **complete firing-chain narrative** start to finish:
      Order issuance → Mission_Attack → fire dispatch → animation cases
      → detonation function → damage application → cleanup.
- [ ] Pin the **exact field offsets**:
      - InfantryTypeClass `CanC4`
      - BuildingTypeClass `CanC4` (immunity)
      - InfantryClass `field_0x1BB`
      - FootClass `field_0x68E`
      - WeaponTypeClass `+0x142` IsAttackBldgsOnly
      - RulesClass C4Warhead pointer (`+0xFA8`?)
      - RulesClass C4Delay
      - Building bombable flag at `+0x5EF` (verify name)
- [ ] State the **damage application chain**: which warhead applies (Sapper's
      Mechanical, C4Warhead's Super, or both), at what point in time, with what
      damage value.
- [ ] Document the **edge cases** from the Detail Checklist § "Edge cases to
      test" — at minimum the death-during-plant and `CanC4=no` cases.
- [ ] End with a **Rust parity status** subsection naming every divergence
      from the (currently absent) Rust implementation.

## Sources

- **Ghidra addresses sampled (light scoping):**
  - `0x005240A0` (InfantryTypeClass__ReadINI), `0x005206B0` (InfantryClass__Fire_At_Target),
  - `0x00520AE0` (DoType_Sequencer), `0x0051E3B0` (What_Action_OnObject),
  - `0x004D4DC0` (FootClass__Mission_Attack),
  - `0x006FDD50` (TechnoClass__Fire_At), `0x006FC0B0` (GetFireError),
  - `0x0066C31F`/`0x0066CD9A` (RulesClass C4 reads),
  - `FUN_0070F770` (detonation candidate), `0x00772121` (WeaponTypeClass__ReadINI)

- **Docs searched:**
  - `docs/` (in-repo)
  - `docs/research/` (~330 standalone)

- **INI files checked:**
  - `ini/rules.ini`, `ini/rulesmd.ini`, `ini/art.ini`, `ini/artmd.ini`

- **Related plans:**
  - `2026-05-05-particles-c4-fire-plan.md` — particles only, no overlap with attack mechanic
  - `2026-05-04-gi-unit-complete-investigation-plan.md` — Phase 3 had open question on `+0xEC2-3` flags (possibly C4-related)
