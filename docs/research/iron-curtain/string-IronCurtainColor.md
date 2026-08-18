# string-IronCurtainColor

## Identity

| Field | Value |
|---|---|
| String | `"IronCurtainColor"` |
| Address | `0x0083a1a4` |
| INI Section | `[AudioVisual]` |
| INI Key | `IronCurtainColor=` |
| Type | int (packed color) |
| Default | Unknown (stock YR: golden/yellow tint) |

## Verification

String address verified via `get_assembly_context 0x0066b844` (RulesClass__ReadAudioVisual).

From `get_assembly_context 0x0066b844`, verbatim assembly:
```asm
0066b844: PUSH 0x83a1a4                         ; "IronCurtainColor" string
...
0066b84c: CALL 0x005276d0                        ; CCINIClass__ReadColor (or similar color-read variant)
0066b851: MOV dword ptr [ESI + 0x18a8], EAX    ; store packed color at RulesClass+0x18a8
```

Adjacent fields in the same ReadAudioVisual function (confirmed via surrounding assembly context):
- `RulesClass + 0x18a4` = `LaserTargetColor` (stored at `0x0066b812` → `[ESI + 0x18a4]`)
- `RulesClass + 0x18a8` = `IronCurtainColor` (stored at `0x0066b851` → `[ESI + 0x18a8]`)
- `RulesClass + 0x18ac` = `BerserkColor` (stored at `0x0066b871` → `[ESI + 0x18ac]`)

**Storage field**: `RulesClass + 0x18a8` (4-byte packed color int).

## Semantics

Color tint applied to units and buildings while they are under Iron Curtain protection. The value is parsed by a CCINIClass color-read variant at `0x005276d0`. Exact color encoding (packed RGB, palette index, or other) is **YELLOW — unverified** without decompiling `0x005276d0`; the function name and packing format are not confirmed.

The color is used by the rendering layer to apply a visual tint to IC'd objects for the duration of their invulnerability. Stock YR uses a golden/yellow shimmer. This field is the INI-configurable color for that tint.

## Xref count: 1 (single consumer in ReadAudioVisual)

## Active in YR: Yes

## Unverified

Color packing format at `0x005276d0` is not confirmed. INI default value is unknown from binary analysis; stock `rulesmd.ini` would be authoritative.
