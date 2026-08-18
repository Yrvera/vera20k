# Superweapon Launch Handlers: Genetic Mutator, ParaDrop, SpyPlane

Research from gamemd.exe via Ghidra decompilation.
Confidence: HIGH for documented offsets and mechanics; MEDIUM for some decompilation details where Ghidra output was noisy.

## Main Entry Point

**SuperClass::Launch** at `0x006CC390` (955 lines decompiled).
Switches on `*(param_1[10] + 0xB4)` = SuperWeaponTypeClass->Type enum.

### SuperWeaponType Enum (from string table at 0x008425C0)

| Case | Enum Name | Description |
|------|-----------|-------------|
| 0 | MultiMissile | Nuclear Missile |
| 1 | IronCurtain | Iron Curtain |
| 2 | LightningStorm | Lightning Storm |
| 3 | ChronoSphere | Chrono Sphere |
| 4 | ChronoWarp | Chrono Warp (internal) |
| 5 | ParaDrop | Paradrop (side-dependent) |
| 6 | AmerParaDrop | American Paradrop |
| 7 | PsychicDominator | Psychic Dominator |
| 8 | SpyPlane | Spy Plane |
| 9 | GeneticConverter | Genetic Mutator |
| 10 | ForceShield | Force Shield |
| 11 | PsychicReveal | Psychic Reveal |

---

## 1. Genetic Mutator (Case 9 = GeneticConverter)

### Launch Handler (Case 9 in SuperClass::Launch, around 0x6CD800)

**Step 1: Play activation animation**
- Gets the target cell's 3D coordinates
- Creates an AnimClass using `Rules+0x298` (`IonBlast` anim) at the target location
- Note: Despite the offset name, this is the standard overhead blast effect; the actual genetic mutation anim is per-infantry

**Step 2: EVA + Sound + Radar**
- Plays EVA announcement (GeneticMutatorActivated)
- Plays GeneticMutatorActivateSound at coordinates
- Creates radar event at target

**Step 3: Kill infantry in area (controlled by `MutateExplosion` flag)**

Two code paths based on `Rules+0x17C8` (bool `MutateExplosion`, read from `[General] MutateExplosion`):

#### Path A: MutateExplosion = false (3x3 grid iteration)
- Iterates the standard 3x3 cell grid (address range `0xB0C038..0xB0C05C`, same grid used by other superweapons)
- For each cell, gets the occupant list (bridge-aware: uses cell+0xE8 for bridge cells, cell+0xE4 for ground)
- For each object in cell:
  - Checks if object type == 0x0F (infantry, via vtable+0x2C `What_Am_I()`)
  - Gets the infantry's TypeClass (vtable+0x84) and reads its armor at offset 0xA0
  - Calls `vtable+0x16C` (`ReceiveDamage`) with:
    - Damage = the armor value (enough to kill)
    - Warhead = `Rules+0xF98` (`MutateWarhead`)
    - Source = 0 (no attacker)
- This kills only infantry, and only in a 3x3 area

#### Path B: MutateExplosion = true (Apply_area_damage)
- Calls `Apply_area_damage()` at the target location
- Uses `Rules+0xF9C` (`MutateExplosionWarhead`) which has `CellSpread=5` and `InfDeath=9`
- This affects a larger area and damages all unit types (though the warhead's Verses make it 0% vs non-infantry)

**In standard YR:** `MutateExplosion=yes` in rulesmd.ini, so Path B is the active one.

### The Mutation Mechanism (How Infantry Become Brutes)

The actual mutation is NOT in the launch handler. It works through the **death animation system**:

1. **Warhead InfDeath field**: Both `Mutate` and `MutateExplosion` warheads have `InfDeath=9`
2. **InfDeath=9 maps to "Brute transformation"** (confirmed by rulesmd.ini comments and code)
3. **Death anim selection**: When infantry is killed, the engine selects a death animation based on the killing warhead's InfDeath value:
   - 0 = instant die
   - 1 = twirl die  
   - 2 = explodes
   - 3 = flying death
   - 4 = burn death
   - 5 = electro
   - 6 = Yuri head explode (plays `InfantryHeadPop` anim, Rules+0xA4)
   - 7 = Nuke Melt (plays `InfantryNuked` anim, Rules+0xA8)
   - 8 = Virus explosion (plays `InfantryVirus` anim, Rules+0xAC)
   - 9 = Brute transformation (plays `InfantryMutate` anim, Rules+0xB4)
   - 10 = smashed by brute (plays `InfantryBrute` anim, Rules+0xB0)

4. **`InfantryMutate` = GENDEATH animation** (from `[General] InfantryMutate=GENDEATH`)
5. **GENDEATH has `MakeInfantry=0`** in artmd.ini (AnimTypeClass offset 0x34C)
6. **AnimToInfantry list**: `[General] AnimToInfantry=BRUTE` (stored at Rules+0xCE4, count at Rules+0xCF4)
   - `MakeInfantry=0` means index 0 in the AnimToInfantry list = BRUTE

7. **AnimClass::AI** (at `0x00423AC0`): When the GENDEATH animation completes:
   - Checks `AnimType+0x34C != -1` (MakeInfantry is set)
   - Looks up `Rules+0xCE8` (AnimToInfantry type list) at index `AnimType[0x34C]`
   - Gets the infantry's AircraftType (at InfantryType+0xDF8) to spawn a paradrop aircraft
   - Creates the aircraft and places the new BRUTE infantry at the anim location
   - The owner is determined from the anim's stored owner (the house that fired the superweapon)

**Summary chain**: Genetic Mutator fires -> MutateExplosionWarhead kills infantry -> InfDeath=9 plays GENDEATH anim -> GENDEATH has MakeInfantry=0 -> AnimToInfantry[0]=BRUTE -> BRUTE infantry spawns owned by the superweapon's owner.

### Rules Offsets (Genetic Mutator)

| Offset | INI Key | Section | Type | Default (YR) |
|--------|---------|---------|------|--------------|
| 0x0298 | IonBlast | [General] | AnimType | (used as overhead SW anim) |
| 0x00B0 | InfantryBrute | [General] | AnimType | BRUTDIE |
| 0x00B4 | InfantryMutate | [General] | AnimType | GENDEATH |
| 0x0CE4 | AnimToInfantry | [General] | InfantryType list | BRUTE |
| 0x0CF4 | (AnimToInfantry count) | - | int | 1 |
| 0x0F98 | MutateWarhead | [SpecialWeapons] | WarheadType | Mutate |
| 0x0F9C | MutateExplosionWarhead | [SpecialWeapons] | WarheadType | MutateExplosion |
| 0x17C8 | MutateExplosion | [General] | bool | true (YR) |

---

## 2. ParaDrop (Cases 5 & 6)

### Case 5: Standard ParaDrop (Side-Dependent)

**Launch handler** (around 0x6CCD20 in SuperClass::Launch):

1. **Target cell validation**:
   - Gets target cell from MapClass::Get_CellClass
   - If target is on a bridge, calls `FootClass::Find_Nearby_Passable_Cell` to find a non-bridge cell nearby
   
2. **Side-based infantry selection** (checks `param_1[0xB] + 0x1E8` = HouseClass->Side):
   - **Side 0 (Allied)**: Rules+0xC40 (infantry type list), Rules+0xC4C (count), Rules+0xC68 (num list)
     - INI: `AllyParaDropInf=E1`, `AllyParaDropNum=6`
   - **Side 2 (Yuri)**: Rules+0xCB0, Rules+0xCBC, Rules+0xCD8
     - INI: `YuriParaDropInf=INIT`, `YuriParaDropNum=6`
   - **Else (Soviet)**: Rules+0xC78, Rules+0xC84
     - INI: `SovParaDropInf=E2`, `SovParaDropNum=9`

3. **For each infantry entry**: Validates `iVar21 != -1` (valid aircraft type exists) and the infantry type has a valid AircraftType field (offset 0xDF8 != -1), then calls `FUN_0065E660` (paradrop aircraft spawner).

### Case 6: AmerParaDrop (American-specific)

Same logic as Case 5 but uses a fixed set:
- Rules+0xC08 (infantry type list), Rules+0xC14 (count), Rules+0xC30 (num list)
- INI: `AmerParaDropInf=E1`, `AmerParaDropNum=8`
- No side switching -- always uses the American set
- Then falls through to the same LAB_006cd500 exit as Case 5

### Paradrop Aircraft Spawner: FUN_0065E660 (at 0x0065E660)

**Parameters** (reconstructed from call sites and decompilation -- __fastcall, decompiler struggled with param resolution):
- ECX (param_1): Owning HouseClass
- EDX (param_2): Index into the paradrop infantry type list
- Stack: Count, target cell, flags

**Behavior** (confidence: HIGH for overall flow, MEDIUM for exact parameter mapping due to decompiler issues):
1. Looks up the InfantryTypeClass from `g_InfantryTypeClass_Array[param_2]` (at 0x00A8B21C)
2. Calls the TypeClass vtable `Create` method (vtable+0x8C) to instantiate the infantry
3. Sets spawned flag at offset 0x3D4 = 1
4. Gets the HouseClass->Side (offset 0x1E0); if invalid (< 0 or > 3), calls `FUN_0050DA80()` to determine spawn edge
5. Calls `FUN_004AA440` to compute the spawn position at the map edge
6. Sets the mission to the target cell (vtable+0x1E8)
7. Sets the passenger (vtable+0x480) -- loads the infantry onto the aircraft
8. Sets the aircraft's target/destination (vtable+0x3C8)
9. Unlimbos the aircraft at the edge position (vtable+0xD8)
10. **Secondary aircraft creation**: If a condition is met (decompiled as `iVar8 == 2`), creates additional aircraft instances from `g_AircraftTypeClass_Array` (at 0x00A8E34C) using a separate aircraft type index. This creates the actual PDPLANE cargo aircraft that carries the infantry.
11. Calls vtable+0x1EC to finalize departure

**Aircraft type**: **PDPLANE** (Cargo Plane, index 7 in AircraftTypes). The aircraft type is resolved through the infantry type's associated AircraftType field (TechnoTypeClass offset 0xDF8). PDPLANE is defined in rulesmd.ini with `Primary=ParaDropWeapon` (dummy weapon).

**Spawn location**: Aircraft spawns at the map edge determined by the owning house's Side. The actual edge cell is computed by `FUN_004AA440` using the side index and map boundaries.

**Drop mechanism**: The aircraft flies to the target cell and drops infantry when within `ParadropRadius` leptons (Rules+0x54C, default 1024 leptons).

### Rules Offsets (ParaDrop)

| Offset | INI Key | Section | Type |
|--------|---------|---------|------|
| 0x0C08 | AmerParaDropInf (type list ptr) | [General] | InfantryType list |
| 0x0C14 | AmerParaDropNum (count) | [General] | int |
| 0x0C30 | AmerParaDropNum (values list ptr) | [General] | int list |
| 0x0C40 | AllyParaDropInf (type list ptr) | [General] | InfantryType list |
| 0x0C4C | AllyParaDropNum (count) | [General] | int |
| 0x0C68 | AllyParaDropNum (values list ptr) | [General] | int list |
| 0x0C78 | SovParaDropInf (type list ptr) | [General] | InfantryType list |
| 0x0C84 | SovParaDropNum (count) | [General] | int |
| 0x0CB0 | YuriParaDropInf (type list ptr) | [General] | InfantryType list |
| 0x0CBC | YuriParaDropNum (count) | [General] | int |
| 0x0CD8 | YuriParaDropNum (values list ptr) | [General] | int list |
| 0x054C | ParadropRadius | [General] | int (leptons) |

**Note on the parallel arrays**: Each paradrop config is stored as a pair of DynamicVectorClass arrays -- one for infantry type pointers and one for counts. The counts array must match the types array in length (checked via the `count == num_count` comparison before iterating). The "count" field is the DynamicVectorClass::Count and "values list ptr" is the backing array pointer.

---

## 3. SpyPlane (Case 8)

### Launch Handler (Case 8 in SuperClass::Launch, around 0x6CD5F0)

1. **Target cell validation**: Gets target cell, checks it's valid and not null
2. **Uses Allied paradrop config for iteration count**: Checks `Rules+0xC4C == Rules+0xC68` (Allied paradrop array count validation), then loops `Rules+0xC4C` times
3. **For each iteration**:
   - Validates aircraft type exists (`iVar21 != -1`)
   - Calls `FUN_0065EAB0` (spy plane spawner) with flag `0x1`
4. Falls through to the same exit as Case 9 (joined_r0x006cd7cd)

**Note**: The spy plane count is determined by the number of entries in the AllyParaDropInf configuration. In default YR, AllyParaDropInf=E1 (one entry), so one spy plane is spawned.

### SpyPlane Spawner: FUN_0065EAB0 (at 0x0065EAB0)

**Parameters** (__fastcall, 6 params based on signature -- decompiler struggled with some param assignments):
- ECX (param_1): Owning HouseClass
- EDX (param_2): Index (into infrastructure type array)
- Stack params: Count, target cell, mission flags, reveal target cell

**Behavior** (structurally very similar to FUN_0065E660 but simpler -- NO infantry loading):
1. Looks up a TypeClass from `g_InfantryTypeClass_Array[param_2]` (same global as paradrop at 0x00A8B21C)
2. Calls TypeClass vtable `Create` method (vtable+0x8C) to instantiate the aircraft
3. Sets spawned flag (offset 0x3D4 = 1)
4. Gets house Side (offset 0x1E0) for spawn edge; if invalid, calls `FUN_0050DA80()`
5. Computes spawn position at map edge via `FUN_004AA440`
6. Sets mission (vtable+0x1E8) -- "Spyplane Overfly" mission
7. Sets passenger/reveal target (vtable+0x480) if param_6 != 0
8. Sets navigation target (vtable+0x3C8) if param_3 != 0
9. Unlimbos at edge position (vtable+0xD8)
10. Calls vtable+0x1EC to finalize departure
11. **NO secondary aircraft creation loop** (unlike paradrop)
12. **NO infantry passenger loading** -- this is purely a flyover/reconnaissance aircraft

**Confidence note**: The decompiler shows both FUN_0065E660 and FUN_0065EAB0 using the same `g_InfantryTypeClass_Array` global. The exact mechanism by which the SPYP aircraft type is resolved (as opposed to PDPLANE for paradrop) needs further investigation -- it may be through the infantry type's AircraftType field (offset 0xDF8 on TechnoTypeClass) or through the superweapon type's own configuration. Confidence: MEDIUM on the exact aircraft type resolution path.

**Aircraft type**: **SPYP** (Soviet Spy Plane) -- defined in rulesmd.ini:
```ini
[SPYP]
Primary=SpyCameraWeapon
Spawned=yes
```

**SpyCameraWeapon**: 
- `Damage=6` -- this is the **reveal radius** (in cells), NOT actual damage
- `Range=20` -- how far from target the plane can be and still "fire" (reveal)
- `Warhead=DummyWarhead` -- does no real damage
- The plane "fires" this weapon during its overfly, which reveals shroud

**Reveal mechanism**: The SPYP aircraft has Mission_Overfly behavior (strings "Spyplane Overfly" at 0x00816D2C, "Spyplane Approach" at 0x00816D40). As it flies over the target area, it periodically fires SpyCameraWeapon which reveals shroud within a radius of 6 cells. The reveal happens every `SpyPlaneCameraFrames` frames (default 16, from `[AudioVisual] SpyPlaneCameraFrames`). A sound (`SpyPlaneCamera` = SpyPlaneSnapshot) plays each time.

**Summary**: The spy plane does NOT attack. It flies a straight path over the target, revealing shroud in a 6-cell radius as it goes, playing a camera snapshot sound every 16 frames.

### Rules Offsets (SpyPlane)

| Offset (AudioVisual) | INI Key | Section | Type |
|--------|---------|---------|------|
| 0x0280 (0xA0 * 4) | SpyPlaneCamera | [AudioVisual] | Sound (VocClass index) |
| 0x0290 (0xA4 * 4) | SpyPlaneCameraFrames | [AudioVisual] | int (default 16) |

Note: AudioVisual offsets use `param_1` as `undefined4*` (int pointer), so multiply the index by 4 for byte offsets.

---

## Key Global Addresses

| Address | Description |
|---------|-------------|
| g_RulesClass_Instance | Global Rules singleton pointer |
| g_InfantryTypeClass_Array | Array of InfantryTypeClass pointers |
| g_AircraftTypeClass_Array | Array of AircraftTypeClass pointers |
| g_BuildingClass_Array | Array of BuildingClass pointers |
| g_HouseClass_Array | Array of HouseClass pointers |
| 0x00B0C038..0x00B0C05C | 3x3 cell offset grid (9 entries of 2 shorts each) |
| DAT_00B0C07C | Bridge height offset |
| g_CurrentFrameCounter | Current game tick |

## Key Function Addresses

| Address | Name | Purpose |
|---------|------|---------|
| 0x006CC390 | SuperClass::Launch | Main superweapon launch dispatcher |
| 0x0065E660 | FUN_0065E660 | Paradrop aircraft spawner |
| 0x0065EAB0 | FUN_0065EAB0 | Spy plane aircraft spawner |
| 0x00423AC0 | AnimClass::AI | Animation per-tick update (handles MakeInfantry spawn) |
| 0x00428100 | AnimTypeClass::ReadINI | Reads MakeInfantry and other anim properties |
| 0x006690A0 | RulesClass::ReadSpecialWeapons | Reads MutateWarhead etc. |
| 0x0066A200 | RulesClass::ReadAudioVisual | Reads SpyPlaneCamera etc. |
| 0x0066D7A0 | RulesClass::ReadGeneral | Reads paradrop configs, MutateExplosion flag, etc. |
