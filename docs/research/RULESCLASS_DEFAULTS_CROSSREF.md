# RulesClass: ctor defaults � INI readers cross-ref

- ctor store-offsets: 1085
- INI reader-offsets: 728
- matched fields: 723
- INI-only (no default): 5
- ctor-only (runtime/dead): 240

## �1 INI-only � fields with a reader but no ctor default

| Offset | Section | Key | Type |
|--------|---------|-----|------|
| 0x214 | AudioVisual | YuriMindControlSound | sound_idx |
| 0x9C0 | AI | AIForcePredictionFudge | int[3] |
| 0xB8C | AudioVisual | OnFire | AnimType* |
| 0xEBC | General | AICaptureLowMoneyMark | int |
| 0x1688 | General | TiberiumTransmogrify | int |

## �2 ctor-only � fields initialized but never INI-parsed

(Subset: non-zero values only, to focus on meaningful runtime state.)

| Offset | Size | Type | Default |
|--------|-----:|------|---------|
| 0x58 | 4 | `int/u32` | `&PTR_FUN_007f0d3c` |
| 0x6C | 4 | `int/u32` | `10` |
| 0x104 | 4 | `int/u32` | `&PTR_FUN_007eb6d4` |
| 0x118 | 4 | `int/u32` | `10` |
| 0x120 | 4 | `int/u32` | `&PTR_FUN_007eb6d4` |
| 0x134 | 4 | `int/u32` | `10` |
| 0x13C | 4 | `int/u32` | `&PTR_FUN_007eb6d4` |
| 0x150 | 4 | `int/u32` | `10` |
| 0x158 | 4 | `int/u32` | `&PTR_FUN_007eb6d4` |
| 0x16C | 4 | `int/u32` | `10` |
| 0x2A0 | 4 | `int/u32` | `&PTR_FUN_007eb6d4` |
| 0x2B4 | 4 | `int/u32` | `10` |
| 0x2BC | 4 | `int/u32` | `&PTR_FUN_007eb6d4` |
| 0x2D0 | 4 | `int/u32` | `10` |
| 0x2D8 | 4 | `int/u32` | `&PTR_FUN_007eb6d4` |
| 0x2EC | 4 | `int/u32` | `10` |
| 0x358 | 4 | `int/u32` | `&PTR_FUN_007e4dd8` |
| 0x36C | 4 | `int/u32` | `10` |
| 0x374 | 4 | `int/u32` | `&PTR_FUN_007e4dd8` |
| 0x388 | 4 | `int/u32` | `10` |
| 0x390 | 4 | `int/u32` | `&PTR_FUN_007e4dd8` |
| 0x3A4 | 4 | `int/u32` | `10` |
| 0x3AC | 4 | `int/u32` | `&PTR_FUN_007e4dd8` |
| 0x3C0 | 4 | `int/u32` | `10` |
| 0x3C8 | 4 | `int/u32` | `&PTR_FUN_007e4dd8` |
| 0x3DC | 4 | `int/u32` | `10` |
| 0x3E4 | 4 | `int/u32` | `&PTR_FUN_007e4dd8` |
| 0x3F8 | 4 | `int/u32` | `10` |
| 0x450 | 4 | `int/u32` | `10` |
| 0x46C | 4 | `int/u32` | `10` |
| 0x488 | 4 | `int/u32` | `10` |
| 0x600 | 4 | `int/u32` | `&PTR_FUN_007f0d3c` |
| 0x614 | 4 | `int/u32` | `10` |
| 0x648 | 4 | `int/u32` | `&PTR_FUN_007e4dd8` |
| 0x65C | 4 | `int/u32` | `10` |
| 0x6CC | 4 | `int/u32` | `&PTR_FUN_007e4dd8` |
| 0x6E0 | 4 | `int/u32` | `10` |
| 0x72C | 4 | `int/u32` | `-1 (ffffffff)` |
| 0x734 | 4 | `int/u32` | `&PTR_FUN_007e4dd8` |
| 0x748 | 4 | `int/u32` | `10` |
| 0x7D8 | 4 | `int/u32` | `10` |
| 0x7F4 | 4 | `int/u32` | `10` |
| 0x810 | 4 | `int/u32` | `10` |
| 0x82C | 4 | `int/u32` | `10` |
| 0x848 | 4 | `int/u32` | `10` |
| 0x850 | 4 | `int/u32` | `&PTR_FUN_007ed90c` |
| 0x864 | 4 | `int/u32` | `10` |
| 0x880 | 4 | `int/u32` | `&PTR_FUN_007ed90c` |
| 0x894 | 4 | `int/u32` | `10` |
| 0x8C0 | 4 | `int/u32` | `10` |
| 0x8DC | 4 | `int/u32` | `10` |
| 0x8F8 | 4 | `int/u32` | `10` |
| 0x914 | 4 | `int/u32` | `10` |
| 0x930 | 4 | `int/u32` | `10` |
| 0x94C | 4 | `int/u32` | `10` |
| 0x968 | 4 | `int/u32` | `10` |
| 0x984 | 4 | `int/u32` | `10` |
| 0x9A0 | 4 | `int/u32` | `10` |
| 0x9A8 | 4 | `int/u32` | `&PTR_FUN_007e4dd8` |
| 0x9BC | 4 | `int/u32` | `10` |
| 0x9D8 | 4 | `int/u32` | `10` |
| 0x9F4 | 4 | `int/u32` | `10` |
| 0xA10 | 4 | `int/u32` | `10` |
| 0xA2C | 4 | `int/u32` | `10` |
| 0xA48 | 4 | `int/u32` | `10` |
| 0xA64 | 4 | `int/u32` | `10` |
| 0xA6C | 4 | `int/u32` | `&PTR_FUN_007ed90c` |
| 0xA80 | 4 | `int/u32` | `10` |
| 0xA88 | 4 | `int/u32` | `&PTR_FUN_007ed90c` |
| 0xA9C | 4 | `int/u32` | `10` |
| 0xAA4 | 4 | `int/u32` | `&PTR_FUN_007ed90c` |
| 0xAB8 | 4 | `int/u32` | `10` |
| 0xAC0 | 4 | `int/u32` | `&PTR_FUN_007ed90c` |
| 0xAD4 | 4 | `int/u32` | `10` |
| 0xADC | 4 | `int/u32` | `&PTR_FUN_007ed90c` |
| 0xAF0 | 4 | `int/u32` | `10` |
| 0xB20 | 4 | `int/u32` | `&PTR_FUN_007eabe8` |
| 0xB34 | 4 | `int/u32` | `10` |
| 0xB3C | 4 | `int/u32` | `&PTR_FUN_007eabe8` |
| 0xB50 | 4 | `int/u32` | `10` |
| 0xB6C | 4 | `int/u32` | `10` |
| 0xB74 | 4 | `int/u32` | `&PTR_FUN_007eb6d4` |
| 0xB88 | 4 | `int/u32` | `10` |
| 0xB90 | 4 | `int/u32` | `&PTR_FUN_007eb6d4` |
| 0xBA4 | 4 | `int/u32` | `10` |
| 0xBD4 | 4 | `int/u32` | `10` |
| 0xC18 | 4 | `int/u32` | `10` |
| 0xC34 | 4 | `int/u32` | `10` |
| 0xC50 | 4 | `int/u32` | `10` |
| 0xC6C | 4 | `int/u32` | `10` |
| 0xC88 | 4 | `int/u32` | `10` |
| 0xCA4 | 4 | `int/u32` | `10` |
| 0xCC0 | 4 | `int/u32` | `10` |
| 0xCDC | 4 | `int/u32` | `10` |
| 0xCF8 | 4 | `int/u32` | `10` |
| 0xD14 | 4 | `int/u32` | `10` |
| 0xD30 | 4 | `int/u32` | `10` |
| 0xD4C | 4 | `int/u32` | `10` |
| 0xD94 | 4 | `int/u32` | `10` |
| 0xDB0 | 4 | `int/u32` | `10` |
| 0xDCC | 4 | `int/u32` | `10` |
| 0xDE8 | 4 | `int/u32` | `10` |
| 0xE24 | 4 | `int/u32` | `10` |
| 0xE40 | 4 | `int/u32` | `10` |
| 0xE60 | 4 | `int/u32` | `10` |
| 0xE7C | 4 | `int/u32` | `10` |
| 0xE98 | 4 | `int/u32` | `10` |
| 0xEB4 | 4 | `int/u32` | `10` |
| 0xED8 | 4 | `int/u32` | `10` |
| 0xEFC | 4 | `int/u32` | `10` |
| 0xF04 | 4 | `int/u32` | `&PTR_FUN_007e4dd8` |
| 0xF18 | 4 | `int/u32` | `10` |
| 0xF20 | 4 | `int/u32` | `&PTR_FUN_007e4dd8` |
| 0xF34 | 4 | `int/u32` | `10` |
| 0x100C | 4 | `int/u32` | `10` |
| 0x116C | 4 | `int/u32` | `10` |
| 0x1188 | 4 | `int/u32` | `10` |
| 0x11A8 | 4 | `int/u32` | `10` |
| 0x11C4 | 4 | `int/u32` | `10` |
| 0x11E0 | 4 | `int/u32` | `10` |
| 0x11FC | 4 | `int/u32` | `10` |
| 0x1218 | 4 | `int/u32` | `10` |
| 0x1234 | 4 | `int/u32` | `10` |
| 0x1250 | 4 | `int/u32` | `10` |
| 0x126C | 4 | `int/u32` | `10` |
| 0x1288 | 4 | `int/u32` | `10` |
| 0x12A4 | 4 | `int/u32` | `10` |
| 0x12C0 | 4 | `int/u32` | `10` |
| 0x12DC | 4 | `int/u32` | `10` |
| 0x12F8 | 4 | `int/u32` | `10` |
| 0x1318 | 4 | `int/u32` | `10` |
| 0x1334 | 4 | `int/u32` | `10` |
| 0x1350 | 4 | `int/u32` | `10` |
| 0x136C | 4 | `int/u32` | `10` |
| 0x1388 | 4 | `int/u32` | `10` |
| 0x13A4 | 4 | `int/u32` | `10` |
| 0x13C0 | 4 | `int/u32` | `10` |
| 0x1464 | 4 | `int/u32` | `2` |
| 0x1478 | 4 | `int/u32` | `0x2000` |
| 0x14D0 | 4 | `int/u32` | `8` |
| 0x1694 | 4 | `int/u32` | `0x402c0000 (~14.0)` |
| 0x169C | 4 | `int/u32` | `0x40140000 (~5.0)` |
| 0x16A4 | 4 | `int/u32` | `0x40240000 (~10.0)` |
| 0x16FC | 4 | `int/u32` | `0x3ff00000 (~1.0)` |

## �3 Matched � summary only

(723 fields correctly wired; see RULESCLASS_FIELDS.csv � RULESCLASS_CONSTRUCTOR_DEFAULTS.csv for the full join.)

