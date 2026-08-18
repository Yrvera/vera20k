# g_RulesClass_Instance — decode

**Address:** `0x008871e0`  
**Kind:** Global (pointer to RulesClass singleton)  
**Runbook:** global-decode-v1  
**Decoded:** 2026-05-24

---

## Summary

`g_RulesClass_Instance` is the engine-wide singleton pointer to the loaded
`RulesClass` object. It holds the parsed values from `rules(md).ini`. The
Iron Curtain system reads from it in two places:

1. `*(g_RulesClass_Instance + 0xfa8)` — C4Warhead pointer, used by
   `InfantryClass__IronCurtain` and `TechnoClass__StartFidget` (misnamed IC
   dispatch) for the organic-unit instakill path.
2. `TechnoClass__StartFidget` (misnamed IC dispatch) passes it to
   `TakeDamage` when the target has `Organic=yes`.

**Active in YR: Yes** — shared engine infrastructure. Read during every
super weapon application.

**INTERNAL-ONLY from IC perspective.** The pointer itself is invisible to the
player; the IC-relevant observable is the C4Warhead damage result (instakill
of organic units).

---

## Type and address

| Field | Value |
|-------|-------|
| Address | `0x008871e0` |
| Size | 4 bytes (pointer to RulesClass) |
| Type | `RulesClass*` |
| Default | `nullptr` before initialization |

Address verified from `ADDRESS_MAP.md` entry: "g_RulesClass_Instance global pointer" at `0x008871E0`, confidence 99%, "Widely referenced."

---

## Writers

| Address | Function | When | Value written |
|---------|----------|------|---------------|
| `0x007778ce` | (unlabeled init function) | Startup / rules load | Pointer to newly allocated RulesClass object |

Writer verified via `get_xrefs_to 0x008871e0` showing a WRITE at `0x007778ce`.
The function at that address could not be decompiled (`No function found`) —
likely it is inside a function that constructs the rules singleton.

---

## IC-relevant readers

| Function | Address | IC usage | Field read |
|----------|---------|---------|-----------|
| `TechnoClass__StartFidget` (misnamed IC dispatch) | `0x004deae4` | Organic-unit instakill: passes `*(g_RulesClass_Instance + 0xfa8)` as warhead to `TakeDamage` | `+0xfa8` = C4Warhead pointer |
| `InfantryClass__IronCurtain` | `0x00522600` | Instakill infantry: same `+0xfa8` read | `+0xfa8` = C4Warhead pointer |

Verified: `decompile_function 0x004deae4` shows
`*(undefined4 *)(g_RulesClass_Instance + 0xfa8)` passed to TakeDamage vtable call.
`ANIMCLASS_CHAINING_DAMAGE_OWNERSHIP.md` documents `g_RulesClass_Instance + 0xfa8`
= `Rules->C4Warhead` (WarheadTypeClass*), confirmed by INI key `C4Warhead=` in
`[CombatDamage]` section.

---

## Relevant RulesClass field

| Offset | INI key | Section | Semantic |
|--------|---------|---------|----------|
| `+0xfa8` | `C4Warhead=` | `[CombatDamage]` | C4Warhead pointer — maximum damage warhead for instakill paths. Default value: the warhead type named by `C4Warhead=` in `[CombatDamage]`. |

---

## Out-of-scope refs

All other readers of `g_RulesClass_Instance` in the engine (> 100 read sites) are
out of scope. The full RulesClass field map is covered by the RulesClass struct
decode tasks.

---

## Unverified (YELLOW)

- **Constructor address:** The writer at `0x007778ce` is unlabeled and its containing function could not be decompiled. The initialization path for `g_RulesClass_Instance` beyond confirming the write address is unverified in this session.
- **Null check:** Whether any IC-path caller checks for null before dereferencing is not verified here. In practice, the IC system is only reachable after rules are loaded, so null deref is not a runtime risk.
