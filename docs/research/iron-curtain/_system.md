# Iron Curtain — System Synthesis

**Synthesized:** 2026-05-24
**Source:** 16 per-symbol decode docs in this directory + `_parity.md` (32 rows).
**Status:** DRAFT — awaiting user approval.

---

## Summary

The Iron Curtain is a Soviet super weapon (Yuri's Revenge / Red Alert 2). The player clicks the IC cameo, then targets a cell on the map. Within a fixed area-effect radius, the system applies an effect to every unit and building:

- **Vehicles and buildings** become invulnerable for `IronCurtainDuration` frames (default 750 = ~50 s @ 15 fps). All damage is blocked; impacts emit gold sparks. The unit/building renders with the `IronCurtainColor` tint overlay for the duration.
- **Infantry** are **instantly killed**, regardless of duration, via a damage call using the `C4Warhead` warhead and the unit's own Strength as damage. No invulnerability state is set.
- **Organic units** (units with `Organic=yes` — Dolphins, Giant Squids) take the infantry path: instakilled by C4Warhead damage, never invulnerable.

The same code path also serves Yuri's **Force Shield** super weapon, distinguished by an `is_force_shield` flag stored on each affected unit (`TechnoClass+0x1c4`). Force-Shield'd units emit blue-white sparks on damage instead of gold.

---

## Symbol scope

Inventory:
- **9 functions:** gate check, base apply, per-class overrides (Building, Infantry), super-weapon dispatch, 3 RulesClass INI readers, SuperWeaponTypeClass type-name dispatch
- **4 structs:** TechnoClass IC state fields, BuildingClass IC state extras, RulesClass IC config, TechnoTypeClass `Organic` flag
- **2 globals:** `g_CurrentFrameCounter`, `g_RulesClass_Instance` (both shared engine infrastructure)
- **4 strings/INI keys:** `IronCurtainDuration`, `IronCurtainInvokeAnim`, `IronCurtainColor`, `"IronCurtain"` (SW type name)

Excluded by TS-filter: 0 symbols. (No TS-legacy contamination in this system — Iron Curtain is a clean YR feature.)

**Missing-from-disk decodes (phantom completions from the v1 team smoke test):**
- `fn-StartFidget-IronCurtain-Dispatch.md` — content effectively covered by `struct-TechnoTypeClass-IC-immune.md` (deflect path) + `fn-InfantryClass-IronCurtain.md` (TakeDamage chain) + `_manifest.yaml` preflight notes.
- `fn-ReadCombatDamage-IronCurtainDuration.md` — content covered by `struct-RulesClass-IC-config.md` (`+0xfe8` field).
- `fn-ReadGeneral-IronCurtainInvokeAnim.md` — content covered by `struct-RulesClass-IC-config.md` (`+0x348` field).

These phantoms do not leave gaps in the synthesis — their content is recoverable from the struct decodes.

---

## Control flow

```
Player clicks IC cameo + selects target cell
  │
  ▼
Super-weapon manager (NOT decoded in this run — out of scope)
  │  Looks up SuperWeaponTypeClass+0xb4 enum (=1 for IronCurtain).
  │  Reads SW's AreaOfEffect radius, iterates cells in range.
  ▼
For each unit/building in radius:
  │
  ▼
TechnoClass::IronCurtain vtable dispatch
  │  Slot offset 0x154 from each subclass's vtable base.
  │  Routes to BuildingClass / InfantryClass / TechnoClass per object type.
  │
  ├──→ BuildingClass__IronCurtain (0x00457c90)
  │       Check +0x6df gate → if set, reset +0x528/+0x52c/+0x530/+0x540
  │       (purpose unclear; +0x6df setter not identified — YELLOW)
  │       Then super-call TechnoClass__IronCurtain
  │
  ├──→ InfantryClass__IronCurtain (0x00522600)
  │       Load InfantryTypeClass+0xa0 (Strength) into local_4
  │       Call vtable+0x16c (TakeDamage parent) with:
  │         damage = own Strength
  │         warhead = *(RulesClass+0xfa8) = C4Warhead
  │         is_force_shield/source_house passed through
  │       Unit dies. No IC state stored. Duration ignored.
  │
  └──→ TechnoClass__StartFidget [MISNAMED] (0x004deae4)
          (Per-techno dispatch for non-infantry, non-building units.)
          1. Get TechnoTypeClass via vtable+0x84
          2. If type's +0xd97 (Organic flag) set → call vtable+0x16c TakeDamage
             with C4Warhead → unit dies. SKIP IC. (= "deflect" path.)
          3. Else: if +0x694 (chrono warp ptr) set → clear warp state, detach
          4. Set timestamps at +0x6A0/+0x6A4/+0x6A8 (apply_frame, source_house, 0)
          5. Call TechnoClass__IronCurtain to stamp +0x18c/+0x194/+0x1c4
              │
              ▼
TechnoClass::IronCurtain (0x0070e2b0) — base apply, leaf function
  Writes:
    this+0x18c = g_CurrentFrameCounter  ← IC apply frame
    this+0x190 = (stack garbage; vestigial — see fn-TechnoClass-IronCurtain doc)
    this+0x194 = duration                ← from arg (default 750)
    this+0x1a4 = 0                        ← purpose unresolved (YELLOW)
    this+0x1c4 = is_force_shield          ← 1 = Force Shield, 0 = Iron Curtain
```

After apply, every tick:
- Damage applications consult `TechnoClass__IsIronCurtainActive(this)` before deducting HP. If true → damage blocked → spark emitted (gold or blue-white per +0x1c4).
- Renderer reads `+0x1c4` and `IsIronCurtainActive` → applies `IronCurtainColor` tint to sprite.
- AI target-validity consults `IsIronCurtainActive` (out of scope here).

When `elapsed >= duration`, `IsIronCurtainActive` returns false. No explicit expire-handler is needed — the state simply becomes ignored.

---

## State machine (per-unit)

```
                 ┌──────────────────┐
                 │ IC NEVER APPLIED  │   +0x18c == -1 (sentinel)
                 │ (initial state)   │
                 └────────┬─────────┘
                          │
                          │  IC super-weapon hits this unit
                          │  TechnoClass::IronCurtain called
                          ▼
                 ┌──────────────────┐
                 │   IC ACTIVE       │   +0x18c = apply_frame
                 │ (or Force Shield) │   +0x194 = duration (e.g., 750)
                 │                   │   +0x1c4 = 0 (IC) or 1 (FS)
                 └────────┬─────────┘
                          │
                          │  every-frame: IsIronCurtainActive checks
                          │  elapsed = current - apply_frame
                          │  if elapsed >= duration → return false
                          │
                          ▼
                 ┌──────────────────┐
                 │  IC EXPIRED       │   +0x18c still set, but
                 │  (silent)         │   IsIronCurtainActive returns false
                 │                   │   No explicit cleanup
                 └────────┬─────────┘
                          │
                          │  IC super-weapon hits again
                          ▼
                       (back to IC ACTIVE)
```

**Boundary:** the active check is `elapsed < duration AND remaining > 0`. At the exact expiry frame (`elapsed == duration`), the result is false (IC ends on that frame, not the next).

**Persistence across save/load:** IC fields are NOT serialized in `TechnoClass__Save`. Reloading a saved game clears all IC state — by design.

---

## INI surface

| Key | Section | Default (stock YR) | Field | Effect |
|---|---|---|---|---|
| `IronCurtainDuration` | `[CombatDamage]` | 750 (frames @ 15fps ≈ 50s) | `RulesClass+0xfe8` | Effect duration. |
| `IronCurtainInvokeAnim` | `[General]` | `IRONBLST` | `RulesClass+0x348` (AnimTypeClass*) | Anim spawned at target on apply. |
| `IronCurtainColor` | `[AudioVisual]` | (packed RGB; default unread in this decode) | `RulesClass+0x18a8` (int) | Tint overlay color on IC'd sprites. |
| `C4Warhead` | `[CombatDamage]` | `Super` (verified via `rulesmd.ini:818`; the warhead named "Super" — confusingly the C4Warhead key's value is NOT "C4") | `RulesClass+0xfa8` (WarheadTypeClass*) | Warhead used for infantry/organic instakill. |
| `Type=IronCurtain` | `[<SuperWeapon section>]` | — | `SuperWeaponTypeClass+0xb4` (enum) | Wires the SW INI section to IC dispatch (enum value `1`). |
| `Organic=yes/no` | `[<TechnoType section>]` | no (default) | `TechnoTypeClass+0xd97` (1-byte bool) | When yes → IC instakills instead of protects. Set on Dolphin (DLPH), DNOA, DNOB, Giant Squid (SQD) in stock rulesmd.ini. |

**Sound/EVA keys** (referenced but not part of the IC apply path proper — these are SW-system level):
- `EVA_IronCurtainDetected`, `EVA_IronCurtainActivated`, `EVA_IronCurtainReady` (eva.ini)
- Anim sounds `IronCurtainReady`, `IronCurtainReadyLoop` (art.ini StartSound)
- Damage-event reports `IronCurtainBlast`, `IronCurtainDeflect` (art.ini Report on the dome anim)

---

## Observable behaviors (parity bar surface)

For each input below, the player observes the listed output. These are what the Rust port must match.

| Trigger | gamemd observable output |
|---|---|
| Fire IC at a Grizzly tank | Tank gains gold dome anim, gold tint for 750 frames, blocks all damage, emits gold sparks on hits, EVA "Iron Curtain activated" voice. |
| Fire IC at a Conscript | Conscript dies instantly (C4Warhead instakill animation). No tint, no protection. |
| Fire IC at a Dolphin or Giant Squid | Same as Conscript — instakill via C4Warhead. The unit's `Organic=yes` flag routes to the deflect path. |
| Fire IC at a unit type with `Organic=yes` (any custom INI mod) | Instakill. (Same mechanism as Dolphins/Squids.) |
| Fire IC at a building (Power Plant, etc.) | Building gains gold dome anim, gold tint, blocks all damage including SuperWeapon impacts for the duration. BuildingClass extra state at +0x6df/+0x528/etc. is reset (purpose unclear). |
| Fire Force Shield at a Grizzly | Same as IC, but blue-white sparks on damage and `+0x1c4 = 1`. (Visual tint may also differ; not decoded.) |
| Fire IC at a unit currently chrono-warping | Chrono warp is detached (warp state cleared), then IC applied normally. Unit becomes invulnerable but is no longer mid-teleport. |
| Damage applied to an IC'd unit | TechnoClass::ReceiveDamage gates on IsIronCurtainActive → damage blocked, spark color = 1 (IC) or 6 (Force Shield). |
| IC duration expires | IsIronCurtainActive starts returning false. Tint and spark behavior revert. No anim. No EVA cue (only on apply). |
| Save/load mid-IC | IC state lost. Unit appears unprotected on reload. (TechnoClass::Save does not serialize IC fields.) |

---

## Edge cases / known parity hazards

1. **Boundary frame (elapsed == duration):** IsIronCurtainActive returns false. IC ends ON the duration frame, not after.
2. **Zero duration (IronCurtainDuration=0):** IC applies but instantly expires (false on the apply frame itself).
3. **Sentinel `-1`:** `+0x18c == -1` indicates never-applied. Reload state is also undefined until first apply. Constructor does not explicitly set this field — relies on zero-init or subclass init.
4. **Reapplying IC during active IC:** simply re-stamps `+0x18c` and `+0x194`. Effect extends from re-apply frame, not adds.
5. **Force Shield + Iron Curtain interaction:** the same field `+0x1c4` stores both flags. The last-applied wins. There is no "both" state.
6. **InfantryClass duration ignored:** even passing `duration > 0` to InfantryClass::IronCurtain results in instakill — duration is consumed but unused. Rust must mirror this.
7. **Organic flag is broader than "infantry":** Dolphins and Squids are *VehicleType* / *NavyType* units but carry `Organic=yes`. The deflect path must check the flag, not the entity category.
8. **Chrono warp interaction:** A unit IC'd mid-warp loses its warp state. If Rust applies IC without detaching the warp, the unit will appear invulnerable AND warping simultaneously.
9. **+0x190 is dead-write garbage:** the `source_house` parameter passed to TechnoClass::IronCurtain is NOT stored. Don't implement a `source_house` field on the Rust state unless a downstream consumer needs it.
10. **+0x1a4 unresolved (YELLOW):** cleared on apply but no reader found in the IC system. Could be a timer-pair field for another effect. Currently INTERNAL-ONLY; upgrade to DRIFT if a reader surfaces.
11. **BuildingClass `+0x6df` gate (YELLOW):** non-zero value triggers a state-reset block on IC apply. The setter for this byte is not identified in this decode. If it fires during building production or animation completion, the gate path becomes player-observable.

---

## Parity report rollup

Full per-row report in `_parity.md`. Highest-leverage findings (by player-visibility × frequency):

### HIGH

1. **`Organic` flag deflect missing in Rust (DRIFT).** Iron Curtain on Dolphins or Giant Squids gives them invulnerability in the Rust port instead of instakilling. Fires every match on any water map where IC hits naval units. Citations: `struct-TechnoTypeClass-IC-immune.md` + Rust `iron_curtain.rs:53` (only checks `EntityCategory::Infantry`).
2. **`ImmuneToIronCurtain` / general type-flag deflect missing (DRIFT).** No type-flag immunity check in Rust dispatch. Any modded unit with the deflect flag would be invulnerable in our engine and instakilled in gamemd.
3. **Spark color on blocked hit missing (DRIFT).** gamemd emits gold sparks (IC) or blue-white sparks (Force Shield) when projectiles hit an invulnerable unit. Rust skips damage but emits no spark. Visible on every IC'd unit hit, every match.
4. **`IronCurtainColor` tint not parsed / not applied (MISSING).** Stock IC effect tints sprites gold for the duration. Rust does not parse the INI key or apply the tint. Visible on every IC'd unit and building, every match.

### MEDIUM

5. **Chrono warp detach not implemented (DRIFT).** A chrono-unit IC'd mid-warp should have its warp state cleared. Rust leaves teleport_state intact. Niche but visible on chrono units mid-warp.
6. **Infantry instakill uses C4Warhead, not generic kill (INTERNAL→potential DRIFT).** gamemd routes infantry IC kill through `TakeDamage(strength, C4Warhead)`, triggering C4-warhead-specific death animation/sound. Rust sets `health.current = 0; dying = true`. If C4 warhead has distinct death effects, players see a different death on every IC-on-infantry hit.

### INTERNAL-ONLY (informational, not parity drift)

- Save/load: gamemd doesn't serialize IC state; Rust likely doesn't either. Same observable outcome.
- Field storage layouts differ (`Option<InvulnerabilityState>` vs raw `+0x18c == -1` sentinel) — same observable result.
- Source_house received but not stored on either side.

---

## Per-symbol doc index

**Functions:**
- `fn-IsIronCurtainActive.md` — gate check (0x0041bf40)
- `fn-TechnoClass-IronCurtain.md` — base apply (0x0070e2b0)
- `fn-BuildingClass-IronCurtain.md` — building override (0x00457c90)
- `fn-InfantryClass-IronCurtain.md` — infantry instakill (0x00522600)
- `fn-ReadAudioVisual-IronCurtainColor.md` — RGB read site (0x0066b844)
- `fn-SuperWeaponTypeReadINI-IC.md` — type-name dispatch enum (0x006cea20)

**Structs:**
- `struct-TechnoClass-IC-fields.md` — +0x18c, +0x190, +0x194, +0x1a4, +0x1c4
- `struct-BuildingClass-IC-fields.md` — +0x528, +0x52c, +0x530, +0x540, +0x6df
- `struct-RulesClass-IC-config.md` — +0x348, +0x18a8, +0xfa8, +0xfe8
- `struct-TechnoTypeClass-IC-immune.md` — +0xd97 (Organic flag)

**Globals:**
- `global-CurrentFrameCounter.md` — 0x00a8ed84
- `global-RulesClass-Instance.md` — 0x008871e0

**Strings/INI keys:**
- `string-IronCurtainDuration.md`, `string-IronCurtainInvokeAnim.md`, `string-IronCurtainColor.md`, `string-IronCurtain-typename.md`

**Phantom (not on disk):** `fn-StartFidget-IronCurtain-Dispatch.md`, `fn-ReadCombatDamage-IronCurtainDuration.md`, `fn-ReadGeneral-IronCurtainInvokeAnim.md` — content recoverable from the struct decodes.

---

## Implementation recommendations (for /brainstorm → /write-plan)

The synthesis is not a plan, but the parity-report rollup translates directly into a short fix list:

1. Add an `Organic` flag to TechnoType (parsed from `[<unit>] Organic=`). On IC apply to an organic unit, take the instakill path instead of invulnerability. Closes finding #1.
2. Generalize the immune-flag check beyond `EntityCategory::Infantry` — anything with `Organic=yes` OR `ImmuneToIronCurtain=yes` (if the latter exists in stock — verify) routes to instakill. Closes finding #2.
3. Emit a spark in the IC-blocked-damage path. Color = gold (IC) or blue-white (Force Shield) based on `InvulnKind`. Closes finding #3.
4. Parse `IronCurtainColor=` from `[AudioVisual]` and apply as a tint overlay on IC'd sprites in the render path. Closes finding #4.
5. When applying IC to a chrono-warping unit, clear the teleport_state before stamping the IC fields. Closes finding #5.

Findings #1–4 are HIGH visibility and likely small per-fix. #5 is niche but small. The total work is probably 1–2 days of focused implementation + tests.

---

## Open questions to surface to user before downstream consumption

1. **`+0x1a4` (TechnoClass) purpose** — cleared on IC apply, no reader found in the IC system. May be a timer-pair field for another effect that incidentally overlaps the IC region. If a downstream parity audit needs this resolved, add to a follow-up RE pass.
2. **`+0x6df` (BuildingClass) gate setter** — what writes this byte? If it fires during building animation completion or production, the reset block becomes player-observable.
3. **Table at `0x007e4ce4`** — 8-entry pointer table starting with "IronCurtain"; likely a SW handler dispatch table. Role unverified.
4. **The SW manager that calls `TechnoClass__StartFidget`** — not traced in this decode. The connecting caller from the SW launch path to the per-techno apply was out of scope but should be added in a follow-up if disparities surface in area-of-effect or targeting behavior.

---

## Verdict

This system is well-bounded, well-decoded, and ready for downstream implementation work. The Rust port has substantial parity drift in the deflect path (Organic flag) and visual feedback (spark color, tint), all addressable with localized fixes.
