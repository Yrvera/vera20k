# STR_WarpIn / STR_WarpAway — Warp Animation INI Keys

**Kind:** string / INI-key bundle
**Task:** decode-string-warp-anim-keys
**Active in YR:** Yes — both keys read by `RulesClass__ReadGeneral` unconditionally on startup

---

## Summary

Two `[General]` INI keys naming the warp animation types for arrival and departure.
Read by `RulesClass__ReadGeneral` and stored as anim-type pointers in the
`g_RulesClass_Instance` singleton. The stored pointer at Rules+0x33C is consumed by
`AnimClass__Constructor` calls in `TeleportLocomotionClass__StateMachineTick` states
0, 2, and 5 to spawn the warp-shimmer animation.

String addresses verified via `inspect_memory_content`.
Rules struct offsets extracted from `get_assembly_context` on xref sites in
`RulesClass__ReadGeneral`.

---

## Active in YR

**Yes — unconditionally loaded** from `[General]` on startup. Both strings are xref'd
from `RulesClass__ReadGeneral` with DATA references — they are string arguments to
ReadAnim/ReadType calls. No gating flag.

---

## Key table

| INI key | String address | String (verified) | Rules offset | Type | Stock value | Role |
|---|---|---|---|---|---|---|
| `WarpIn` | `0x0083CDCC` | "WarpIn" (null at byte 6, char[7]) | `+0x33C` | AnimTypeClass ptr | `WARPIN;WAKE2` | Arrival shimmer anim; spawned at destination in state 5 |
| `WarpAway` | `0x0083CDB8` | "WarpAway" (null at byte 8, char[9]) | `+0x340` | AnimTypeClass ptr | `WARPAWAY;RING1` | Departure/destroy anim variant; distinct from the standard WarpOut |

String verification via `inspect_memory_content`:
- 0x0083CDCC → "WarpIn" (hex: `57 61 72 70 49 6E 00`)
- 0x0083CDB8 → "WarpAway" (hex: `57 61 72 70 41 77 61 79 00`)

INI values from `ini/rulesmd.ini` lines 548–550:
```
WarpIn=WARPIN;WAKE2       ; animation when warping in
WarpAway=WARPAWAY;RING1   ; animation when warping something out of existence
```

---

## Rules struct offsets (verified)

From `get_assembly_context` on xref sites in `RulesClass__ReadGeneral`:

**WarpIn** @ 0x0066E188:
```asm
0066e173: MOV EBX,[ESI + 0x338]    ; load existing ptr at Rules+0x338
0066e16d: MOV ECX,[0x007f0c9c]
0066e167: MOV [ESI + 0x32c],EAX    ; store anim ptr at Rules+0x32C (prior key)
0066e188: PUSH 0x83cdcc            ; push "WarpIn" string
0066e190: CALL 0x00528a10          ; ReadAnim call
0066e1ed: MOV [ESI + 0x33c],EAX    ; → result stored at Rules+0x33C
```
Rules+0x33C = WarpIn anim type pointer.

**WarpAway** @ 0x0066E205:
```asm
0066e1f9: MOV EBX,[ESI + 0x340]   ; load existing ptr at Rules+0x340
0066e1ed: MOV [ESI + 0x33c],EAX   ; store WarpIn result at Rules+0x33C
0066e205: PUSH 0x83cdb8            ; push "WarpAway" string
0066e20d: CALL 0x00528a10          ; ReadAnim call
; result stored at Rules+0x340
```
Rules+0x340 = WarpAway anim type pointer.

---

## Consumption in teleport locomotor

`Rules+0x33C` (WarpIn anim ptr) is consumed in:
- `TeleportLocomotionClass__StateMachineTick` state 0 (departure anim spawn at Location)
- `TeleportLocomotionClass__StateMachineTick` state 2 (warp-out anim at departure cell)
- `TeleportLocomotionClass__StateMachineTick` state 5 (arrival anim at destination Location)

Verified via `decompile_function 0x007192F0` — all three state arms call
`AnimClass__Constructor(*(undefined4*)(g_RulesClass_Instance+0x33c), ...)`.

`Rules+0x340` (WarpAway) is not directly consumed in TeleportLocomotionClass — it is the
ChronoSphere "warp something away" anim used by the temporal weapon system. YELLOW on
whether it has any teleport locomotor path.

---

## Observation note

Per memory `[feedback_chrono_miner_no_arrival_shimmer]`: WarpOut SHP plays at the depart
cell only, not at the arrival cell. The anim spawned in state 5 (arrival) is WarpIn
(`Rules+0x33C`), not WarpAway. The departure spawn in state 0 also uses WarpIn anim
(same Rules+0x33C pointer), consistent with the shimmer being one anim type used for
both the warp-out and warp-in shimmer (the SHP itself is directional — the same anim
plays forwards at arrival and is a different visual from the depart-side SHP).

---

## Proposed Ghidra labels

| Symbol | Address | Proposed name |
|---|---|---|
| 0x0083CDCC | string | STR_INIKey_WarpIn |
| 0x0083CDB8 | string | STR_INIKey_WarpAway |

---

## Out-of-scope refs

- `RulesClass__ReadGeneral` (0x006621A0 approx) — general Rules reader; not teleport-specific
- `AnimClass__Constructor` — general animation infrastructure; not teleport-specific
- TemporalClass / ChronoSphere weapon system — consumers of WarpAway; not in teleport locomotor scope

---

## Unverified / YELLOW

- **Rules+0x340 WarpAway consumption in teleport path**: The WarpAway anim is loaded at
  Rules+0x340 but no call to `AnimClass__Constructor(Rules+0x340, ...)` was found in the
  StateMachineTick decompile. It may be consumed by the ChronoSphere weapon system, not
  the locomotor. YELLOW — confirm by searching `get_xrefs_to` on the exact load site.

- **Rules+0x32C preceding key**: The assembly before the WarpIn READ shows a store to
  Rules+0x32C. This is a prior key (possibly WarpOut or another anim). Not decoded in
  this task — out of scope. YELLOW on what is at Rules+0x32C.

- **ReadAnim function at 0x00528A10**: Identified as the anim-type reader from context
  (takes INI key string, returns AnimTypeClass ptr). Name not decompiled in this session.
  YELLOW on exact method name.
