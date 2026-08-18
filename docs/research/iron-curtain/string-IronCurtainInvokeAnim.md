# string-IronCurtainInvokeAnim

## Identity

| Field | Value |
|---|---|
| String | `"IronCurtainInvokeAnim"` |
| Address | `0x0083cda0` |
| INI Section | `[General]` |
| INI Key | `IronCurtainInvokeAnim=` |
| Type | AnimTypeClass* (pointer) |
| Default | `IRONBLST` |

## Verification

String address verified via `get_xrefs_to 0x0083cda0` — returns xref from `RulesClass__ReadGeneral` at `0x0066e244`.

From `get_assembly_context 0x0066e244` (RulesClass__ReadGeneral), verbatim assembly:
```asm
0066e22f: MOV EBX, dword ptr [ESI + 0x348]   ; load existing default (prior AnimTypeClass*)
...
0066e244: PUSH 0x83cda0                        ; "IronCurtainInvokeAnim" string
0066e249: PUSH ECX
0066e24a: MOV ECX, EDI
0066e24c: CALL 0x00528a10                       ; CCINIClass__ReadString
0066e251: TEST EAX, EAX
0066e253: JZ 0x0066e260                         ; if not found, keep EBX (prior value)
0066e255: LEA ECX, [ESP + 0x50]
0066e259: CALL 0x00428b80                       ; AnimTypeClass__FindOrAllocate
0066e25e: JMP 0x0066e262
0066e260: MOV EAX, EBX
0066e262: ...
0066e26b: MOV dword ptr [ESI + 0x348], EAX     ; store AnimTypeClass* at RulesClass+0x348
```

**Storage field**: `RulesClass + 0x348` (4-byte AnimTypeClass* pointer).

## Semantics

Name of the animation played when the Iron Curtain superweapon is applied to a unit. The string is looked up via `AnimTypeClass__FindOrAllocate` and stored as a pointer. Default value `IRONBLST` is the "iron blast" animation — the golden shimmer displayed over each unit as it becomes invulnerable.

Used by the IC apply path in `TechnoClass__IronCurtain` (0x0070e2b0) to trigger the animation at the target unit's position.

## Xref count: 1 (single consumer in ReadGeneral)

## Active in YR: Yes
