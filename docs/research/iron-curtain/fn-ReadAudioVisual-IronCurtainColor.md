# RulesClass::ReadAudioVisual — IronCurtainColor Read Site

**Address:** `0x0066b844` (containing function)
**Kind:** function (`__fastcall`)
**Focus:** `IronCurtainColor` INI key read site at data-ref `0x0083a1a4`
**Runbook:** function-decode-v1
**Verified via:** `decompile_function 0x0066b844`

---

## Summary

`RulesClass::ReadAudioVisual` reads the entire `[AudioVisual]` INI section into the
RulesClass instance. This decode is scoped to the `IronCurtainColor` key and its
neighboring color fields. The key is stored in a single packed-integer RulesClass field
using `CCINIClass__ReadInt`, not a 3-byte RGB struct. `ForceShieldColor` is also read
in the same block, one field away.

---

## Active in YR

**Yes.** This function is the rules-loading path, called once at game init. `IronCurtainColor`
controls the tint applied to IC'd units in rendering — player-visible on every IC'd unit
for the full IC duration.

---

## IronCurtainColor INI Key

**Decompilation excerpt** (from `decompile_function 0x0066b844`, param_1 is `undefined4 *`):

```c
// param_1 is undefined4 * → param_1[N] = byte offset N×4

uVar3 = CCINIClass__ReadInt(PTR_s_AudioVisual_007f0c7c,
                             s_IronCurtainColor_0083a1a4,   // key string "IronCurtainColor"
                             param_1[0x62a]);               // current/default value
param_1[0x62a] = uVar3;                                     // stored result
```

**Param_1 pointer arithmetic** (per CLAUDE.md rule — param_1 is `undefined4 *`):
- `param_1[0x62a]` = byte offset `0x62a × 4 = 0x18A8`

| INI Key | String Address | ReadINI Function | RulesClass Offset | Type |
|---------|---------------|-----------------|-------------------|------|
| `IronCurtainColor` | `0x0083a1a4` | `CCINIClass__ReadInt` | `+0x18A8` | packed `int` (single int color code) |

**INI section:** `[AudioVisual]` (via `PTR_s_AudioVisual_007f0c7c` — a pointer to the
AudioVisual section handle, confirmed by all reads in this function using the same pointer).

---

## Neighboring Color Fields

Found in the same decompilation block (immediate neighbors in RulesClass):

```c
uVar3 = CCINIClass__ReadInt(PTR_s_AudioVisual_007f0c7c, s_LaserTargetColor_0083a1b8, param_1[0x629]);
param_1[0x629] = uVar3;  // byte offset 0x629*4 = 0x18A4 — LaserTargetColor

uVar3 = CCINIClass__ReadInt(PTR_s_AudioVisual_007f0c7c, s_IronCurtainColor_0083a1a4, param_1[0x62a]);
param_1[0x62a] = uVar3;  // byte offset 0x62a*4 = 0x18A8 — IronCurtainColor

uVar3 = CCINIClass__ReadInt(PTR_s_AudioVisual_007f0c7c, s_BerserkColor_0083a194, param_1[0x62b]);
param_1[0x62b] = uVar3;  // byte offset 0x62b*4 = 0x18AC — BerserkColor

uVar3 = CCINIClass__ReadInt(PTR_s_AudioVisual_007f0c7c, s_ForceShieldColor_0083a180, param_1[0x62c]);
param_1[0x62c] = uVar3;  // byte offset 0x62c*4 = 0x18B0 — ForceShieldColor
```

**ForceShieldColor** is the Force Shield (Yuri's secondary super weapon) tint — stored
immediately after `IronCurtainColor` and using the same `ReadInt` format. Both share
the IC dispatch path (`TechnoClass::IronCurtain` with `is_force_shield` flag).

---

## Storage Type Analysis

`CCINIClass__ReadInt` is used — NOT `CCINIClass__ReadColorRGB`. Compare nearby:
- `LocalRadarColor`, `LineTrailColorOverride`, `ChronoBeamColor`, `MagnaBeamColor`
  all use `CCINIClass__ReadColorRGB` returning `undefined2 *` with a 3-byte struct.
- `IronCurtainColor`, `ForceShieldColor`, `LaserTargetColor`, `BerserkColor`
  all use `CCINIClass__ReadInt` returning a single `undefined4`.

**Conclusion:** `IronCurtainColor` is stored as a **packed 16-bit color integer** in
the original Win32/WW color format (RGB555 or similar), NOT as a separate R,G,B triplet.
The `ReadInt` call reads a decimal/hex integer from the INI file.

> Note: in-repo `ini/rulesmd.ini` should be checked for the default value. Grepping
> `IronCurtainColor` in the INI will confirm format and default.

---

## Struct Field Summary

| Offset | INI Key | Section | ReadINI fn | Default (YELLOW — unverified) | Notes |
|--------|---------|---------|------------|-------------------------------|-------|
| `+0x18A4` | `LaserTargetColor` | `[AudioVisual]` | `ReadInt` | — | adjacent, out-of-scope |
| `+0x18A8` | `IronCurtainColor` | `[AudioVisual]` | `ReadInt` | see ini/rulesmd.ini | packed int color |
| `+0x18AC` | `BerserkColor` | `[AudioVisual]` | `ReadInt` | — | adjacent, out-of-scope |
| `+0x18B0` | `ForceShieldColor` | `[AudioVisual]` | `ReadInt` | — | Force Shield tint |

---

## Callers

Not enumerated — this function is the rules-init path called once at load. The
`IronCurtainColor` value at `+0x18A8` is read downstream by the rendering layer to
apply the IC tint to units and buildings under the effect.

---

## TS-vs-YR Filter

**Active in YR: Yes.** `IronCurtainColor` is a visible tint applied to all IC'd entities
in standard YR play. `ForceShieldColor` is similarly active for Yuri's Force Shield.

---

## Out-of-Scope Refs

| Symbol | Reason |
|--------|--------|
| `CCINIClass__ReadColorRGB` | Other color keys use this; relevant for understanding color storage format difference |
| `ForceShieldColor` at `+0x18B0` | Force Shield color — IC family sibling; scope-explorer may want to add |
| `BerserkColor` at `+0x18AC` | Adjacent field — low relevance for IC decode |
| Downstream rendering consumer of `RulesClass+0x18A8` | Not found in this decode; scope-explorer should look for xrefs from the IC rendering path |

---

## Unverified Claims

> **YELLOW**

- Default value of `IronCurtainColor` in stock YR rulesmd.ini: not checked in this
  decode. The in-repo `ini/rulesmd.ini` should be grepped.
- The exact packed color format (RGB555? BGR555? Windows COLORREF?) requires checking
  either the `CCINIClass__ReadInt` implementation or a stock INI default to confirm.
