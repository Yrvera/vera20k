# TechnoTypeClass IC "immune" flag — decode

**Target:** `TechnoTypeClass + 0xd97` (byte)  
**Runbook:** struct-decode-v1  
**Decoded:** 2026-05-24

---

## Summary

`TechnoTypeClass + 0xd97` is the `Organic` flag. When set, Iron Curtain
application on this unit type routes to the instakill path (TakeDamage with
C4Warhead maximum damage) instead of the invulnerability path. This is the
IC "deflect" gate that the team-lead preflight identified — but the flag is
`Organic`, not "ImmuneToIronCurtain".

**Active in YR: Yes.** Organic units (Dolphins, Giant Squids) are YR-specific
units. This path fires in every standard YR skirmish map that contains water.

**Preflight correction:** The preflight note described this field as
"likely ImmuneToIronCurtain." That was incorrect. The field is `Organic`.
Verified via `TechnoTypeClass__ReadINI` decompile at `0x00712170` and confirmed
by finding `Organic=yes` in rulesmd.ini for DNOA, DNOB, DLPH, SQD.

---

## Field layout

| Offset | Size | Type | INI key | Default | Semantic |
|--------|------|------|---------|---------|----------|
| `+0xd97` | 1 byte | bool | `Organic=` | `false` | Organic unit flag. When true: IC application instakills instead of protecting. |

`param_1` in `TechnoTypeClass__ReadINI` is `int *` — BUT offsets like
`*(undefined1 *)((int)param_1 + 0xd97)` cast the pointer to `int` first,
making the offset a **direct byte offset**, not `0xd97 * 4`.
Verified: the cast is `(int)param_1 + 0xd97`.

---

## Writer

| Address | Function | INI key | Details |
|---------|----------|---------|---------|
| `0x00715037` (approx) | `TechnoTypeClass__ReadINI` at `0x00712170` | `Organic=` | `CCINIClass__ReadBool` writes to `+0xd97`. Preceding line reads `+0xd97` as default argument. |

Verified via `decompile_function 0x00712170` (output file search for "0xd97").

---

## Reader (IC-critical)

| Address | Function | Usage |
|---------|----------|-------|
| `0x004deaec` (approx) | `TechnoClass__StartFidget` (misnamed IC dispatch) at `0x004deae4` | Calls vtable+0x84 to get type class, then checks `*(char *)(type + 0xd97)`. If non-zero: skip invulnerability, call TakeDamage with `*(g_RulesClass_Instance + 0xfa8)` (C4Warhead). |

Verified via `decompile_function 0x004deae4`:
```c
iVar1 = (**(code **)(*param_1 + 0x84))();  // GetTechnoTypeClass vtable dispatch
if (*(char *)(iVar1 + 0xd97) != '\0') {
    iVar1 = (**(code **)(*param_1 + 0x84))();
    param_8 = *(undefined4 *)(iVar1 + 0xa0);  // GetOwnerHouse (approx)
    (**(code **)(*param_1 + 0x16c))  // TakeDamage vtable
        (&param_8, 0, *(undefined4 *)(g_RulesClass_Instance + 0xfa8), 0, 0, 0, 0);
    return;
}
```

The `g_RulesClass_Instance + 0xfa8` = **C4Warhead** pointer (verified from existing
doc `ANIMCLASS_CHAINING_DAMAGE_OWNERSHIP.md` which documents this offset as
`Rules->C4Warhead`). The instakill uses the C4 (maximum damage) warhead.

---

## Stock YR units with Organic=yes

From `ini/rulesmd.ini` (verified via grep):

| Section | Unit |
|---------|------|
| `DNOA` | Dolphin type A |
| `DNOB` | Dolphin type B |
| `DLPH` | Dolphin |
| `SQD` | Giant Squid |

All are YR naval organic units. No TS-legacy consideration — these units do
not exist in TS. The path is live in standard YR skirmish on water maps.

No units have this flag set by default in the building/vehicle/infantry
non-organic categories, so normal combat vehicles, tanks, and buildings
are not affected by the Organic gate.

---

## TS-legacy assessment

**Active in YR: Yes.** The `Organic=` flag and the IC-instakill branch for
organic units is live YR behavior on any water map. Frequency: fires every
match in any scenario containing dolphins or giant squids (all YR water maps
and navy-focused skirmish maps).

---

## Out-of-scope refs

| Symbol | Address | Note |
|--------|---------|------|
| `vtable+0x84` dispatch | (vtable-relative) | `GetTechnoTypeClass` or equivalent; not IC-specific |
| `vtable+0x16c` dispatch | (vtable-relative) | `TakeDamage`; not IC-specific |
| `g_RulesClass_Instance + 0xa0` | depends | Field at `TechnoTypeClass+0xa0` — house pointer or similar. Used to pass owner house to TakeDamage. |
| Bunkerable at `+0xd96` | adjacent | Preceding ReadBool writes to `+0xd96`; out of IC scope. |
| ImmuneToPoison at `+0xd3b` | nearby | ReadBool after `+0xd97`; out of IC scope. |

---

## Unverified (YELLOW)

- **TechnoTypeClass constructor default:** The default value of `+0xd97` is almost certainly `false` (0) given how ReadINI works (reads with current value as default, and units like tanks don't appear in the Organic list). Not directly verified via constructor decompile in this session.
- **`TechnoTypeClass+0xa0` purpose:** Used in the instakill path to pass something to TakeDamage. Likely the owner house pointer or a target struct. Not resolved here.
