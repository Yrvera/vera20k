# String: "BridgeStrength" — Decode Doc

**String address:** `0x0083AD90`
**INI key:** `BridgeStrength=`
**INI section:** `[CombatDamage]` (in `rules(md).ini`)
**Stock value:** `BridgeStrength=1500` (rulesmd.ini line 816)
**Storage:** `RulesClass + 0x1740` (`int32`)
**Read function:** `CCINIClass::ReadInt @ 0x005276D0`
**Read callsite:** `0x0066CD73` (PUSH of string address) in `RulesClass::ReadCombatDamage @ 0x0066BBB0`

---

## Summary

`"BridgeStrength"` is an INI key read by `RulesClass::ReadCombatDamage` that stores the bridge
tile hit-point threshold at `RulesClass + 0x1740`. The value `1500` from `rulesmd.ini` is used
in bridge damage calculations to determine when a bridge tile collapses from direct weapon fire.
This is distinct from the C4/hut-death path, which uses `BuildingClass::Update` timers and
collapses unconditionally once the timer expires.

---

## String Verification

`inspect_memory_content 0x0083AD90` (20 bytes):
- Hex: `42 72 69 64 67 65 53 74 72 65 6E 67 74 68 00 ...`
- Detected string: `"BridgeStrength"`
- Null-terminated at byte 14

Single xref confirmed via `get_xrefs_to 0x0083AD90` → exactly one result:
`From 0066cd73 in RulesClass__ReadCombatDamage [DATA]`.

---

## INI Read Callsite

From `get_assembly_context 0x0066cd73` (12 instructions context):

```asm
0066cd60: MOV dword ptr [ESI+0x1734], EAX  ; store prior field
0066cd66: MOV EAX, dword ptr [ESI+0x1740]  ; load +0x1740 as default
0066cd6c: MOV ECX, dword ptr [0x007F0C84]  ; CCINIClass* rules INI
0066cd72: PUSH EAX                          ; default value
0066cd73: PUSH 0x0083AD90                   ; key = "BridgeStrength"
0066cd78: PUSH ECX                          ; section
0066cd79: MOV ECX, EDI                      ; this = CCINIClass*
0066cd7b: CALL 0x005276d0                   ; CCINIClass::ReadInt
0066cd86: MOV dword ptr [ESI+0x1740], EAX   ; STORE result to RulesClass+0x1740
```

`CCINIClass::ReadInt @ 0x005276D0` confirmed via `get_function_by_address 0x005276D0`.
`RulesClass::ReadCombatDamage @ 0x0066BBB0` confirmed via `get_function_by_address 0x0066cd73`
(address falls within body `0x0066BBB0 – 0x0066CF64`).

The default value is the current contents of `[ESI+0x1740]` — ReadInt only overrides if the
key exists in the INI file. Since `BridgeStrength=1500` is present in `rulesmd.ini`, the
result at runtime is `1500`.

---

## Storage

| Class | Offset | Type | Notes |
|---|---|---|---|
| RulesClass | `+0x1740` | `int32` | Bridge tile HP threshold; default 1500 |

---

## Adjacent fields (from assembly context)

| Offset | Notes |
|---|---|
| `+0x1734` | Preceding `[CombatDamage]` int field (written just before BridgeStrength callsite) |
| `+0x1740` | **BridgeStrength** (this field) |
| `+0x1754` | Following field (loaded at `0x0066CD80` after BridgeStrength store) |

---

## Callers of `RulesClass::ReadCombatDamage`

`RulesClass::ReadCombatDamage @ 0x0066BBB0` is called from `RulesClass::Process` at game startup
(same call chain as `ReadAudioVisual` and `ReadGeneral` — all invoked once during
`ScenarioClass::Full_Init`). The string is parsed exactly once per game load.

---

## Self-Proof (exit gate)

### Claim 1: String `"BridgeStrength"` at `0x0083AD90`

`inspect_memory_content 0x0083AD90` → hex `42 72 69 64 67 65 53 74 72 65 6E 67 74 68 00`,
detected string `"BridgeStrength"`, null at byte 14. **VERIFIED.**

### Claim 2: Single xref at `0x0066CD73` from `RulesClass::ReadCombatDamage`

`get_xrefs_to 0x0083AD90` → `From 0066cd73 in RulesClass__ReadCombatDamage [DATA]`.
Exactly one reference. **VERIFIED.**

### Claim 3: Storage offset `RulesClass + 0x1740`; store instruction at `0x0066CD86`

`get_assembly_context 0x0066cd73` → context_after shows `0066cd86: MOV dword ptr [ESI+0x1740], EAX`.
`ESI` = RulesClass instance (confirmed from enclosing function `RulesClass__ReadCombatDamage`).
**VERIFIED at write site.**
