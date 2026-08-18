# Radiation Glow Render — INI Key Surface (worknote)

**Date:** 2026-06-15
**Lane:** INI surface for radiation lighting — stock values + what Rust parses today.
**Scope:** Read-only. No code edits. In-repo `ini/` only (never the ra2nextevolution mod).
**Authority order applied:** binary → Ghidra → docs/research → ini/.

---

## 1. Stock `[Radiation]` section — exact values

### YR — `ini/rulesmd.ini` lines 913–933

```
[Radiation]
RadDurationMultiple=1     ; frames a site lasts per level of radiation
RadApplicationDelay=16    ; delay between radiation-damage applications to units
RadLevelMax=500           ; max level a cell damages as (cell may store more)
RadLevelDelay=90          ; frames between rad-LEVEL decrements
RadLightDelay=90          ; frames between rad-LIGHTING-intensity decrements
RadLevelFactor=0.2        ; scales damage done by a given radiation level
RadLightFactor=0.1        ; scales brightness contribution to the rad display
RadTintFactor=1.0         ; scales tint contribution to the rad display
RadColor=0,255,0          ; the color of the radiation (pure green)
RadSiteWarhead=RadSite    ; warhead used by irradiated tiles
```

### Base RA2 — `ini/rules.ini` lines 733–753

**Identical to YR.** Every key and value matches byte-for-byte (same `RadColor=0,255,0`,
same delays, same factors, same `RadSiteWarhead=RadSite`).

### YR-vs-RA2 difference

**NONE.** The `[Radiation]` section is unchanged between RA2 and YR. The YR (`md`) merge
does not patch any `Rad*` key. So fallback == YR value for every key.

### Keys that exist (complete list — only 10)

`RadDurationMultiple`, `RadApplicationDelay`, `RadLevelMax`, `RadLevelDelay`,
`RadLightDelay`, `RadLevelFactor`, `RadLightFactor`, `RadTintFactor`, `RadColor`,
`RadSiteWarhead`.

### Keys that do NOT exist in stock (do not look for them)

- **`RadSiteColor`** — does NOT appear anywhere in `rulesmd.ini` or `rules.ini`
  (grep: only `RadApplicationDelay` and `RadColor` contain the substring "RadColor"/
  "Color"). This is an **Ares extension**, not stock. Stock has only one color knob:
  `RadColor`. Do not parse/implement `RadSiteColor` for stock parity.
- No `RadLevel` key inside `[Radiation]` — `RadLevel` is a **per-weapon** key (see §3).

---

## 2. Lighting-relevant keys — which ones drive the glow

The three render-only keys plus the color feed the dynamic light/glow (per
`docs/research/RADIATION_EMP_GHIDRA_REPORT.md` §1.6 / §1.11, ghidra-verified at
RadSite Activation `0x0065B580`):

| Glow input        | Stock value | Role in the green glow |
|-------------------|-------------|------------------------|
| `RadColor`        | `0,255,0`   | Base RGB of the light (pure green). |
| `RadTintFactor`   | `1.0`       | `Tint{R,G,B} = ftol(RadColor.{R,G,B} * RadTintFactor)`. At 1.0 the tint == raw RadColor. |
| `RadLightFactor`  | `0.1`       | `LightIntensity = ftol(RadLevel * RadLightFactor)`. At RadLevel 500 → intensity 50. |
| `RadLightDelay`   | `90`        | Frames between light-intensity decrements (fade cadence). Independent of `RadLevelDelay`. |

Light + tint both fade **linearly** over the site lifetime (per-step decrement computed
at activation; see report §1.6 step 4). `RadLevelMax` (500) caps the level used for the
intensity computation, so a single max-rad Desolator/RadSite cell glows at intensity
~50 (`500 * 0.1`) and decays from there.

---

## 3. Rust parse status — key | stock value | parsed?

Struct: `RadiationRules` in `src/rules/ruleset.rs` (struct at lines 952–979,
`from_ini` at 998–1042, `Default` at 981–996). **All 10 keys are parsed.** Nothing is
MISSING.

| Key                   | Stock (YR=RA2) | Parsed in Rust? |
|-----------------------|----------------|-----------------|
| `RadDurationMultiple` | `1`            | YES — ruleset.rs:1007 (`get_i32`) |
| `RadApplicationDelay` | `16`           | YES — ruleset.rs:1010 (`get_i32`, `.max(1)`) |
| `RadLevelMax`         | `500`          | YES — ruleset.rs:1011 (`get_i32`) |
| `RadLevelDelay`       | `90`           | YES — ruleset.rs:1012 (`get_i32`, `.max(1)`) |
| `RadLightDelay`       | `90`           | YES — ruleset.rs:1013 (`get_i32`, `.max(1)`) |
| `RadLevelFactor`      | `0.2`          | YES — ruleset.rs:1014–1017 (parsed as **f64** straight from string, not fixed; intentional float exception for damage math) |
| `RadLightFactor`      | `0.1`          | YES — ruleset.rs:1018–1021 (`get_f32` → `SimFixed`) |
| `RadTintFactor`       | `1.0`          | YES — ruleset.rs:1022–1025 (`get_f32` → `SimFixed`) |
| `RadColor`            | `0,255,0`      | YES — ruleset.rs:1026–1035 (split on `,`, parsed as `(u8,u8,u8)`) |
| `RadSiteWarhead`      | `RadSite`      | YES — ruleset.rs:1036–1040 (string, trimmed). **NOTE:** the doc-comment at ruleset.rs:977 says "uppercased" but the code does NOT uppercase — it only `trim()`s. Minor comment drift; not a parse gap. |

**Defaults match stock exactly** (ruleset.rs:983–994): `duration_multiple=1`,
`application_delay=16`, `level_max=500`, `level_delay=90`, `light_delay=90`,
`level_factor=0.2`, `light_factor=0.1`, `tint_factor=1.0`, `color=(0,255,0)`,
`site_warhead="RadSite"`. So even an absent section yields the right values.

### Per-weapon `RadLevel`

| Key (section)        | Stock          | Parsed in Rust? |
|----------------------|----------------|-----------------|
| `RadLevel` (weapon)  | `500` for the Desolator/RadBeam weapons; `100` and `500` on others (rulesmd.ini lines 23790, 24022, 24449=500, 24512=100, 24582=500) | YES — `src/rules/weapon_type.rs:204` (`get_i32("RadLevel").unwrap_or(0)`) |

### Render-side consumption status (out of lane, but relevant to the open item)

The struct doc-comment (ruleset.rs:950–951) explicitly states the render-only keys
(`light_delay`, `light_factor`, `tint_factor`, `color`) are "parsed here so the render
layer can pick them up later." So: **fully parsed, not yet consumed by render.** The
missing piece for open-item #4 is the render-layer light/glow, NOT the INI surface.

---

## 4. How a warhead triggers a RadSite (trigger mechanism)

Two ingredients, BOTH required (per `RADIATION_EMP_GHIDRA_REPORT.md` §1.5, ghidra
`WarheadTypeClass::Detonate` `0x004690B0`, and `WARHEADTYPECLASS_REINVESTIGATION` §6):

1. **The fired weapon must carry `RadLevel > 0`** — read at weapon offset `0x158`
   (`WeaponTypeClass::ReadINI`, address `0x007728DA`). RadSite creation in `Detonate`
   is gated on `*(weapon + 0x158) > 0`. (INFERRED from doc; ghidra-verified per the
   cited report.)
2. **The warhead's `Radiation` flag** (`Radiation=yes`) — warhead offset documented as
   `+0x177` in `WARHEADTYPECLASS_REINVESTIGATION` §6, where its role is listed as
   "Immunity check + RadSite creation" in `TechnoClass::ReceiveDamage + Detonate`.

The stock plumbing: weapons that irradiate (Desolator's `RadBeamWeapon`/`CRRadBeamWeapon`/
`RadBeamWeaponE` etc.) set `RadLevel=500` and use a warhead with `Radiation=yes`.
The recurring irradiated-tile warhead is **`RadSite`** itself
(`[RadSite]` at rulesmd.ini line 27349):

```
[RadSite]
Verses=100%,100%,100%,50%,10%,10%,0%,0%,0%,100%,100%
InfDeath=7
Radiation=yes
```

`[Radiation] RadSiteWarhead=RadSite` names this warhead as the one applied to units
standing on irradiated cells (the per-tick damage warhead). So `RadSite` is both the
verse table for radiation damage and the carrier of the `Radiation=yes` flag.

### Rust parse status of the trigger keys

- Weapon `RadLevel` — PARSED (`weapon_type.rs:204`).
- Warhead `Radiation` flag — PARSED as `radiation: bool` (`src/rules/warhead_type.rs:100`,
  `from_ini` at line 190 via `get_bool("Radiation")`).
  - **Offset-doc drift (out of lane, flag-only):** the Rust comment at warhead_type.rs:99
    labels `Radiation` as offset `+0x179`, but `WARHEADTYPECLASS_REINVESTIGATION` §6 puts
    it at `+0x177` (`+0x179` is `AffectsAllies` in that report). This is a comment/offset
    mismatch, NOT a parse correctness issue (`get_bool("Radiation")` keys by name). Flagged
    for the warhead-offset lane; verify against binary before trusting either number.

---

## 5. Net for open-item #4 (radiation green glow render)

- **INI surface is complete and stock-correct in Rust.** All 10 `[Radiation]` keys plus
  weapon `RadLevel` and warhead `Radiation=yes` are parsed. Defaults match stock exactly.
- **No new INI parsing is required** for the render feature. The four glow inputs
  (`RadColor=0,255,0`, `RadTintFactor=1.0`, `RadLightFactor=0.1`, `RadLightDelay=90`)
  are already in `RadiationRules` — they are simply not consumed by the render layer yet.
- **Do NOT add `RadSiteColor`** — it is not a stock key (Ares-only).
- One cosmetic note: ruleset.rs:977 comment claims `RadSiteWarhead` is uppercased but the
  code only trims; harmless for the glow.

### Confidence

- Stock INI values, key list, RA2-vs-YR identity, Rust parse mapping: **VERIFIED** (direct
  file reads, file:line cited).
- Light/tint/intensity formulas and the RadSite-creation gate: **doc-sourced, ghidra-verified
  upstream** (`RADIATION_EMP_GHIDRA_REPORT.md` §1.5/§1.6, `0x004690B0`/`0x0065B580`); not
  re-verified in Ghidra this session — treat as INFERRED-from-verified-doc.
- Warhead `Radiation` offset (`+0x177` vs Rust `+0x179`): **UNRESOLVED** offset drift,
  parsing unaffected.
