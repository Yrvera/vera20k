---
name: SideClass (gamemd.exe)
date: 2026-04-19
related: COUNTRY_SIDE_TYPE_CLASSES.md (2026-03-22) — extends and corrects
---

# SideClass — Ghidra Research Report

**Primary address:** `0x006a4550` (constructor, **mislabeled in Ghidra as `SidebarClass__Constructor`**)
**Size:** `0xB4` bytes (180 bytes)
**Confidence:** HIGH (all core paths decompiled and verified)
**Active in YR:** Yes — loaded at rules parse time, consulted at runtime for the Civilian special case.

This report focuses on **SideClass** specifically. For the adjacent CountryTypeClass / HouseTypeClass, see
`COUNTRY_SIDE_TYPE_CLASSES.md` (which this report extends and corrects in two places).

---

## 1. Overview

SideClass is a grouping class: one instance per `[Sides]` entry in `rulesmd.ini` (e.g. `GDI`, `Nod`,
`ThirdSide`, `Civilian`, `Mutant`). It owns (a) a name string and (b) a `DynamicVectorClass<int>` of
country indices that belong to that side. A country's side is represented at runtime by the
integer index of its SideClass in the global array `DAT_008b4124`.

The class inherits from AbstractClass (4-vtable multi-inheritance) but has **no custom ReadINI,
Save, or Load**. Its lifecycle is driven entirely by:

1. The `[Sides]` registration pass (`FUN_00672440`), which populates the country list.
2. `HouseTypeClass::ReadINI`, which can **retroactively override** a country's side via the
   per-country `Side=` key (see §5 — this is a correction to the prior report).

At runtime, only one side name is looked up by name: **`"Civilian"`** (6 distinct call sites).

---

## 2. Class Layout (0xB4 bytes, verified from `0x006a4550`)

| Offset | Size | Field | Notes |
|-------:|-----:|-------|-------|
| `+0x00` | 16 | 4 vtable pointers | `vtable__SideClass` + 3 secondaries (AbstractTypeClass base) |
| `+0x10` | 20 | AbstractClass base fields | unique-ID, ref count, RTTI id — standard |
| `+0x24` | 64 | `char name[64]` | side name, e.g. `"GDI"`, `"Civilian"` |
| `+0x64` | 52 | (unused for Side) | UIName slot from AbstractTypeClass; not set by constructor |
| `+0x98` | 4 | DVC vtable ptr | set to `&PTR_FUN_007e4dd8` |
| `+0x9C` | 4 | DVC data ptr | array of country indices (int) |
| `+0xA0` | 4 | DVC capacity | initial `0` |
| `+0xA4` | 1 | DVC owns_memory | initial `1` |
| `+0xA5` | 1 | padding | zero-initialized |
| `+0xA6` | 2 | padding | |
| `+0xA8` | 4 | DVC count | number of countries on this side |
| `+0xAC` | 4 | DVC grow_size | initial `10` |
| `+0xB0` | 4 | padding/alignment | zero-initialized |

Layout derived from constructor assignments: `param_1[0x26]=0` (DVC vtable), `param_1[0x27]=0`
(data), `param_1[0x28]=0` (capacity), `*(byte)(param_1+0xa4)=1`, `*(byte)(param_1+0xa5)=0`,
`param_1[0x2a]=0` (count), `param_1[0x2b]=10` (grow).

---

## 3. Global Array (VectorClass<SideClass*>)

| Address | Meaning |
|---------|---------|
| `DAT_008b4120` | VectorClass vtable ptr |
| `DAT_008b4124` | **data pointer** — `SideClass**` array |
| `DAT_008b4128` | capacity |
| `DAT_008b412d` | owns_memory byte |
| `DAT_008b4130` | **count** — side count |
| `DAT_008b4134` | grow_size |

A SideClass's integer "side index" is its position in this array, set implicitly by insertion
order in the `[Sides]` section (see §4).

---

## 4. Key Functions

### 4.1 `SideClass::Constructor` — `0x006a4550` (mislabeled as `SidebarClass__Constructor`)

```
calls AbstractTypeClass__Constructor
zeroes DVC fields (+0x98..+0xAC), owns=1, grow=10
sets vtable__SideClass + 3 secondaries
calls AssignUniqueID
appends `this` to global array via VectorClass grow-on-full logic
```

**Naming caveat:** Ghidra has this labeled `SidebarClass__Constructor` alongside two other
functions with the same name. The decompilation clearly writes `&vtable__SideClass` — this is
SideClass, not SidebarClass. Consider renaming.

### 4.2 `SideClass::FindByName(name)` — `0x006a46d0`

```c
int FindByName(char* name) {
    for (int i = 0; i < DAT_008b4130; i++) {
        if (stricmp(array[i] + 0x24, name) == 0) return i;
    }
    return -1;
}
```

Linear scan comparing `side->name` (offset +0x24). Uses `FUN_007c8d20` which is a case-insensitive
string compare. Returns the side index or `-1`.

**Callers (9 total via `get_xrefs_to`):**

- Registration/rules paths (4): `FUN_00672440` (the [Sides] parser), `FUN_004756f0` (the per-country
  `Side=` reader, §5), `FUN_004767c0` (DVC init helper).
- Runtime paths (6): all decompiled (§6). All pass the literal string address
  `0x00818164 = "Civilian"`.

### 4.3 `[Sides]` Registration — `FUN_00672440`

For each entry in the `[Sides]` section:

```
key = INI entry key (e.g. "GDI")
idx = SideClass::FindByName(key)
if idx == -1:
    allocate 0xB4, call SideClass::Constructor(key)
    (newly added to global array via constructor)
re-initialize side->countries DVC at +0x98  (FUN_00477b60 — clears existing list)
parse entry VALUE (comma-separated country names) -> DVC of country indices
  (each name resolved via CountryTypeClass::FindOrCreate)
copy parsed DVC's count/grow/owns into side->+0xA8/+0xAC/+0xB0
for each country index in list:
    find *this side's* own index in the global array
    set CountryType[country_idx]+0xBC = that index
emits debug log "Side %d: %s" then per-country "  %s"
```

**Idempotency detail:** when `FindByName` hits an existing SideClass, the country-list DVC is
**wiped and rebuilt** from the current value. This means a repeated `[Sides]` section (e.g. map
override) replaces — does not append to — the country list.

### 4.4 `SideClass::FindOrCreate_FromKey(section, key, default_idx)` — `FUN_004756f0`

```c
int FindOrCreate_FromKey(section, key_name, default) {
    char buf[128];
    if (!CCINIClass::ReadString(section, key_name, "", buf, 128))
        return default;       // key missing → keep default
    int idx = FindByName(buf);
    if (idx != -1) return idx;
    // Key present but name unknown → allocate and register a new SideClass
    operator new(0xB4);
    SideClass::Constructor(buf);
    // re-scan global array for our new pointer's index
    return idx_of_new_side;
}
```

Used by HouseTypeClass::ReadINI (with `key_name = "Side"`) — see §5. Note: reading a
previously-unknown side name **creates a new SideClass on the fly**. This is why a typo in
`Side=` won't crash — it silently instantiates an extra side with that name.

---

## 5. **CORRECTION to prior doc:** `Side=` on a country is parsed and can override `[Sides]`

The prior report (`COUNTRY_SIDE_TYPE_CLASSES.md`, §5 and §"Flow") states that the per-country
`Side=` key is "legacy/unused" and that the side index at `+0xBC` is set only by the `[Sides]`
parser. **That is wrong.** Decompilation of `HouseTypeClass::ReadINI` (`0x00511850`) shows the
following at the tail, after all multipliers and veteran lists:

```c
int old_side = this->side_index;                                   // +0xBC, set by [Sides]
int new_side = FUN_004756f0(this + 0x24, "Side", old_side);         // reads per-country Side=
this->side_index = new_side;
if (new_side != old_side) {
    if (old_side != -1) {
        // Remove this country's index from old_side's country-list DVC at +0x98..+0xB0
        // (shift-down compaction at +0x9c data, decrement +0xa8 count)
    }
    if (new_side != -1) {
        // Append to new_side's country-list DVC (with grow-on-full logic)
    }
}
```

Execution order at rules load (from `FUN_00668bf0`):

1. `[Countries]` — register HouseTypeClass instances.
2. `[Sides]` — register SideClass instances, parse country lists, write `HouseType+0xBC`.
3. For each HouseType, vtable-dispatch `ReadINI` → reads per-country `Side=` → **overrides
   +0xBC and rewrites both the old and new side's country-list DVC**.

In vanilla YR the two agree, so this override is invisible in a standard game. It matters for:

- **Mods** where `[Sides]` and per-country `Side=` disagree (the per-country key wins).
- **Maps/mods with unknown side names** in `Side=` — a phantom SideClass is created.

Also noteworthy: `HouseTypeClass::ReadINI` itself does **not** read an `ArmorDefensesMult` or
`IncomeMult` field in the order the prior doc implies — it does read them, but the strings
appear in sequence with `DAT_*` placeholders; spot-checking is worthwhile if fine-grained
multiplier ordering matters.

---

## 6. Runtime use: all 6 non-registration callers look up `"Civilian"`

All runtime `FindByName` sites load `ECX = 0x00818164` before the call. That address decodes to
the bytes `53 69 64 65 00` ... `43 69 76 69 6C 69 61 6E 00` — the ASCII strings `"Side"` then
`"Civilian"` sit adjacent; the call-sites pass `"Civilian"` (start of the second string).

Verified sites:

| Caller | Site | Purpose |
|--------|-----:|---------|
| `HouseClass::Is_Enemy` | `0x00501581` | In MP (`g_GameMode != 0`), if the *other* house is on Civilian side, never hostile |
| `BuildingClass::CheckAutoSellOrCivilian` | `0x0045823d` | Find the Civilian house in `g_HouseClass_Array` to transfer ownership of an abandoned/auto-sell building |
| `CaptureManagerClass::SetOriginalOwner` | `0x00472341` | Find Civilian house to reassign mind-control "original owner" when the real owner is gone |
| `AnimClass::AI` | `0x004249a7` | When an anim without an owner finishes, locate the Civilian house to credit destruction / ownership |
| `FUN_0041ec90` | `0x0041ecb5` | Trigger event evaluator — resolves "Civilian's house index" for comparison-style TEvents |
| `FUN_006b0ae0` | `0x006b0b0b` | Cleanup / ownership fallback (context unclear without further dive) |

**Implication:** only `"Civilian"` is load-bearing at runtime in compiled code. The names
`GDI`, `Nod`, `ThirdSide`, `Mutant` (and any mod names) exist solely in INI and are referenced
*by index*, never by name, after rules load. This is why renaming the other sides in INI is
safe but renaming/removing `"Civilian"` breaks the engine.

### 6.1 `HouseClass::Is_Enemy` — the civilian rule

```c
bool Is_Enemy(this, other) {
    if (other == NULL) goto civilian_check;
    if (other == this) return false;
    if (allied via color/alliance bitmask) return false;
civilian_check:
    int civilian_idx = SideClass::FindByName("Civilian");
    if (other->HouseType->side_index == civilian_idx && g_GameMode != 0)
        return false;                  // MP: civilian is never enemy
    if (g_MapEditorMode) return true;
    // ... remainder: enumerate active non-passive houses, compare team indices
}
```

`g_GameMode != 0` means any multiplayer / skirmish mode. In single-player (`== 0`) the Civilian
check is skipped, and enemy status is decided purely by alliance bitmask — this is why campaign
civilian units can be attacked (they're aligned to whichever scenario side the map chose).

---

## 7. INI surface (cross-check with vanilla `rulesmd.ini`)

```
[Sides]
GDI=British,French,Germans,Americans,Alliance       ; side 0 — Allied
Nod=Russians,Africans,Confederation,Arabs           ; side 1 — Soviet
ThirdSide=YuriCountry                               ; side 2 — Yuri (YR-only)
Civilian=Neutral                                    ; side 3 — Civilian
Mutant=Special                                      ; side 4 — Mutant (TS holdover)
```

| Side name | Active in YR? | Notes |
|-----------|---------------|-------|
| GDI | Yes | Allied playable faction |
| Nod | Yes | Soviet playable faction |
| ThirdSide | Yes (YR adds it — absent from RA2 base) | Yuri playable faction |
| Civilian | Yes — **load-bearing** | Looked up by name at runtime (§6) |
| Mutant | No (TS holdover) | One country (`Special`). No direct code reference found. Safe to preserve but not required. |

The vanilla per-country `Side=` key always matches the `[Sides]` listing, so the override
machinery in §5 is latent in stock play.

---

## 8. What the engine does NOT read from SideClass

- No custom `Save`/`Load` (uses AbstractClass base serialization).
- No custom `ReadINI` (construction is done by the `[Sides]` parser, not by vtable-dispatched
  ReadINI).
- No CRC contribution observed via `ComputeCRC` — the global string `"*************** Sides
  CRCs**************"` at `0x00838008` suggests a debug/logging print but is not wired to the
  sync hash in the normal path I traced.
- No rendering/audio path reads the side name. Audio/art selection is done through
  HouseTypeClass fields (`Prefix`, `Suffix`, `Color`) or via hardcoded per-side keys on
  RulesetClass (`AmerParaDropInf`, `AllyParaDropInf`, etc. — see the prior doc §3).

---

## 9. Rust implementation status

Current state (verified by `Explore` agent on `src/`):

- **No `SideClass` / no `Side` enum for game mechanics.**
- `HouseDefinition.side: Option<String>` in `src/map/houses.rs:34` is parsed from the **map**
  `[Houses]` section's `Side=` key (line 142), not from `rulesmd.ini [Sides]`.
- `HouseState.side_index: u8` in `src/sim/house_state.rs:21` — 0/1/2 for Allied/Soviet/Yuri.
- `side_index_from_name()` in `src/sim/house_state.rs:114` — hardcoded string match
  (`"allied|allies|gdi"→0`, `"soviet|nod|russia"→1`, `"thirdside|yuricountry|yuri"→2`, other→0).
- `SidebarTheme::{Allied, Soviet, Yuri}` in `src/render/sidebar_chrome.rs:35` — UI-only, not
  gameplay.
- `rulesmd.ini [Sides]` section is **not parsed**.
- `Civilian` and `Mutant` sides are **not modeled at all**.

Gaps vs. gamemd.exe:

1. No SideType registry — sides are hardcoded 0/1/2 rather than derived from `[Sides]` order.
2. No Civilian side handling — which means `HouseClass::Is_Enemy` semantics (civilian is
   never-hostile in MP) and the Civilian-house-transfer fallback used by
   `BuildingClass::CheckAutoSellOrCivilian` / `CaptureManagerClass::SetOriginalOwner` /
   `AnimClass::AI` are not replicable as written.
3. Per-country `Side=` override of `[Sides]` (§5) is not modeled — irrelevant for vanilla but
   relevant if mods are eventually supported.
4. The engine treats the side's INTEGER INDEX as the identity. If Rust replicates this, it
   must preserve insertion order from `[Sides]` — not use a hardcoded enum.

---

## 10. Follow-up deep dive (2026-04-19)

After the initial pass, four of the open questions resolved:

### 10.1 `FUN_006b0ae0` — generic **container-eject** helper

Callers confirm its role: `TechnoClass::ReceiveDamage`, `TeleportLocomotionClass::PostWarpValidation`,
`TemporalClass::Update`, `MissionClass::Constructor` (ctor-label is suspect, likely mislabeled —
the logic is mid-flight cleanup), `PowerUp_Cleanup`, plus one unknown (`FUN_0054ca90`).

It receives a container struct (`param_1`) whose layout is:
- `+0x24` — "is active" flag; the function early-returns if zero, and clears it at the end.
- `+0x3c` — data pointer for an array of contained objects.
- `+0x48` — count.

Pseudocode:

```c
void EjectContainerContents(container* c, TechnoClass* new_owner, HouseClass* dest_house) {
    if (c->active == 0) return;
    int civ_idx = SideClass::FindByName("Civilian");
    HouseClass* civilian = find first house where HouseType+0xBC == civ_idx;

    for (int i = c->count - 1; i >= 0; i--) {
        obj = (*(c->data + i*4))[0];    // double-deref
        if (!obj || !g_GameActive) continue;
        obj[0xb7] = 0;
        if (obj[0x81]) {                 // special-flagged object → destroy directly
            obj->vtbl[0xe0](new_owner);  // some damage/kill call
            obj->vtbl[0xf8]();           // finalize
            continue;
        }
        HouseClass* target;
        if (new_owner) {
            target = new_owner->+0x21c;  // owner of the attacker
        } else if (dest_house) {
            target = dest_house;
        } else {
            target = civilian;           // ← CIVILIAN FALLBACK
        }
        if (!target) {                   // no-one to receive — limbo the object
            obj->vtbl[0x16c](obj->xy, 0, rules_fallback_anim, 0,0,0,0);
            continue;
        }
        obj->vtbl[0x3d4](target, 1);     // ChangeOwner(target, force=1)
        obj->vtbl[0x3d0]();              // post-transfer update
        obj->vtbl[0x388](1);
        if (first_survivor == NULL) first_survivor = obj;
    }
    if (g_GameActive && first_survivor && !g_MapEditorMode && rules->AudioEventIdx != -1) {
        VocClass::PlayAt(0);             // play an eject sound cue
    }
    c->active = 0;
}
```

This is the **"passenger/contents dump"** path used when a building is destroyed, a transport
is warped, a temporal-weapon victim dies mid-carry, or a powerup building expires. The
**Civilian house is the ultimate fallback owner** — if no attacker or designated destination
house exists, whatever was inside is handed to Civilian. If Civilian itself is missing, the
object is sent into Limbo with a death anim.

**Consequence for the Rust engine:** any container/transport/PowerUp cleanup logic must have a
"Civilian-owned" fallback, else ejected objects leak. In `src/sim/` today, there is no such
fallback (no Civilian house modeled).

### 10.2 `FUN_0041ec90` — **TEvent "Civilian owns N of TechnoType X"** predicate

The decompilation now reads cleanly. The function is a trigger-event evaluator:

```c
bool Eval_CivilianOwnedCount(TEventClass* ev) {
    // 1) Locate Civilian house by side index
    int civ_idx = SideClass::FindByName("Civilian");
    HouseClass* civ = find_house_by_side(civ_idx);
    if (!civ) return false;

    int count = 0;
    TechnoTypeClass* subject = ev->+0xd8;        // the TechnoType whose instances are counted
    if (subject) {
        void* param = subject->vtbl[0x40]();     // "get argument for count" (type-specific)
        int rtti  = subject->vtbl[0x2c]();       // RTTI code
        switch (rtti) {
          case 0x03 /*Aircraft*/:
          case 0x07 /*Unit*/:
          case 0x10 /*Infantry*/:
          case 0x28 /*Building*/:
            count = HouseClass::CountOwnedInstances(param);
            break;
          default:
            count = 0;
        }
    }

    int threshold = ev->+0xe4;
    switch (ev->+0xe8) {                         // comparison operator
      case 0: return count <  threshold;
      case 1: return count <= threshold;
      case 2: return count == threshold;
      case 3: return count >= threshold;
      case 4: return count >  threshold;
      case 5: return count != threshold;
    }
    return false;
}
```

This is the condition behind trigger actions such as **"all civilian buildings destroyed"** (N=0,
op==), **"at least one civilian survives"** (N=1, op>=), etc. — the signature trigger used by
campaign "defend the town" / "destroy the base" objectives.

Dispatch tables at `0x0041ee30`, `0x0041ee44`, `0x0041ee6c` confirm the RTTI→category routing
and the 6-way comparison-op table. Only `0x0041ec90` in the TEvent family uses `SideClass::FindByName`;
other TEvents (cases 1–7 in `FUN_0041e720`) use alliance bitmasks, distance checks, or
per-house counters instead.

### 10.3 Write at `0x004e6f91` — **VectorClass destructor for the SideClass global array**

Raw disassembly of the bytes around `0x004e6f60..0x004e6fb0` reveals a VectorClass dtor sequence
targeting `DAT_008b4120..DAT_008b4134` (the SideClass global array):

```
mov  eax, [0x008b4124]         ; load data ptr
...
mov  [0x008b4120], 0x007ea044  ; restore vtable
mov  [0x008b4134], 10          ; reset grow_size
mov  [0x008b4130], eax         ; write count (observed at 0x004e6f67 / 0x004e6f91)
cmp  [0x008b412d], bl          ; if owns_memory
jz   +0x0f
push eax
call 0x007c8afe                ; operator delete
mov  [0x008b4124], ebx         ; clear data ptr
mov  [0x008b412d], bl          ; clear owns_memory
mov  [0x008b4128], ebx         ; clear capacity
ret
```

Ghidra has not classified this as a function (`get_function_by_address` returns none). It sits
inside an untagged region and is most likely referenced via a destructor vtable slot for the
global VectorClass wrapper that owns the SideClass array — i.e. the teardown path run on
process exit or between game sessions. **Not a hot path.** The earlier concern that a stray
write could corrupt the count is unfounded: it's a controlled cleanup.

### 10.4 Save-game interaction (not fully verified — reduced to a narrower open question)

A full trace of save/load paths for `DAT_008b4124` was not attempted this pass. What is
verifiable:

- SideClass has no `Save`/`Load` listed (no `SideClass::Save`/`Load` functions exist; the
  AbstractClass base vtable slots are present but not overridden).
- HouseTypeClass+0xBC (the side index stored on each country) is written/read by the country's
  own save path.
- The [Sides] parser (`FUN_00672440`) is called from `FUN_00668bf0` (rules loader), which runs
  on every game init — not only on new-game start.

Inference (not yet verified): on load, rules are re-parsed, SideClass instances re-created in
the same order (since the INI is deterministic), and saved country+0xBC indices remain
consistent. If a mod changes `[Sides]` order between save and load, indices would desync.
**To convert this to HIGH confidence, trace `ScenarioClass::Read_Scenario` / save-deserialization paths.**

---

## 11. Updated finding: runtime "Civilian" usage summary

With the new details, the six runtime `FindByName("Civilian")` call sites partition cleanly:

| Site | Purpose |
|------|---------|
| `HouseClass::Is_Enemy` | MP rule: Civilian is never hostile |
| `BuildingClass::CheckAutoSellOrCivilian` | Transfer abandoned building to Civilian |
| `CaptureManagerClass::SetOriginalOwner` | Mind-control "original owner" fallback |
| `AnimClass::AI` | **correction from prior section** — final-owner assignment when an anim's linked object has no owner (not "destruction credit" as I originally hedged) |
| `FUN_0041ec90` | TEvent "Civilian owns N of X" (§10.2) |
| `FUN_006b0ae0` | Container-eject fallback destination (§10.1) |

Civilian = universal neutral fallback. Every site follows the same pattern: *"if I can't find a
real owner/target, use Civilian."* Implementing the Civilian side as a special pseudo-house is
load-bearing; skipping it will cause misattribution bugs in each of these paths.

---

## 11. Ghidra annotation suggestions (for later — **not** applied in this pass)

- Rename `0x006a4550` from `SidebarClass__Constructor` to `SideClass__Constructor` (three
  functions currently share that label; the one at `0x006a4550` is unambiguous from its vtable
  write).
- Rename `0x006a46d0` to `SideClass__FindByName`.
- Rename `0x00672440` to `Sides_ReadINI` (or `SideClass::Process_Sides_Section`).
- Rename `0x004756f0` to `SideClass__FindOrCreate_FromKey`.
- Label `DAT_00817334` as `s_Side_Key`, `DAT_00818164` as `s_Civilian_Name`.
- Label `DAT_008b4124` as `g_SideClass_Array`, `DAT_008b4130` as `g_SideClass_Count`.

---

## Sources

**Ghidra addresses decompiled this session:**
- `0x006a4550` — SideClass::Constructor
- `0x006a46d0` — SideClass::FindByName
- `0x00672440` — [Sides] parser
- `0x004756f0` — SideClass::FindOrCreate_FromKey ("Side" key reader)
- `0x00511850` — HouseTypeClass::ReadINI (for §5 correction)
- `0x005113f0` — HouseTypeClass::Constructor (for field layout verification)
- `0x00501540` — HouseClass::Is_Enemy (+ disasm)
- `0x00458200` — BuildingClass::CheckAutoSellOrCivilian (+ disasm)
- `0x00472330` — CaptureManagerClass::SetOriginalOwner (+ disasm)
- `0x00423ac0` — AnimClass::AI (disasm — `0x004249a2` site)
- `0x0041ec90` — trigger evaluator (disasm — `0x0041ecb0` site)
- `0x006b0ae0` — cleanup path (disasm — `0x006b0afe` site)

**Memory reads:** `0x00817334` = `"Side\0"` verified. `0x00818164` = `"Civilian"`.

**Docs referenced:**
- `COUNTRY_SIDE_TYPE_CLASSES.md` (2026-03-22) — prior report covering SideTypeClass at a
  higher level; this report extends it and corrects §5 (`Side=` parsing claim).
- `COUNTRY_MULTIPLIERS_APPLICATION.md` (2026-03-22) — unrelated but adjacent.

**INI checked:** `ini/rulesmd.ini` `[Sides]`, `[British]`, `[Russians]`, `[YuriCountry]`,
`[Neutral]`, `[Special]`. `ini/rules.ini` for base-RA2 diff (no `ThirdSide` entry).

**Rust scan:** `src/map/houses.rs`, `src/sim/house_state.rs`, `src/render/sidebar_chrome.rs`,
`src/rules/ruleset.rs` (confirmed [Sides] not parsed).
