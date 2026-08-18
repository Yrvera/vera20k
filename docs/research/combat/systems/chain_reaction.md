# Chain Reaction (Tiberium + Explosive Overlays)

This doc is the canonical reference for **chain-reaction mechanics** in gamemd.exe:

- `Tiberium=` flag on Warheads (`wh+0x148`) — gate for damage-on-impact to tiberium overlays
- `Tiberium=` flag on OverlayTypes (`overlay+0x2A9`) — marks an overlay as tiberium
- `Explodes=` flag on OverlayTypes (`overlay+0x2B0`) — IC-barrel-style chain explosive
- `ChainReaction=` flag on OverlayTypes (`overlay+0x2B1`) — gates `Reduce_Tiberium` in AoE dispatch
- `[CombatDamage] TiberiumExplosionDamage=` Rules constant — global toggle (set to **0** in retail YR)
- The IC-barrel recursive Apply_area_damage chain (documented in [`splash_cellspread.md`](splash_cellspread.md) §11)
- TS-legacy filtering: most tiberium-chain mechanics are dormant in YR

This is partly an "anti-doc" — documenting what IS in the binary, what's DISABLED via INI
defaults, and what's TS-legacy unused. The classic TS "chain-reaction shockwave from
exploding tiberium" is dormant in YR; the only live chain is **explosive barrels**.

Out-of-scope:
- Cell-side warhead effects dispatched from `Apply_area_damage` → [`splash_cellspread.md`](splash_cellspread.md) §6
- IC-barrel chain (recursive Apply_area_damage call with C4Warhead) → [`splash_cellspread.md`](splash_cellspread.md) §11
- Tiberium growth / spreading (the production side, not the destruction side) → [`../../HARVESTER_MISSION_HARVEST_GHIDRA_REPORT.md`](../../HARVESTER_MISSION_HARVEST_GHIDRA_REPORT.md) and related

---

## 1. Layout — verified from live decomp of `OverlayTypeClass::ReadINI`

Decompiled live 2026-05-17 at `0x005FE7F0`:

| Offset | INI key | Type | String addr | Live decomp parse site |
|---|---|---|---|---|
| `overlay+0x298` | (Armor) | int | `0x00833580` | first ReadInt — Armor type index |
| `overlay+0x2A4` | `Strength=` | int | `s_Strength` | `0x00832B78` |
| `overlay+0x2A8` | `Wall=` | bool | `0x0081ac58` | `0x005fe8??` |
| `overlay+0x2A9` | **`Tiberium=`** | bool | `0x00817278` | IsTiberium-on-overlay (verified) |
| `overlay+0x2AA` | `Crate=` | bool | `0x00833578` | |
| `overlay+0x2AB` | `CrateTrigger=` | bool | `0x00833568` | |
| `overlay+0x2B0` | **`Explodes=`** | bool | `0x0083355c` | IC-barrel explosive (verified) |
| `overlay+0x2B2` | `Overrides=` | bool | `0x00833550` | |
| `overlay+0x2AD` | `IsVeinholeMonster=` | bool | | TS-legacy |
| `overlay+0x2AE` | `IsVeins=` | bool | `s_IsVeins 0x008184A8` | TS-legacy |
| `overlay+0x2B1` | **`ChainReaction=`** | bool | `0x008334F0` | the chain gate (verified) |
| `overlay+0x2B3` | `DrawFlat=` | bool | `0x008334e4` | |
| `overlay+0x2B5` | `IsARock=` | bool | `0x008334dc` | |
| `overlay+0x2B4` | `IsRubble=` | bool | `0x008334d0` | |

### Correction to `splash_cellspread.md`

The earlier `splash_cellspread.md` doc (iteration 6) lists:
- `OverlayType+0x2A8` | IsWall_like  ← actually `Wall=` (close enough)
- `OverlayType+0x2A9` | Veinhole_like  ← **WRONG** — actually `Tiberium=`
- `OverlayType+0x2B0` | IsChainExplosive (IC barrel)  ← actually `Explodes=` (semantically same)
- `OverlayType+0x2B1` | IsTiberium  ← **WRONG** — actually `ChainReaction=`

The mapping is swapped between `+0x2A9` and `+0x2B1`. The corrected mapping is:

- `+0x2A9` = `Tiberium=` (is THIS overlay a tiberium overlay)
- `+0x2B1` = `ChainReaction=` (can this overlay participate in the chain mechanic)

Documenting here so the splash_cellspread.md `+0x2A9`/`+0x2B1` interpretations get corrected when that doc is revisited. The behavior described in splash_cellspread.md (the cell-side gate that calls `Reduce_Tiberium`) is correct; only the flag IDENTITY labels were swapped.

### Confidence (layout)

- **Content: HIGH** — live ReadINI decomp confirms parse order and offsets.
- **Identity: HIGH** — strings cross-verified to specific addresses.
- **Binding: HIGH** — these are the only INI keys with these names; single-function parser.

---

## 2. Warhead-side `Tiberium=` flag

| Field | Value |
|---|---|
| Offset | `wh+0x148` |
| Type | `bool` |
| INI key | `Tiberium=` |
| Default | `false` |
| String addr | `0x00817278` (shared with overlay parser) |
| Parser xref | `WarheadTypeClass::ReadINI_Body 0x0075d563` |

### Semantics (from `Apply_area_damage` cell-side gate)

The cell-side check in `Apply_area_damage` (per [`splash_cellspread.md`](splash_cellspread.md) §6, verified live):

```c
if (overlay.ChainReaction (+0x2B1) != 0
    && (overlay.Tiberium (+0x2A9) == 0  ||  warhead.Tiberium (+0x148) != 0)
    && allowTiberiumChain (param_5)):
    CellClass.Reduce_Tiberium(cell)
```

Plain meaning:

- For an overlay with `ChainReaction=yes` (the chain-capable flag):
  - If the overlay is NOT a tiberium overlay → call `Reduce_Tiberium`. (This is for explosive crates, IC barrels, etc. — anything that "reacts.")
  - If the overlay IS a tiberium overlay → call `Reduce_Tiberium` only when the warhead itself has `Tiberium=yes`.
- `allowTiberiumChain` is the `param_5` of `Apply_area_damage` (gated by the caller).

So `Tiberium=yes` on a warhead is a **"this warhead destroys tiberium ore"** marker.
HE, NUKE, ARTY, fire warheads have it; weapon-specific warheads (Tanya pistol's `SA`,
Yuri's psi-beam's `PsiPulse`) typically do not.

### Confidence (warhead Tiberium flag)

- **Content: HIGH** — verified xref of `"Tiberium"` string at `0x00817278` into WarheadTypeClass::ReadINI_Body at `0x0075d563`.
- **Identity: HIGH** — single string, two parsers (warhead and overlay).
- **Binding: HIGH** — read in Apply_area_damage cell-side block, verified live.

### TS-legacy concern (warhead Tiberium flag)

The flag itself is LIVE in YR — almost every retail warhead with `CellSpread>0` sets
`Tiberium=yes`. The behavior it triggers (`Reduce_Tiberium`) is the modern "blow up ore"
behavior, not a TS-style chain. So the FLAG is live; the underlying chain-reaction
mechanism (next section) is partially dormant.

---

## 3. `ChainReaction=` overlay flag

| Field | Value |
|---|---|
| Offset | `overlay+0x2B1` |
| Type | `bool` |
| INI key | `ChainReaction=` |
| Default | (TS-era default; need to verify from constructor) |
| String addr | `0x008334F0` |
| Parser | `OverlayTypeClass::ReadINI 0x005fe9ae` |

### Retail YR INI survey

In `ini/rulesmd.ini`, `ChainReaction=no` is **explicitly set** on:
- `[TIB13]..[TIB20]` and `[Tib<image>13]..[Tib<image>20]` (the BLUE tiberium variants — Tiberium Vinifera in TS lore, retained as a separate ore type in YR)

GREEN tiberium ([TIB01]..[TIB12] and their alphas) does **NOT** explicitly set
ChainReaction. They likely inherit the default (TS default = true, per the original mechanic).

### The CHAIN damage value is 0 in retail

From `ini/rulesmd.ini` `[CombatDamage]` section:

```ini
TiberiumExplosionDamage = 0     ; the amount of damage dealt out by explosion in a big tiberium chain reaction
```

The comment is explicit: this IS the chain-reaction damage. In retail YR it is set to
**0**, meaning even when `ChainReaction=yes` overlays are blown up, the chain emits 0
damage. **The mechanism still exists in the binary; the damage value is the disable
switch.**

### Confidence

- **Content: HIGH** for the offset and parse site.
- **Identity: HIGH** — single INI key string.
- **Binding: MEDIUM** — the `+0x2B1` flag is read in the Apply_area_damage cell-side gate (gating `Reduce_Tiberium`). The TS chain-shockwave that emits `TiberiumExplosionDamage` is **not yet traced** in this iteration; it likely lives in `Reduce_Tiberium` or a sibling helper. Open follow-up.

---

## 4. `TiberiumExplosionDamage=` Rules constant

| Field | Value |
|---|---|
| Offset | `Rules+0x???` (not extracted this pass) |
| Type | int (likely; or double) |
| INI key | `TiberiumExplosionDamage=` |
| Default | **`0`** in retail YR |
| String addr | `0x0083B258` |
| Parser xref | `RulesClass::ReadCombatDamage 0x0066bc44` |
| Active in retail | NO — value is 0, mechanism is disabled |

### Semantic

This is the damage value emitted by a tiberium-chain explosion when the chain fires.
With retail YR's `=0`, the chain does no damage to surrounding units. The TS-era
mechanic (a destroyed tiberium cell deals X damage in a radius, potentially chaining
through neighbor ore cells) is therefore **dormant** in vanilla YR.

A mod can re-enable it by setting `TiberiumExplosionDamage=` to a positive integer.

### Confidence

- **Content: HIGH** for the INI string and parser xref.
- **Identity: HIGH** — single string match, single xref.
- **Binding: MEDIUM** — the Rules offset that stores this value, and the function that reads it during chain dispatch, are NOT yet traced. Open follow-up.

---

## 5. `Explodes=` overlay flag (IC barrel)

| Field | Value |
|---|---|
| Offset | `overlay+0x2B0` |
| Type | `bool` |
| INI key | `Explodes=` |
| String addr | `0x0083355c` |
| Parser | `OverlayTypeClass::ReadINI` |

### Semantic — the only LIVE chain mechanism in vanilla YR

Set on red explosive barrels and similar destructible overlays in skirmish maps. When
a barrel cell is hit, the IC-barrel chain mechanism in `Apply_area_damage` fires:

```c
// At end of Apply_area_damage, per splash_cellspread.md §11:
if (cell.OverlayTypeIndex != -1
    && OverlayType[cell.OverlayTypeIndex].byte+0x2B0 != 0):     // Explodes=yes
    // Remove the overlay, recalc map, refresh display
    FUN_00486e70()
    cell.OverlayTypeIndex = -1
    cell.RecalcAttributes()
    ...
    // RECURSIVE — detonate C4Warhead at this cell, which can chain to adjacent barrels
    Apply_area_damage(NULL, Rules.C4Warhead (Rules+0xFA8), 1, sourceHouse)
    // Plus debris + smoke + particle spawn
```

The recursion implements the chain: barrel A's explosion includes barrel B in its
CellSpread, barrel B fires its own Apply_area_damage with C4Warhead, which can reach
barrel C, etc. The chain naturally terminates when no more `Explodes=yes` cells are in
the C4Warhead's CellSpread.

`Rules.C4Warhead` (at `Rules+0xFA8`) is the recursive-call warhead. Its CellSpread and
damage define the chain radius and per-link damage.

### Confidence

- **Content: HIGH** — `splash_cellspread.md` §11 was verified live in iteration 6; the +0x2B0 field identity is now corrected via the OverlayTypeClass::ReadINI decomp.
- **Identity: HIGH** — single string, single parse site.
- **Binding: HIGH** — Apply_area_damage cell-side recursive call site is verified.

### TS-legacy filter

`Explodes=` is **LIVE** in YR — vanilla skirmish maps with red barrels exercise this
every match.

---

## 6. The two distinct flag pairs (avoiding confusion)

There are **two pairs** of related flags that are easy to confuse:

| Pair | Overlay flag | Warhead flag | Behavior |
|---|---|---|---|
| Tiberium-ore destruction | `Tiberium=yes` (`overlay+0x2A9`) | `Tiberium=yes` (`wh+0x148`) | Warhead with Tiberium=yes can destroy tiberium overlays via `Reduce_Tiberium`. Gated additionally by `ChainReaction=yes` on the overlay. |
| Chain explosive (barrel) | `Explodes=yes` (`overlay+0x2B0`) | (any warhead does it) | Any warhead hitting an `Explodes=yes` overlay triggers the recursive C4Warhead chain. |

So:
- "Tiberium" the WARHEAD KEY = "can this warhead destroy ore"
- "Tiberium" the OVERLAY KEY = "is this overlay ore"
- "Explodes" the OVERLAY KEY = "is this a barrel that chains"
- "ChainReaction" the OVERLAY KEY = "can this overlay participate in the destruction
  chain" (gates Reduce_Tiberium; on TS, also gates the chain shockwave)

`ChainReaction=no` on blue tiberium ([TIB13-20]) means **a hit on blue tiberium does NOT call `Reduce_Tiberium`** — even with a Tiberium=yes warhead. The ore at that cell persists. This is the visible YR behavior where high-value blue tiberium is destruction-resistant (it can be harvested but not blown up the same way).

---

## 7. TS-legacy filter — what's dead, what's live

| Mechanism | TS-era purpose | YR status |
|---|---|---|
| Tiberium chain shockwave (`TiberiumExplosionDamage`) | A blown-up ore cell deals N damage in a radius, potentially chaining to neighbor ore | **Dormant** — `TiberiumExplosionDamage=0` in retail |
| `Veins` system (`IsVeins=`, `IsVeinholeMonster=`) | TS veinhole monsters and vein creep terrain | **Dead in YR** — no `IsVeins=yes` or `IsVeinholeMonster=yes` overlay in retail rulesmd |
| `Veinhole=` warhead (`wh+0x17B`) | Damages vein creeper / veinhole monster | **Parsed but irrelevant** — no shipping YR warhead with `Veinhole=yes` |
| `Tiberium=` warhead (the `wh+0x148` flag) | Damage tiberium overlays | **LIVE** — most retail damage warheads set it |
| `ChainReaction=` overlay flag (`+0x2B1`) | Gates the chain shockwave + Reduce_Tiberium | **PARTIALLY LIVE** — the Reduce_Tiberium gate is live; the shockwave is dormant due to `TiberiumExplosionDamage=0` |
| `Explodes=` overlay flag (`+0x2B0`) | IC barrels chain via recursive C4Warhead | **LIVE** — vanilla skirmish maps use this |

The `Tiberium=` warhead flag is live and matters every match. The `ChainReaction` /
`TiberiumExplosionDamage` chain mechanism is "code path is reachable, damage value is
zero" — i.e., the chain runs but emits no damage. The IC barrel mechanism is fully
live.

---

## 8. Chain-mechanism summary table

| Trigger | Mechanism | Active in YR? |
|---|---|---|
| Warhead with `Tiberium=yes` hits a `Tiberium=yes` overlay with `ChainReaction=yes` | `Reduce_Tiberium(cell)` clears the ore | YES (green tiberium); NO for blue (ChainReaction=no) |
| Any warhead hits an `Explodes=yes` overlay (barrel) | Recursive `Apply_area_damage(0, Rules.C4Warhead, 1, sourceHouse)` chains through adjacent barrels | YES |
| Tiberium cell destroyed → adjacent units take shockwave damage | `TiberiumExplosionDamage` × radius (TS mechanism) | NO (damage value = 0 in retail) |
| Veinhole monster reacts to damage | Veinhole-specific code | NO (no veinholes in YR maps) |
| `IsRubble=yes` overlay destroyed → reverts to terrain | Standard overlay destruction | YES (rocks/rubble pile, terrain effect) |

---

## 9. Why this doc is short (sparse system in YR)

By design, "chain reaction" is a thin layer in YR:
- The TS chain-shockwave damage is OFF (TiberiumExplosionDamage=0).
- The Veinhole system is unused.
- The only live chain is **IC barrels** (Explodes=yes + recursive C4Warhead) — which is
  documented in detail in [`splash_cellspread.md`](splash_cellspread.md) §11.

The flags that ARE used (`Tiberium=yes` on warheads, `ChainReaction=yes` on green tib,
`Explodes=yes` on barrels) are well-defined and easy to reproduce. The TS-legacy
machinery is documented here primarily for completeness — modders re-enabling
`TiberiumExplosionDamage>0` resurrects the TS chain in YR.

---

## 10. Edge cases

| Case | Behavior |
|---|---|
| Warhead `Tiberium=yes` hits non-tiberium ChainReaction-yes overlay | `Reduce_Tiberium` called. Specific effect depends on the overlay — for non-ore overlays the call may be a no-op or have specific side effects. Open follow-up. |
| Warhead `Tiberium=no` hits tiberium overlay | `Reduce_Tiberium` is NOT called (the gate requires warhead.Tiberium OR overlay-not-tiberium). Ore is untouched. Useful for weapons that should not destroy ore (Mind Control, Magnetron, ChronoBeam). |
| Warhead `Tiberium=yes` hits blue tiberium (ChainReaction=no) | `Reduce_Tiberium` is NOT called (the outer `+0x2B1` gate). Ore is untouched. **This is why a tank shell does not destroy blue ore — by ChainReaction=no on those overlays.** |
| Two adjacent IC barrels destroyed simultaneously | Each triggers its own recursive C4Warhead Apply_area_damage. Order depends on cell iteration order in the splash dispatcher. Damage may double-count for units in the overlap. |
| `Explodes=yes` AND `Tiberium=yes` on the same overlay | Both mechanisms fire — Reduce_Tiberium and the recursive chain. Untested combination (no retail overlay has both). |
| Modder sets `TiberiumExplosionDamage=50` | TS chain shockwave activates. Each destroyed tiberium cell emits 50 damage in (presumably) a small radius, potentially chaining via the same Apply_area_damage path. Specific radius and chain-depth control are TS-legacy and not traced in YR. Use with caution. |

---

## 11. Open follow-ups

1. **Trace `Reduce_Tiberium` consumer of `TiberiumExplosionDamage`.** Find where `Rules+0x???` (the parsed `TiberiumExplosionDamage` value) is read at runtime and what shockwave radius/dispatch it produces. Priority: MEDIUM. The mechanism is disabled by `=0` in retail, but the trace would document the dormant code path for modders.
2. **Fix `splash_cellspread.md` §6 flag identities.** The `+0x2A9` and `+0x2B1` labels need to be swapped (current: Veinhole_like / IsTiberium; correct: Tiberium / ChainReaction). Priority: HIGH for doc accuracy. Track as a follow-up to revisit splash_cellspread.md.
3. **`OverlayType+0x2A8` `Wall=` flag.** The decomp shows a ReadBool with string at `&DAT_0081ac58`. Verify the key name is literally `Wall=` (or a variant). Priority: LOW.
4. **`Rules.TiberiumExplosionDamage` Rules offset.** Not extracted in this pass. Trace `RulesClass::ReadCombatDamage 0x0066bc44` for the storage offset. Priority: LOW.
5. **Default value of `ChainReaction=` if unset.** In retail INI, only blue tib sets `=no`. Green tib doesn't set it — what is the default? Trace `OverlayTypeClass::Constructor`. Likely default = true (TS heritage). Priority: LOW.
6. **`+0x2A8` Wall= overlay flag interaction with chain mechanism.** Does an `Explodes=yes, Wall=yes` overlay (theoretical) chain through? Priority: VERY LOW (no retail combination).
7. **VeinholeMonster TS-only confirmation.** The `IsVeinholeMonster=` and `IsVeins=` parser exists. Verify NO retail overlay sets them, then officially declare the system TS-only-dead. Priority: LOW (likely already known).

---

## 12. Sources

- Live decompilation of `OverlayTypeClass::ReadINI` at `0x005FE7F0` (2026-05-17) — confirmed `+0x2A9 = Tiberium`, `+0x2B0 = Explodes`, `+0x2B1 = ChainReaction`.
- Live xrefs (2026-05-17):
  - `"Tiberium"` at `0x00817278` → WarheadTypeClass::ReadINI_Body `0x0075d563`, OverlayTypeClass::ReadINI `0x005fe7f2`, plus 7 other (mostly data-section) refs
  - `"ChainReaction"` at `0x008334F0` → OverlayTypeClass::ReadINI `0x005fe9ae`
  - `"TiberiumExplosionDamage"` at `0x0083B258` → RulesClass::ReadCombatDamage `0x0066bc44`
- INI quotes from `ini/rulesmd.ini`:
  - line 811: `TiberiumExplosionDamage = 0`
  - lines 26436, 26559, 26577, ... (many): warhead sections with `Tiberium=yes`
  - lines 28187..28508: `[TIB01]..[TIB20]` and `[Tib*]` alpha overlays
  - lines 28331..28395: blue-tib `ChainReaction=no` entries
- Existing canonical doc cross-references:
  - [`splash_cellspread.md`](splash_cellspread.md) §6 (cell-side warhead effects in Apply_area_damage) — **needs flag-identity correction per §1 of this doc**
  - [`splash_cellspread.md`](splash_cellspread.md) §11 (IC-barrel recursive Apply_area_damage chain) — verified live
  - [`../../WARHEADTYPECLASS_FULL_STRUCT_LAYOUT.md`](../../WARHEADTYPECLASS_FULL_STRUCT_LAYOUT.md) — `wh+0x148 = Tiberium`
- Sister system docs: [`damage_formula.md`](damage_formula.md), [`splash_cellspread.md`](splash_cellspread.md), [`warhead_detonate_dispatch.md`](warhead_detonate_dispatch.md).
