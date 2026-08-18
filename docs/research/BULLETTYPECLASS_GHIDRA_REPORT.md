# BulletTypeClass — Ghidra Research Report

**Program:** gamemd.exe
**Primary addresses:**
- `0x0046BBC0` — `BulletTypeClass::Constructor`
- `0x0046BEE0` — `BulletTypeClass::ReadINI`
- `0x0046BE10` — *(Ghidra-labeled "BulletTypeClass__ReadINI_wrapper" but actually the destructor; see §6)*
- `0x006C2E30` — COM CreateInstance call site (`operator_new(0x2F8)` + constructor)

**Class instance size:** **0x2F8 bytes (760)** — verified via `push 0x2F8` immediately preceding `operator_new` and the constructor invocation at `0x006C2E30`.

**Confidence:** High (every offset and key in this report verified by decompiling the constructor and ReadINI, plus reading the underlying string-table bytes).
**Active in YR:** Yes for all keys except `Proximity=` which is parsed but dead (see §3.3).

This report extends and corrects the existing `BULLET_PROJECTILE_SYSTEM_CONSOLIDATED_REPORT.md` §3 and §4. The consolidated report's §3.3 ("ReadINI_Part2") is **incorrect** — see §5 of this report for the full story.

---

## 1. Overview

`BulletTypeClass` is the **type definition** for projectiles — one instance per `[ProjectileSection]` listed in `rulesmd.ini`. It holds immutable per-projectile properties read from INI: trajectory flags (`Arcing`, `ROT`, `Floater`, `Inviso`), targeting flags (`AA`, `AG`), detonation behavior (`Cluster`, `Airburst`, `AirburstWeapon`, `ShrapnelWeapon`), animation rate (`AnimLow/High/Rate`), and the embedded reference to the SHP image animation (`Image=`).

A `BulletClass` runtime instance points to its `BulletTypeClass` at `BulletClass+0xAC`; all per-tick logic in `BulletClass::AI` reads its immutable parameters through that pointer. `BulletTypeClass` does **not** participate in per-tick simulation — it is pure configuration.

Inheritance chain (all classes COM-style with multi-vtable):

```
IUnknown
  └─ IPersistStream
      └─ IRTTITypeInfo
          └─ INoticeSink
              └─ AbstractClass        (vtable + UniqueID, ~0x20 bytes)
                  └─ AbstractTypeClass (adds Name, IniName, etc.)
                      └─ ObjectTypeClass (adds Image, palette, art bindings — ~0x294 bytes)
                          └─ BulletTypeClass (adds bullet-specific flags 0x294-0x2F7, total 0x2F8)
```

Multiple vtable slots (set in constructor):
- `vtable__BulletTypeClass` → primary
- `vtable__BulletTypeClass__secondary_4`
- `vtable__BulletTypeClass__secondary_8`
- `vtable__BulletTypeClass__secondary_12`

---

## 2. Class Layout — bullet-specific fields (0x294 - 0x2F7)

`param_1` in both `Constructor` and `ReadINI` is decompiled differently:
- **Constructor (`0x0046BBC0`)**: `param_1` is `undefined4 *` → `param_1[N]` means byte offset `N * 4`.
- **ReadINI (`0x0046BEE0`)**: `param_1` is `int` → all `param_1 + N` are direct byte offsets.

This is the standard CLAUDE.md decompilation pitfall. All offsets in this report are **byte offsets from BulletTypeClass instance base**.

> **Note (2026-05-07):** Three of the bytes in this 0x294-0x29F block are read directly by
> `TechnoClass::InRange` (via `weapon[0xA0]` = Projectile pointer) to gate distance
> branches and range bonuses. See [TECHNOCLASS_INRANGE_DISTANCE_GHIDRA_REPORT.md §0/§3/§5/§7](TECHNOCLASS_INRANGE_DISTANCE_GHIDRA_REPORT.md):
>
> - **`+0x295 Floater`** → Branch B per-projectile gravity override (uses
>   `FUN_0048ACF0 = Rules.Gravity × _DAT_007e1738`). TS-legacy — no standard YR
>   projectile sets it.
> - **`+0x297 SubjectToElevation`** → gates the **height-fire range bonus** (high-ground
>   advantage) in both Branch A and Branch B. The bonus formula is `(target_height −
>   attacker_height) / Rules.ElevationIncrement` plus a ballistic distance term.
> - **`+0x29B Arcing`** → switches InRange to Branch B (2D distance + ballistic-arc
>   reachability check, using `Rules.Gravity` as the default gravity scalar). Used by V3,
>   Tank Howitzer, Prism, MIRV, etc.
>
> These flags propagate from rules.ini to InRange via `WeaponType.Projectile`
> (`WeaponTypeClass+0xA0`), not via the Warhead. The earlier InRange report incorrectly
> attributed them to TechnoTypeClass / WarheadTypeClass; corrected on 2026-05-07.

| Offset | Size | Type | Field | Default | INI Key | Notes |
|--------|------|------|-------|---------|---------|-------|
| 0x294  | 1 | bool | Airburst        | 0 | `Airburst`        | Detonates in-air; spawns `AirburstWeapon` sub-bullets (bypasses Cluster loop). |
| 0x295  | 1 | bool | Floater         | 0 | `Floater`         | Use alternate gravity (`FUN_0048ACF0`). TS-era; no standard YR unit sets it. **Also gates per-projectile gravity in `TechnoClass::InRange` Branch B.** |
| 0x296  | 1 | bool | SubjectToCliffs | 0 | `SubjectToCliffs` | Cliff deflection (BounceCheck). |
| 0x297  | 1 | bool | SubjectToElevation | 0 | `SubjectToElevation` | Terrain elevation affects path. **Also gates the height-fire range bonus in `TechnoClass::InRange`.** |
| 0x298  | 1 | bool | SubjectToWalls  | 0 | `SubjectToWalls`  | Wall deflection (BounceCheck). |
| 0x299  | 1 | bool | VeryHigh        | 0 | `VeryHigh`        | Exempts from fly-by approach detonation. |
| 0x29A  | 1 | bool | **Shadow**      | **1** | `Shadow`        | Draw ground shadow. **Non-zero default.** |
| 0x29B  | 1 | bool | Arcing          | 0 | `Arcing`          | Ballistic gravity path. **Also switches `TechnoClass::InRange` to Branch B (2D distance + ballistic-arc reachability check).** |
| 0x29C  | 1 | bool | Dropping        | 0 | `Dropping`        | "HasDropped" / paratroop bomb. TS-era. |
| 0x29D  | 1 | bool | Level           | 0 | `Level`           | Straight-line ground-hugging. |
| 0x29E  | 1 | bool | Inviso          | 0 | `Inviso`          | Invisible bullet; raycast + instant impact in `BulletClass::Fire`. **If true, ReadINI skips SHP image load.** |
| 0x29F  | 1 | bool | Proximity       | 0 | `Proximity`       | **Read and stored, never consulted by any code path.** Dead in YR. See §3.3. |
| 0x2A0  | 1 | bool | Ranged          | 0 | `Ranged`          | The real proximity-fuse gate (combined with `ROT>0`). |
| 0x2A1  | 1 | bool | (Rotates inverted) | **1** | `Rotates` (art) | **In-memory storage is NEGATED.** `Rotates=yes` → field becomes `false`; `Rotates=no` → field becomes `true`. Default in-memory `1` means "Rotates=no" semantically. |
| 0x2A2  | 1 | bool | Inaccurate      | 0 | `Inaccurate`      | No target-snap on detonation. |
| 0x2A3  | 1 | bool | FlakScatter     | 0 | `FlakScatter`     | Combined with `Inviso=yes`: scatter offset in Fire. |
| 0x2A4  | 1 | bool | AA              | 0 | `AA`              | Valid against aircraft. |
| 0x2A5  | 1 | bool | **AG**          | **1** | `AG`              | Valid against ground. **Non-zero default.** |
| 0x2A6  | 1 | bool | Degenerates     | 0 | `Degenerates`     | Damage decrements each AI tick (min 5). |
| 0x2A7  | 1 | bool | Bouncy          | 0 | `Bouncy`          | Reflect velocity off ground. |
| 0x2A8  | 1 | bool | AnimPalette     | 0 | `AnimPalette` (art) | Use anim-defined palette. |
| 0x2A9  | 1 | bool | FirersPalette   | 0 | `FirersPalette`   | Use firer's house color (copied to `BulletClass+0x114` at Init). |
| 0x2AA  | 2 | — | (padding)         | 0 | — | |
| 0x2AC  | 4 | int  | **Cluster**     | **1** | `Cluster`         | Sub-munition count for non-Airburst detonation loop. **Non-zero default.** |
| 0x2B0  | 4 | ptr  | AirburstWeapon  | NULL | `AirburstWeapon`  | `WeaponTypeClass*` for airburst sub-weapon. |
| 0x2B4  | 4 | ptr  | ShrapnelWeapon  | NULL | `ShrapnelWeapon`  | `WeaponTypeClass*` for shrapnel sub-weapon. |
| 0x2B8  | 4 | int  | ShrapnelCount   | 0 | `ShrapnelCount`   | Negative = distance-based count. |
| 0x2BC  | 4 | int  | DetonationAltitude | 0 | `DetonationAltitude` | Z threshold for Vertical/Straight detonation. |
| 0x2C0  | 1 | bool | Vertical        | 0 | `Vertical`        | Straight vertical descent (V3 terminal phase). |
| 0x2C1  | 7 | — | (padding)        | 0 | — | Aligns 8-byte double at 0x2C8. |
| 0x2C8  | 8 | double | **Elasticity** | **0.75** | `Elasticity`      | Bounce energy retention. Stored as IEEE 754 — constructor writes 0x3FE8000000000000. |
| 0x2D0  | 4 | int  | **Acceleration** | **3** | `Acceleration`    | Speed change per tick (homing ramp). **Non-zero default.** |
| 0x2D4  | 4 | int(RGB) | Color       | 0 | `Color`           | Trail/line color (packed RGB). |
| 0x2D8  | 4 | ptr  | Trailer         | NULL | `Trailer` (art)   | `AnimTypeClass*` for trail effect. |
| 0x2DC  | 4 | int  | ROT             | 0 | `ROT`             | Rate of turn; >0 = homing missile, ≤0 = ballistic/straight. |
| 0x2E0  | 4 | int  | CourseLockDuration | 0 | `CourseLockDuration` | Ticks of locked heading after launch (homing). |
| 0x2E4  | 4 | int  | **SpawnDelay**  | **3** | `SpawnDelay` (art) | Trailer spawn interval in ticks. **Non-zero default.** |
| 0x2E8  | 4 | int  | (uninitialized by ReadINI) | 0 | — | Constructor zeros it. **No INI key writes here on BulletType.** See §5 for why prior docs claimed RandomRate. |
| 0x2EC  | 1 | bool | Scalable        | 0 | `Scalable`        | Trail rate-throttle gate (only honored from `UnitClass::Fire`). |
| 0x2ED  | 3 | — | (padding)        | 0 | — | |
| 0x2F0  | 4 | int  | **Arm**         | 0 | `Arm`             | **Arming delay (ticks) for ProximityDetector.** Wired in `BulletClass::Fire` → `ProximityDetector::Set`. **NOT a speed field.** |
| 0x2F4  | 1 | byte | AnimLow         | 0 | `AnimLow` (art)   | First sprite frame (read as int, stored as byte). |
| 0x2F5  | 1 | byte | AnimHigh        | 0 | `AnimHigh` (art)  | Last sprite frame. |
| 0x2F6  | 1 | byte | AnimRate        | 0 | `AnimRate` (art)  | Ticks per animation frame. |
| 0x2F7  | 1 | bool | Flat            | 0 | `Flat` (art)      | Flat-to-ground render flag. |
| **0x2F8** | — | — | **(end of struct)** | — | — | Total class size = 0x2F8 (760 bytes). |

`Image=` is not a stored field on the class — its value is read into the inherited `ObjectTypeClass` string buffer at `+0x1F8` (24 bytes) and used immediately to look up the art-section name.

---

## 3. Constructor — `BulletTypeClass::Constructor` @ 0x0046BBC0

**Signature:** `BulletTypeClass *__thiscall BulletTypeClass::Constructor(BulletTypeClass *this, INI *ini)`

```c
BulletTypeClass *Constructor(BulletTypeClass *this, INI *ini) {
    ObjectTypeClass::Constructor(this, ini);   // base init, sets fields 0x00..0x293

    // Bullet-specific defaults (every byte/int in 0x294..0x2F7):
    Airburst        = 0;
    Floater         = 0;
    SubjectToCliffs = 0;
    SubjectToElevation = 0;
    SubjectToWalls  = 0;
    VeryHigh        = 0;
    Shadow          = 1;       // ★ non-zero
    Arcing          = 0;
    Dropping        = 0;
    Level           = 0;
    Inviso          = 0;
    Proximity       = 0;
    Ranged          = 0;
    (Rotates inv)   = 1;       // ★ inverted-storage default — semantically Rotates=no
    Inaccurate      = 0;
    FlakScatter     = 0;
    AA              = 0;
    AG              = 1;       // ★ non-zero
    Acceleration    = 3;       // ★ non-zero (param_1[0xB4])
    SpawnDelay      = 3;       // ★ non-zero (param_1[0xB9])
    Degenerates     = 0;
    Bouncy          = 0;
    AnimPalette     = 0;
    field_0x2E8     = 0;       // never overwritten by ReadINI on BulletType
    FirersPalette   = 0;
    Cluster         = 1;       // ★ non-zero (param_1[0xAB])
    AirburstWeapon  = NULL;
    ShrapnelWeapon  = NULL;
    ShrapnelCount   = 0;
    DetonationAltitude = 0;
    Vertical        = 0;
    Elasticity      = 0.75;    // ★ non-zero (param_1[0xB2]=0, param_1[0xB3]=0x3FE80000)
    Color           = 0;
    Trailer         = NULL;
    ROT             = 0;
    CourseLockDuration = 0;
    Scalable        = 0;
    Arm             = 0;
    AnimLow         = 0;
    AnimHigh        = 0;
    AnimRate        = 0;
    Flat            = 0;

    this->vtable[0..3] = vtable__BulletTypeClass[primary, secondary_4, _8, _12];
    AbstractClass::AssignUniqueID(this + 1);   // sets +0x10 UniqueID (inherited)

    // ObjectTypeClass tail-fix (some inherited fields written after vtable):
    field_0x234 = 1;  field_0x22F = 1;
    field_0x230 = 0;  field_0x231 = 0;
    field_0x232 = 1;  field_0x233 = 1;  field_0x235 = 0;

    // Register in two global tables:
    if (DAT_00a83c90 < DAT_00a83c88 || ...)   // class-list growth check
        ((DynVec *)DAT_00a83c84)[DAT_00a83c90++] = this;
    if (DAT_00b0f680 has room || grow)         // instance-list growth check
        ((DynVec *)DAT_00b0f674)[DAT_00b0f680++] = this;

    return this;
}
```

**Non-zero defaults — these MUST be honored in any port:**
- `Shadow=true`
- `AG=true`
- `Cluster=1`
- `Acceleration=3`
- `SpawnDelay=3`
- `Elasticity=0.75`
- `Rotates`-inverted = 1 (semantically `Rotates=no` is the default)

Any field not in the constructor's body (Inaccurate, AA, AnimLow, etc.) defaults to zero/false/NULL.

---

## 4. ReadINI — `BulletTypeClass::ReadINI` @ 0x0046BEE0

**Signature:** `bool __thiscall BulletTypeClass::ReadINI(BulletTypeClass *this, INI *ini)`
**Vtable slot:** primary vtable[11] (offset 0x2C). Verified — only xref to `0x0046BEE0` is the data write at `0x007E49AC` inside `vtable__BulletTypeClass`.

### 4.1 Flow

```c
bool ReadINI(BulletTypeClass *this, INI *ini) {
    INIClass::ClearSectionCache();
    if (!ObjectTypeClass::ReadINI(this, ini))    // base class reads Name=, Image=, palette
        return false;

    int rules_section = (int)this + 0x24;        // ObjectTypeClass section-name buffer
    int art_section   = (int)this + 0x1F8;       // Image= name buffer (inherited)

    // Read each field (existing-value passed as default):
    Arm            = ini->ReadInt   (rules_section, "Arm",            Arm);
    ROT            = ini->ReadInt   (rules_section, "ROT",            ROT);
    CourseLockDuration = ini->ReadInt(rules, "CourseLockDuration", CourseLockDuration);
    Elasticity     = ini->ReadDouble(rules, "Elasticity", Elasticity);  // 8 bytes
    Acceleration   = ini->ReadInt   (rules, "Acceleration", Acceleration);
    Color          = ini->ReadColor (rules, "Color", Color);          // FUN_00474A90
    Arcing         = ini->ReadBool  (rules, "Arcing", Arcing);
    Floater        = ini->ReadBool  (rules, "Floater", Floater);
    SubjectToCliffs    = ini->ReadBool(rules, "SubjectToCliffs", ...);
    SubjectToElevation = ini->ReadBool(rules, "SubjectToElevation", ...);
    SubjectToWalls = ini->ReadBool  (rules, "SubjectToWalls", ...);
    VeryHigh       = ini->ReadBool  (rules, "VeryHigh", VeryHigh);
    Shadow         = ini->ReadBool  (rules, "Shadow", Shadow);
    Dropping       = ini->ReadBool  (rules, "Dropping", Dropping);
    Level          = ini->ReadBool  (rules, "Level", Level);
    Inviso         = ini->ReadBool  (rules, "Inviso", Inviso);
    Proximity      = ini->ReadBool  (rules, "Proximity", Proximity);   // dead — see §3.3
    Ranged         = ini->ReadBool  (rules, "Ranged", Ranged);
    Inaccurate     = ini->ReadBool  (rules, "Inaccurate", Inaccurate);
    FlakScatter    = ini->ReadBool  (rules, "FlakScatter", FlakScatter);
    AA             = ini->ReadBool  (rules, "AA", AA);
    AG             = ini->ReadBool  (rules, "AG", AG);
    Degenerates    = ini->ReadBool  (rules, "Degenerates", Degenerates);
    Bouncy         = ini->ReadBool  (rules, "Bouncy", Bouncy);
    Airburst       = ini->ReadBool  (rules, "Airburst", Airburst);
    Cluster        = ini->ReadInt   (rules, "Cluster", Cluster);
    Scalable       = ini->ReadBool  (rules, "Scalable", Scalable);

    // Image= → enables art-section reads
    int n = ini->ReadString(rules, "Image", "", art_section, 25);
    if (n > 0) {
        // Trailer= → AnimType lookup
        char buf[128];
        if (ini->ReadString(art_section, "Trailer", "", buf, 128) != 0)
            Trailer = AnimTypeClass::FindOrAllocate(buf);  // corrected 2026-05-28: was FindByName; binary shows AnimTypeClass__FindOrAllocate via decompile_function 0x0046BEE0 — ROOT_CAUSE: RTTI_LABEL_DRIFT
        SpawnDelay = ini->ReadInt(art_section, "SpawnDelay", SpawnDelay);

        // Rotates= is INVERTED:
        //   default-passed-in   = !Rotates_field
        //   read-result-stored  = !ReadBool(...)
        bool r = ini->ReadBool(art_section, "Rotates", Rotates_field == 0);
        Rotates_field = (r == 0);

        Flat = ini->ReadBool(art_section, "Flat", Flat);
    }

    // AirburstWeapon=
    if (ini->ReadString(rules, "AirburstWeapon", "", buf, 128) != 0)
        AirburstWeapon = WeaponTypeClass::FindOrAllocate(buf);

    // ShrapnelWeapon=
    if (ini->ReadString(rules, "ShrapnelWeapon", "", buf, 128) != 0)
        ShrapnelWeapon = WeaponTypeClass::FindOrAllocate(buf);

    ShrapnelCount      = ini->ReadInt(rules, "ShrapnelCount", ShrapnelCount);
    DetonationAltitude = ini->ReadInt(rules, "DetonationAltitude", DetonationAltitude);
    Vertical           = ini->ReadBool(rules, "Vertical", Vertical);
    FirersPalette      = ini->ReadBool(rules, "FirersPalette", FirersPalette);

    // Animation byte-fields (ReadInt result truncated to byte on store):
    AnimLow  = (byte)ini->ReadInt(art_section, "AnimLow",  AnimLow);
    AnimHigh = (byte)ini->ReadInt(art_section, "AnimHigh", AnimHigh);
    AnimRate = (byte)ini->ReadInt(art_section, "AnimRate", AnimRate);
    AnimPalette = (byte)ini->ReadBool(art_section, "AnimPalette", AnimPalette);

    // Final: load SHP image if not Inviso
    if (Inviso == 0)
        FUN_005F9070(this);   // SHP/file-from-MIX loader (theater-aware)

    // Optional CD-file backing (inherited flag at +0x236)
    if (this->field_0x236 != 0)
        CDFileClass::Constructor(...);

    return true;
}
```

### 4.2 Complete INI key table — verified

| # | INI Key | Section | Read Fn | Offset | Type |
|---|---------|---------|---------|--------|------|
| 1 | `Arm`                | rules | ReadInt    | 0x2F0 | int |
| 2 | `ROT`                | rules | ReadInt    | 0x2DC | int |
| 3 | `CourseLockDuration` | rules | ReadInt    | 0x2E0 | int |
| 4 | `Elasticity`         | rules | ReadDouble | 0x2C8 | double |
| 5 | `Acceleration`       | rules | ReadInt    | 0x2D0 | int |
| 6 | `Color`              | rules | ReadColor  | 0x2D4 | int (RGB) |
| 7 | `Arcing`             | rules | ReadBool   | 0x29B | bool |
| 8 | `Floater`            | rules | ReadBool   | 0x295 | bool |
| 9 | `SubjectToCliffs`    | rules | ReadBool   | 0x296 | bool |
| 10 | `SubjectToElevation`| rules | ReadBool   | 0x297 | bool |
| 11 | `SubjectToWalls`    | rules | ReadBool   | 0x298 | bool |
| 12 | `VeryHigh`          | rules | ReadBool   | 0x299 | bool |
| 13 | `Shadow`            | rules | ReadBool   | 0x29A | bool |
| 14 | `Dropping`          | rules | ReadBool   | 0x29C | bool |
| 15 | `Level`             | rules | ReadBool   | 0x29D | bool |
| 16 | `Inviso`            | rules | ReadBool   | 0x29E | bool |
| 17 | `Proximity`         | rules | ReadBool   | 0x29F | bool **(dead — §3.3)** |
| 18 | `Ranged`            | rules | ReadBool   | 0x2A0 | bool |
| 19 | `Inaccurate`        | rules | ReadBool   | 0x2A2 | bool |
| 20 | `FlakScatter`       | rules | ReadBool   | 0x2A3 | bool |
| 21 | `AA`                | rules | ReadBool   | 0x2A4 | bool |
| 22 | `AG`                | rules | ReadBool   | 0x2A5 | bool |
| 23 | `Degenerates`       | rules | ReadBool   | 0x2A6 | bool |
| 24 | `Bouncy`            | rules | ReadBool   | 0x2A7 | bool |
| 25 | `Airburst`          | rules | ReadBool   | 0x294 | bool |
| 26 | `Cluster`           | rules | ReadInt    | 0x2AC | int |
| 27 | `Scalable`          | rules | ReadBool   | 0x2EC | bool |
| 28 | `Image`             | rules | ReadString | (+0x1F8 inherited buffer) | char[25] |
| 29 | `Trailer`           | art   | ReadString → AnimTypeClass::FindOrAllocate | 0x2D8 | AnimTypeClass* | (corrected 2026-05-28: was FindByName; binary shows FindOrAllocate via decompile_function 0x0046BEE0 — ROOT_CAUSE: RTTI_LABEL_DRIFT) |
| 30 | `SpawnDelay`        | art   | ReadInt    | 0x2E4 | int |
| 31 | `Rotates`           | art   | ReadBool **(inverted storage)** | 0x2A1 | bool |
| 32 | `Flat`              | art   | ReadBool   | 0x2F7 | bool |
| 33 | `AirburstWeapon`    | rules | ReadString → WeaponTypeClass::FindOrAllocate | 0x2B0 | WeaponTypeClass* |
| 34 | `ShrapnelWeapon`    | rules | ReadString → WeaponTypeClass::FindOrAllocate | 0x2B4 | WeaponTypeClass* |
| 35 | `ShrapnelCount`     | rules | ReadInt    | 0x2B8 | int |
| 36 | `DetonationAltitude`| rules | ReadInt    | 0x2BC | int |
| 37 | `Vertical`          | rules | ReadBool   | 0x2C0 | bool |
| 38 | `FirersPalette`     | rules | ReadBool   | 0x2A9 | bool |
| 39 | `AnimLow`           | art   | ReadInt → byte | 0x2F4 | byte |
| 40 | `AnimHigh`          | art   | ReadInt → byte | 0x2F5 | byte |
| 41 | `AnimRate`          | art   | ReadInt → byte | 0x2F6 | byte |
| 42 | `AnimPalette`       | art   | ReadBool   | 0x2A8 | bool |

**Total: 41 BulletType-specific keys + `Image=` reference.**

### 4.3 Inverted Rotates storage — exact mechanic

```c
// In ReadINI:
char default_to_pass = (Rotates_field_at_+0x2A1 == 0);   // pass !field as default
char read_result = ini->ReadBool(art, "Rotates", default_to_pass);
Rotates_field_at_+0x2A1 = (read_result == 0);            // store !result
```

This double-negation has a real purpose: it makes the constructor's default of `1` mean "Rotates=no" (semantically), which matches the engine's behavior — most projectiles do NOT rotate (e.g., flat sprites). When INI sets `Rotates=yes`, the field becomes `false`, which the renderer reads as "do rotate."

Practically: the in-memory field name is **NotRotates** in spirit, even though Ghidra/labels call it `Rotates`. Treat it as inverted whenever reading or porting.

---

## 5. THE CORRECTION — there is NO `BulletTypeClass::ReadINI_Part2`

`BULLET_PROJECTILE_SYSTEM_CONSOLIDATED_REPORT.md` §3.3 documents a "second-pass reader" at `0x00428319` named `BulletTypeClass__ReadINI_Part2` and lists a long table of additional keys (`StartSound`, `StopSound`, `BounceAnim`, `ExpireAnim`, `TrailerAnim`, `TrailerSeperation`, `DamageRadius`, `Warhead`, `Bouncer`, `Tiled`, `ShouldUseCellDrawer`, `UseNormalLight`, `SpawnsParticle`, `NumParticles`, `RandomRate`, `YDrawOffset`, `ZAdjust`) supposedly stored at offsets +0x2F8 through +0x35D on the BulletType.

**This is wrong. All of those keys belong to `AnimTypeClass`, not `BulletTypeClass`.**

Evidence:

1. **The "function" at `0x00428319` is mid-stream code inside `AnimTypeClass::ReadINI`.** Calling `get_function_by_address(0x00428300)` returns `AnimTypeClass__ReadINI` with body `00427d00 - 004287f5`, which fully **encloses** the `0x00428319 - 004287e8` range that Ghidra mislabeled as a separate function. Ghidra created a phantom function entry-point because the address has no valid prologue — the decompilation shows `unaff_ESI` and `unaff_EDI` at function entry (a Ghidra tell that ESI/EDI were set by the enclosing function before this point).

2. **The function has zero callers and zero xrefs.** `get_xrefs_to(0x00428319)` and `get_function_callers(0x00428319)` both return empty. A real ReadINI follow-up would be called from somewhere — minimally from a vtable or a wrapper. It is not.

3. **BulletTypeClass instance size is 0x2F8.** Verified via `push 0x2F8 ; call operator_new` at `0x006C2E30` immediately preceding the constructor invocation. The phantom Part2 writes to offsets +0x300, +0x304, +0x308, +0x30C, +0x330, +0x334, +0x344, +0x348, +0x35A-+0x35D — all **out of bounds** for a 760-byte BulletType instance. They land safely inside an AnimTypeClass instance, which is **0x378 (888) bytes** (verified via the `operator_new(0x378)` calls inside the same function at lines for BounceAnim/ExpireAnim/TrailerAnim).

4. **The keys listed match AnimTypeClass behavior exactly.** The decompiled comment header on the real `AnimTypeClass::ReadINI` at `0x00427D00` reads:
    > Key fields: Rate(0x2B0)=900/INI_Rate, Start(0x2B4), End(0x2C0), LoopStart(0x2B8), LoopEnd(0x2BC), LoopCount(0x2C4), Next(0x2C8), Layer(0x364), Flat(0x369), Translucent(0x36A), Shadow(0x372), YDrawOffset(0x344), ZAdjust(0x348). AnimTypeClass size = 0x378 bytes.

   The +0x344/+0x348 offsets line up with `YDrawOffset`/`ZAdjust` in the AnimType, which the phantom function also writes to.

### 5.1 Practical impact

- **`BulletTypeClass` does not parse `StartSound`, `StopSound`, `BounceAnim`, `ExpireAnim`, `TrailerAnim`, `TrailerSeperation`, `DamageRadius`, `Warhead` (on bullet), `Bouncer`, `Tiled`, `ShouldUseCellDrawer`, `UseNormalLight`, `SpawnsParticle`, `NumParticles`, `RandomRate`, `YDrawOffset`, or `ZAdjust`.**
- These keys appear in `art(md).ini` under animation sections (e.g., `[DRAGON]`, `[NUKEPUFF]`) and are read by `AnimTypeClass::ReadINI` against the AnimType pointed to by the bullet's `Image=`. In other words: when a BulletType has `Image=DRAGON`, the **DRAGON AnimType** carries those properties, not the BulletType.
- The consolidated report's §3.4 paragraph about "RandomRate=Min,Max" being read into BulletType +0x2E4/+0x2E8 is similarly wrong — it's read into the **AnimType**'s +0x2E4/+0x2E8 (which are `param_1[0xB9]` and `param_1[0xBA]` in AnimType where `param_1` is `int *`, i.e., byte offsets 0x2E4 and 0x2E8 in the AnimType layout, **distinct fields from the same byte offsets in BulletType**).
- BulletType's own `+0x2E4` (`SpawnDelay`) is set by `ReadINI` directly from `art_section.SpawnDelay=` as raw ticks. BulletType's own `+0x2E8` is **never written by any ReadINI path** — only zeroed by the constructor, then read by `BulletClass::AI` when computing trailer cadence.

### 5.2 What `BulletClass::AI` actually reads for trailer cadence

The cadence formula in `BulletClass::AI` is the one stated in the consolidated report:

```c
if (BulletType.Trailer != NULL) {                   // BulletType+0x2D8
    if (BulletType.field_0x2E8 == 0) {
        if (g_FrameCounter % BulletType.SpawnDelay /* +0x2E4 */ == 0)
            spawn_trailer_anim();
    } else {
        if (g_FrameCounter % BulletType.field_0x2E8 == 0)
            spawn_trailer_anim();
    }
}
```

Because BulletType+0x2E8 is **never written** by any reader, the second branch is permanently dead in current YR data — every BulletType uses the +0x2E4 (SpawnDelay) cadence. The +0x2E8 field exists structurally (constructor zeros it) but no INI key reaches it on BulletType. The two-tier RandomRate cadence applies to **AnimType** trail spawn behavior (and only when an AnimType is independently spawned through its own anim system), not to BulletType trail spawning.

This means in a port, BulletType.SpawnDelay (+0x2E4) is the single trailer cadence source for projectiles. The +0x2E8 slot can be omitted from the BulletType struct entirely.

### 5.3 Other corrections to the consolidated report

- §10 "Open questions" item 1 ("TrailerSeperation reader") — `TrailerSeperation=` is **not a BulletType key** either. It's an AnimType key at AnimType+0x30C. So `TrailerSeperation=` on a bullet's section in `rulesmd.ini` does nothing; on its `Image=` art section, it's read into the AnimType. The consolidated report's Step 1 conclusion ("dead at runtime") is right for the wrong reason.

---

## 6. The destructor — `BulletTypeClass::Destructor` @ 0x0046BE10

Ghidra labels `0x0046BE10` as `BulletTypeClass__ReadINI_wrapper`, but the body shows otherwise:

1. Re-writes vtable pointers (typical destructor pattern to make sure dtor calls dispatch to BulletType vtable, not a derived class).
2. Calls `FUN_007258d0` (likely a string-buffer cleanup).
3. **Removes `this` from the global instance list** at `DAT_00B0F674` (the same list Constructor pushed into) by linear search-and-shift.
4. **Removes `this` from the global class list** at `DAT_00A83C84` (same as 3).
5. Calls `ObjectTypeClass::Constructor` — almost certainly mislabeled by Ghidra; this is `ObjectTypeClass::Destructor` chaining up.

There is no actual ReadINI wrapper function — `BulletTypeClass::ReadINI` at `0x0046BEE0` is dispatched directly through `vtable__BulletTypeClass[11]` (offset 0x2C). When the rules-loader iterates projectile sections, it calls the ReadINI vtable slot on each newly-constructed BulletType instance.

---

## 7. Integration points

**Construction path (rules.ini load):**
1. Caller wants a new BulletType: `operator_new(0x2F8)` (at `0x006C2E30` in COM CreateInstance).
2. `BulletTypeClass::Constructor` runs — base class chains, defaults written, vtables set, registered in two global lists at `DAT_00A83C84` (class-list) and `DAT_00B0F674` (instance-list).
3. Rules loader resolves the next `[Section]` in projectile-list and dispatches `vtable[11]` → `BulletTypeClass::ReadINI`.
4. `ReadINI` calls `ObjectTypeClass::ReadINI` first (parses inherited `Name=`, `Image=`, palette keys), then reads the 41 bullet-specific keys.
5. If `Inviso=no`, the SHP file is loaded via `FUN_005F9070` (theater-aware MIX lookup).

**Lookup at runtime:**
- `WeaponTypeClass::ReadINI` (caller `0x00772990`) reads the `Projectile=` key on each weapon and resolves it to a BulletType via the global list.
- `BulletClass::Init` (`0x004664C0`) writes the resolved `BulletTypeClass *` into `BulletClass+0xAC`.
- Every `BulletType.Field` reference inside `BulletClass::AI`, `BulletClass::Fire`, `BulletClass::BulletDetonation`, `BulletClass::BounceCheck` dereferences through `BulletClass+0xAC`.

**No per-tick logic on `BulletTypeClass` itself.** It is pure configuration; all behavior is in `BulletClass`.

---

## 8. Current Rust implementation status

`src/rules/projectile_type.rs` parses 37 fields. Coverage matches the verified key set very well, with these specific gaps and bugs:

### 8.1 Field naming bug (mislabeled `speed`)

`projectile_type.rs:47-49` declares:

```rust
/// Binary offset: +0x2F0 (labeled "Arm" in the binary, read via "Speed" key)
pub speed: i32,
```

This is wrong on every count:
- The INI key is `Arm=`, not `Speed=`.
- Field semantics is **arming delay (ticks) for ProximityDetector**, not speed.
- BulletType has no speed field at all; projectile speed comes from `WeaponType.Speed` and is set into `BulletClass+0x110` (`TargetSpeed`) at `BulletClass::Init`.

Fix: rename `speed` → `arm`, update the comment (drop the "via Speed key" claim entirely), and audit any consumers expecting projectile-level speed here.

### 8.2 Default values that need to match the binary

The Rust `Default::default()` produces zeros for every field, but the constructor at `0x0046BBC0` writes these non-zero defaults:

| Field | Binary default | Rust default (current) |
|-------|----------------|------------------------|
| `shadow`        | true   | false |
| `ag`            | true   | (per scan, true — OK) |
| `cluster`       | 1      | 0     |
| `acceleration`  | 3      | 0     |
| `spawn_delay`   | 3      | 0     |
| `elasticity`    | 0.75   | 0.0   |
| `rotates` (semantic) | "Rotates=no" (renderer reads as: do not rotate) | true (would mean "do rotate" — opposite) |

If these defaults remain zero, any BulletType that omits a key in INI behaves differently from gamemd.exe. For weapons that rely on `Cluster=1` implicit default (e.g., a default cluster loop running once, not zero times), the engine would silently produce zero-detonation bullets.

### 8.3 Keys correctly NOT parsed

The following keys appear in some `art(md).ini` sections that BulletType uses for `Image=`, but are **not** BulletType keys (they belong to AnimType, parsed separately when the AnimType is loaded):

- `StartSound`, `Report`, `StopSound`
- `BounceAnim`, `ExpireAnim`, `TrailerAnim`
- `TrailerSeperation` (note the misspelling — reproduce verbatim)
- `DamageRadius`
- `Warhead` (on the BulletType — distinct from WeaponType.Warhead)
- `Bouncer`, `Tiled`, `ShouldUseCellDrawer`, `UseNormalLight`
- `SpawnsParticle`, `NumParticles`
- `RandomRate`
- `YDrawOffset`, `ZAdjust`

The Rust impl correctly omits these from `ProjectileType`. They should land on the AnimType when AnimType parsing is added (which is a separate task).

### 8.4 Runtime simulation gap (unchanged from consolidated report)

`BulletTypeClass` parsing is feature-complete-ish, but no Rust code spawns a `BulletClass`-equivalent runtime entity. `src/sim/combat/mod.rs` applies damage instantly; `src/sim/movement/rocket_movement.rs` has a parabolic arc state machine but is not wired to weapon fire. To hit the 99% parity bar, every non-Inviso projectile needs a real per-tick flight phase consuming the parsed BulletType properties.

---

## 9. Open questions

1. **`field_0x2E8` purpose.** Constructor zeros it; no `ReadINI` path writes it on BulletType; `BulletClass::AI` reads it. Possibilities: (a) reserved for future use and stayed dead; (b) overwritten by an unrelated patch I missed in the constructor decomp; (c) computed at runtime by some Init helper. Worth a targeted byte-pattern search for `8B 8? E8 02 00 00` on BulletType-range writers.
2. **`+0x236` flag (inherited).** Triggers `CDFileClass::Constructor` at the end of `ReadINI`. Probably an ObjectTypeClass "load from CD" flag; not bullet-relevant but worth a one-line note in the ObjectTypeClass report when written.
3. **AnimTypeClass full struct layout.** Now flagged as a follow-up since the consolidated report's §3.3 keys belong here. The AnimType section in `BULLET_PROJECTILE_SYSTEM_CONSOLIDATED_REPORT.md` would need a dedicated report — out of scope here.
4. **`Color=` parser (`FUN_00474A90`).** Format is reportedly RGB packed but not re-decompiled here. Worth a quick verification that the byte order matches the renderer's expectation.

---

## Sources

### Ghidra addresses decompiled
- `BulletTypeClass::Constructor` @ `0x0046BBC0`
- `BulletTypeClass::ReadINI` @ `0x0046BEE0`
- `BulletTypeClass::Destructor` (Ghidra-labeled "ReadINI_wrapper") @ `0x0046BE10`
- `AnimTypeClass::ReadINI` @ `0x00427D00` (verified the phantom function at `0x00428319` is mid-stream here, not a separate function)
- `ObjectTypeClass::Constructor` @ `0x005F7090`
- `FUN_005F9070` (SHP loader) @ `0x005F9070`
- COM `operator_new(0x2F8)` site @ `0x006C2E30`

### Memory inspections
- BulletTypeClass primary vtable @ `0x007E4980` (slot 11 = `0x0046BEE0` = ReadINI)
- INI key strings: `0x0081B098` ("AG"), `0x0081B09C` ("AA"), `0x0081B164` ("ROT"), `0x0081B168` ("Arm")

### Doc files cross-referenced
- `BULLET_PROJECTILE_SYSTEM_CONSOLIDATED_REPORT.md` (§3, §4 verified; §3.3 corrected)
- `BULLET_CLASS_LAYOUT_GHIDRA_REPORT.md` (BulletClass instance layout — sister doc, kept as-is)

### INI files checked
- `c:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini` — `[Projectiles]` section + 56 BulletType section bodies
- `c:/Users/enok/Documents/ra2-rust-game/ini/artmd.ini` — sample art-section bodies for bullet `Image=` references
