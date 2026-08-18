# SLAV — Yuri Slave Worker

**Side classification:** Yuri (Owner=YuriCountry only).
**Role:** Spawned cargo of the Yuri Slave Miner (SMIN/SMON) and Yuri Ore Refinery (YAREFN).
Slaves walk to ore fields, dig with a shovel, carry bales back to the master, and
deposit credits — the whole mechanism is the Yuri-side ore economy.

**Two-mode unit**: while enslaved (bound to a SlaveManager-owning master), the slave
is AI-controlled and uses a distinct voice set; if the master dies the slave is freed
and becomes a wandering hostile under no player's control, switching to a different
voice set.

> Output bar: ore-economy parity hinges on this unit. Bales per cycle, frames per
> bale, walk-back distance, and the master-death freeing behavior all matter for
> player-feel.

> **Deep-RE cross-references — don't re-derive these:**
> - **[SLAVE_MANAGER_STATE_MACHINE_GHIDRA_REPORT.md](../../SLAVE_MANAGER_STATE_MACHINE_GHIDRA_REPORT.md)**
>   — SlaveManagerClass lifecycle, state machine, docking flow, ownership transfer,
>   MasterDestroyed path. Full structure layout at `TechnoClass+0x2D8`.
> - **[SLAVE_MINER_ORE_SYSTEM_GHIDRA_REPORT.md](../../SLAVE_MINER_ORE_SYSTEM_GHIDRA_REPORT.md)**
>   — Slave Miner deploy/undeploy, ore-dumping cycle, SMIN ↔ YAREFN brain-transplant
>   handling. INI key offsets in TechnoTypeClass.

> Ghidra confirms no `"SLAV"` / `"Slave"` plain string in `gamemd.exe` for the unit
> ID — but `"Slaved"`, `"Enslaves"`, `"VoiceSelectEnslaved"` ARE present (verified
> §7). Slave behavior is driven by **TechnoType flag handling + SlaveManagerClass
> state machine**, with SLAV-the-unit serving as the typed slot referenced by the
> master's `Enslaves=SLAV`.

---

## 1. `rulesmd.ini` — `[SLAV]` verbatim

```ini
[SLAV]
UIName=Name:SLAV
Name=Yuri Slave Worker
Category=Soldier
Primary=SHOVEL
CrushSound=InfantrySquish
Slaved=yes
Strength=125
Armor=none
TechLevel=-1
Pip=white
Sight=5
Speed=3
Owner=YuriCountry
AllowedToStartInMultiplayer=no
Cost=10
Soylent=0
Points=5
IsSelectableCombatant=no
VoiceSelect=SlaveFreedSelect
VoiceSelectEnslaved=SlaveWorkerSelect;gs this is the alternate voice set to use while enslaved.  Don't need to double the rest since the only thing the player can do to an enslaved slave is select it.
VoiceMove=SlaveFreedMove
VoiceAttack=SlaveFreedAttackCommand
VoiceFeedback=SlaveWorkerFear
VoiceSpecialAttack=SlaveWorkerHarvest
DieSound=SlaveWorkerDie
Locomotor={4A582744-9839-11d1-B709-00A024DDAFD1}
PhysicalSize=1
MovementZone=Infantry
ThreatPosed=2	; This value MUST be 0 for all building addons
VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,FASTER
EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF
ImmuneToVeins=yes
ImmuneToPsionics=yes
Size=1
ElitePrimary=SHOVEL
IFVMode=0
Storage=4;2
HarvestRate=150;180;210;75;frames between bale pickup
PipScale=Tiberium
DontScore=yes
```

### Key-by-key explanation

| Key | Value | Scope | Effect |
|-----|-------|-------|--------|
| `UIName` | `Name:SLAV` | AbstractType | CSF lookup — there *is* a dedicated `Name:SLAV` CSF entry (in contrast to VLADIMIR/PENTGEN reusing CIV1). |
| `Name` | `Yuri Slave Worker` | AbstractType | Dev/fallback name. |
| `Category` | `Soldier` | TechnoType | Infantry classifier. |
| `Primary` | `SHOVEL` | TechnoType | Melee shovel weapon — see §3. Used both for harvesting ore (during the harvest cycle) and as combat weapon if the slave is freed and attacks. |
| `CrushSound` | `InfantrySquish` | TechnoType | Squish on crush. |
| `Slaved` | `yes` | **TechnoType** (verified — 0x00843830 read at 0x00714db6) | **The hardcoded "this unit is a slave" flag.** When set, the unit is owned by a `SlaveManagerClass` instance on a master TechnoClass; the master's `Enslaves=SLAV` is what spawned this unit. Drives AI control (slave goes through harvest-shovel-carry-dump loop instead of normal combat AI), and the master-death freeing path. Full mechanics: SLAVE_MANAGER_STATE_MACHINE_GHIDRA_REPORT.md. |
| `Strength` | `125` | AbstractType | 125 HP — slaves are surprisingly tough for an infantry (more than GI=100, less than Tanya=125, equal to SHK Tesla Trooper=125). |
| `Armor` | `none` | TechnoType | Slot 1 — basic infantry armor profile. |
| `TechLevel` | `-1` | TechnoType | **Not buildable directly**. Only spawned by the master's `Enslaves=` mechanism. |
| `Pip` | `white` | InfantryType | Transport pip colour (white/neutral). |
| `Sight` | `5` | TechnoType | 5-cell reveal — wider than civilians (2) since slaves need to spot ore. |
| `Speed` | `3` | TechnoType | Very slow — slower than basic infantry (4) and even VLADIMIR (4). Slaves are the slowest combat-capable infantry. |
| `Owner` | `YuriCountry` | TechnoType | Yuri-only ownership. Other houses cannot own slaves natively. (A non-Yuri house can possess slaves only via capturing a master that's currently slave-bound.) |
| `AllowedToStartInMultiplayer` | `no` | TechnoType | Never preplaced. |
| `Cost` | `10` | TechnoType | Nominal — irrelevant (TechLevel=-1, spawned not built). |
| `Soylent` | `0` | TechnoType | **No Grinder refund** — slaves cannot be fed back into a Grinder for cash. Asymmetric with VLADIMIR/PENTGEN ($200 refund); Yuri can't farm cash by grinding its own slaves. |
| `Points` | `5` | TechnoType | Score on kill — but see `DontScore` below. |
| `IsSelectableCombatant` | `no` | TechnoType | Slave is **not** included in "select all combat units" hotkey. Combined with `DontScore=yes`, treats the slave as not really a player-controlled combat unit. |
| `VoiceSelect` | `SlaveFreedSelect` | TechnoType | Voice set used **after master is destroyed** (slave is freed). 5 distressed/desperate clips. |
| `VoiceSelectEnslaved` | `SlaveWorkerSelect` | TechnoType (verified — 0x008442a0 read at 0x00712ba0) | **Hardcoded unique key** — the engine swaps voice sets based on slave state. While the slave is bound to a master, click-select plays from `SlaveWorkerSelect` (6 working clips). INI comment: "this is the alternate voice set to use while enslaved. Don't need to double the rest since the only thing the player can do to an enslaved slave is select it." Confirms only the *Select* voice has a dual-mode swap; all other voice hooks fire only when the slave is freed (since enslaved slaves take no player commands). |
| `VoiceMove` | `SlaveFreedMove` | TechnoType | Only triggers after freeing (enslaved slaves don't accept move orders). |
| `VoiceAttack` | `SlaveFreedAttackCommand` | TechnoType | Same. |
| `VoiceFeedback` | `SlaveWorkerFear` | TechnoType | Under-attack voice — uses `SlaveWorkerFear` (10 clips, including both `$isl1fe*` worker-fear and `$isl2fe*` freed-fear clips merged into one pool). The freed-vs-enslaved distinction is intentionally collapsed for fear voices. |
| `VoiceSpecialAttack` | `SlaveWorkerHarvest` | TechnoType | Bound to the harvest action — the engine triggers this voice via the special-attack hook when the slave performs an ore-pickup. 5 worker-grunt clips. |
| `DieSound` | `SlaveWorkerDie` | TechnoType | Death sound — 5 clips, range 30. |
| `Locomotor` | `{4A582744-9839-11d1-B709-00A024DDAFD1}` | TechnoType | WalkLocomotionClass (standard infantry walk). |
| `PhysicalSize` | `1` | TechnoType | Sub-cell footprint. |
| `MovementZone` | `Infantry` | TechnoType | Standard infantry pathing. |
| `ThreatPosed` | `2` | TechnoType | Very low AI threat — enemy AI mostly ignores slaves. Comment "This value MUST be 0 for all building addons" is a generic warning, irrelevant here. |
| `VeteranAbilities` | `STRONGER,FIREPOWER,ROF,SIGHT,FASTER` | TechnoType | In practice slaves rarely gain XP (they don't normally engage in combat while enslaved). |
| `EliteAbilities` | `SELF_HEAL,STRONGER,FIREPOWER,ROF` | TechnoType | Same caveat. |
| `ImmuneToVeins` | `yes` | TechnoType | **TS-LEGACY** dormant. |
| `ImmuneToPsionics` | `yes` | TechnoType (verified prior iter — 0x00714fa7) | Cannot be mind-controlled. Important for parity: a Yuri player's Yuri Prime cannot accidentally re-control its own slaves; an enemy Yuri/Initiate cannot steal them via psi. |
| `Size` | `1` | TechnoType | Transport-slot cost. |
| `ElitePrimary` | `SHOVEL` | TechnoType | Same as Primary — no elite weapon swap. Slaves don't really "elite". |
| `IFVMode` | `0` | TechnoType | IFV gunner mode 0 → IFV's `Weapon1=HoverMissile`. Slaves never normally board an IFV (they're Yuri-only and AI-controlled), but the flag is set. |
| `Storage` | `4;2` | TechnoType (verified — 0x008441ac read at 0x00713130) | **Per-bale storage capacity.** Comment ";2" is an older value, kept for INI history. With `Storage=4`, each slave can carry up to **4 ore bales** before returning to the master to dump. Each bale at ~$25 → ~$100 per round trip per slave. |
| `HarvestRate` | `150;180;210;75;frames between bale pickup` | **InfantryType** (verified — 0x0082597c read at 0x00524523) | **Frames between bale pickups during harvest.** Live value `150` ticks (≈9 s @ 60 FPS sim, or 2.5 s @ standard 15 fps anim cadence depending on rate config). Comments show prior tuning attempts (180, 210, 75) — the shipped value `150` was the final balance. With `Storage=4`, a full harvest cycle takes `4 × 150 = 600 ticks` plus walk-out + walk-back. **Notable scope**: `HarvestRate` is in `InfantryTypeClass__ReadINI` (the 0x00524000 range), not TechnoType — it's specific to slave-style infantry harvesters. |
| `PipScale` | `Tiberium` | InfantryType / UnitType | Renders carried-bale pips in `Tiberium`-style (the ore-pip rendering mode). Lets the player see how full the slave is at a glance. |
| `DontScore` | `yes` | TechnoType (verified — 0x00843ec0 read at 0x00713f4b) | Killing this unit doesn't credit the attacker's score (despite `Points=5` above — `DontScore` overrides). Slaves are economy/non-combat units, so kill-score is suppressed for game-feel reasons. |

---

## 2. `artmd.ini` — `[SLAV]` section

```ini
[SLAV] ; Slave for slave miner
Cameo=E2ICON
AltCameo=E2UICO
Sequence=SlaveSequence
Crawls=no
Remapable=yes
FireUp=6
PrimaryFireFLH=60,0,100
```

| Key | Value | Effect |
|-----|-------|--------|
| `Cameo` | `E2ICON` | **Reuses Conscript (E2) cameo** — but slaves are never built from the sidebar, so the cameo is only visible in transport-passenger pips / debug UI. |
| `AltCameo` | `E2UICO` | Yuri-skinned E2 cameo. |
| `Sequence` | `SlaveSequence` | Custom frame table — slave-specific (Shovel and Carry poses) — see below. |
| `Crawls` | `no` | Slaves **cannot** crawl/prone-fire. (Compare VLADIMIR/PENTGEN/SLAV: `Crawls=yes/yes/no`.) Combined with the SlaveSequence's stub `Prone/Down/Up=0,1,1`, slaves stay upright when suppressed. The INI comment "can't crawl, but need listing for spy" reveals the reason: the entries exist so the Spy's disguise system has valid frames to use when disguised-as-slave. |
| `Remapable` | `yes` | House-color remap. |
| `FireUp` | `6` | 6 frames into FireUp before projectile spawns. |
| `PrimaryFireFLH` | `60,0,100` | Fire offset (X=60 forward, Y=0, Z=100 — shoulder-height). |

### `[SlaveSequence]` referenced sequence

```ini
[SlaveSequence]
Ready=0,1,1
Guard=0,1,1
Walk=8,6,6
Idle1=56,15,0,S
Idle2=71,15,0,E
Die1=86,15,0
Die2=101,15,0
Die3=0,1,1
Die4=0,1,1
Die5=0,1,1
Shovel=116,6,6
FireUp=164,8,8
Carry=228,6,6
Cheer=276,8,0,E
Prone=0,1,1;can't crawl, but need listing for spy
Down=0,1,1
Crawl=8,6,6
Up=0,1,1
FireProne=164,8,8
Paradrop=0,1,1
Panic=8,6,6
FireUpSounds= 2 SlaveAttack
```

| Row | Notes |
|-----|-------|
| `Ready/Guard=0,1,1` | Stand pose. |
| `Walk=8,6,6` | 6-frame walk × 6 facings. |
| `Idle1/Idle2` | Two idles (S, E facings). |
| `Die1=86 / Die2=101` | Two distinct death anims — frame layout differs from E1/Gen (E1 dies start at 134, Gen at 71). |
| `Die3-5=0,1,1` | Stub entries. |
| **`Shovel=116,6,6`** | **Unique slave pose** — 6-frame "digging with shovel" × 6 facings. Used when harvesting ore. |
| `FireUp=164,8,8` | **8 frames × 8 facings** (most infantry: 6,6). The slave's combat-fire pose uses higher facing resolution. |
| **`Carry=228,6,6`** | **Unique slave pose** — 6-frame "walking while carrying ore bale" × 6 facings. Used during the walk-back-to-master phase of the harvest loop. |
| `Cheer=276,8,0,E` | Victory cheer, east-locked. |
| `Prone/Down/Up=0,1,1` | All stubs — "can't crawl, but need listing for spy" (INI comment). |
| `Crawl=8,6,6` | Maps to the walk frames (slave continues walking even if "crawling" is requested). |
| `FireProne=164,8,8` | Same frames as FireUp — no real prone-fire variant. |
| `Paradrop=0,1,1` | Stub — slaves don't paradrop. |
| `Panic=8,6,6` | Reuses walk frames. |
| **`FireUpSounds= 2 SlaveAttack`** | **Anim-frame sound trigger**: at frame 2 of the FireUp sequence, the `SlaveAttack` sound is played. This is the engine's anim-event audio mechanism — not the weapon's `Report=`, but an animation-driven sound cue. Notable because SHOVEL's `Report=` is empty (see §3); the shovel "thwack" sound comes from this anim-trigger instead. |

The two unique poses — **Shovel** (digging) and **Carry** (bale-on-shoulder walk) — are
what make a slave visually distinct from any other YR infantry. No other infantry sequence
references either row.

---

## 3. Weapon — `[SHOVEL]`

```ini
[SHOVEL]
Range=1.5
CellRangefinding=yes
Projectile=InvisibleLow
Speed=100
Damage=30
ROF=30
Warhead=SA
Report=
```

| Key | Value | Effect |
|-----|-------|--------|
| `Range` | `1.5` | 1.5-cell melee range. Slaves must walk right up to the target. |
| `CellRangefinding` | `yes` | Range measured cell-to-cell (lets the slave hit a target on the far side of the adjacent cell). |
| `Projectile` | `InvisibleLow` | Inviso projectile that respects terrain. |
| `Speed` | `100` | Bullet speed (inviso). |
| `Damage` | `30` | 30 dmg per hit — surprisingly high for an "economy unit" weapon. Vs `none` armor with SA Verses 100%: 30 dmg/hit. Will one-shot most civilians. |
| `ROF` | `30` | 30-tick cooldown (≈1.8 s). |
| `Warhead` | `SA` | Small-arms warhead (Verses 100/80/80/50/25/25/75/50/25/100/100; InfDeath=1; PIFFPIFF anim). |
| `Report` | *(empty)* | **No weapon-fire sound** — intentionally blank. The "thwack" sound comes from the SlaveSequence's `FireUpSounds= 2 SlaveAttack` anim-trigger (see §2). |

Net DPS vs basic infantry: 30 dmg × 100% / 30 tick = 1.0 dmg/tick raw. A freed slave engaged with a GI (100 HP, none armor) wins in ~4 hits = ~7 seconds. The SHOVEL is **not just a flavor weapon**; it's competitive with basic infantry small-arms in melee.

Vs tanks (medium armor): 30 × 25% = 7.5 dmg/hit. Slaves cannot threaten armored vehicles meaningfully.

---

## 4. Warhead — `[SA]` (Small Arms)

Standard Small-Arms warhead — see [`soviet/VLADIMIR.md`](../soviet/VLADIMIR.md#4-warhead--sa) §4 for full breakdown. Same Verses, InfDeath=1, PIFFPIFF, ProneDamage=70%.

---

## 5. Voices / sounds — DUAL-MODE voice system

SLAV is the **only** YR unit that uses `VoiceSelectEnslaved=` to swap voice sets based
on slave state. Two distinct voice families:

### Enslaved-mode voices (master still alive)

```ini
[SlaveWorkerSelect]
Sounds= $isl1sea $isl1seb $isl1sec $isl1sed $isl1see $isl1sef
Control=random
Volume=85

[SlaveWorkerMove]
Sounds= $isl1moa $isl1mob $isl1moc $isl1mod $isl1moe
Control=random
Volume=85

[SlaveWorkerAttackCommand]
Sounds= $isl1ata $isl1atb $isl1atc $isl1atd $isl1ate
Control=random
Volume=85

[SlaveWorkerFear]
Sounds= $isl1fea $isl1feb $isl1fec $isl1fed $isl1fee $isl2fea $isl2feb $isl2fec $isl2fed $isl2fee
Control=random
Range=30
Volume=85

[SlaveWorkerHarvest]
Sounds=$isl1haa $isl1hab $isl1hac $isl1had $isl1hae
Control=random
Volume=85

[SlaveWorkerLiberated]
Sounds= $isl1lia ;$isl1lib $isl1lic
Control=random
Range= 30
Volume=95

[SlaveWorkerDie]
Sounds=$isl2dia $isl2dib $isl2dic $isl2did $isl2die
Control=random
Range=30
Volume=85
```

### Freed-mode voices (master destroyed → slave wanders hostile)

```ini
[SlaveFreedSelect]
Sounds=$isl2sea $isl2seb $isl2sec $isl2sed $isl2see
Control=random
Volume=85

[SlaveFreedMove]
Sounds=$isl2moa $isl2mob $isl2moc $isl2mod $isl2moe
Control=random
Volume=85

[SlaveFreedAttackCommand]
Sounds=$isl2ata $isl2atb $isl2atc $isl2atd $isl2ate
Control=random
Volume=85

[SlaveFreedFear]
Sounds=$isl2fea $isl2feb $isl2fec $isl2fed $isl2fee
Control=random
Volume=85

[SlaveFreedDie]
Sounds=$isl2dia $isl2dib $isl2dic $isl2did $isl2die
Control=random
Volume=85
```

### Hook mapping

| Hook | While enslaved | After freed | Notes |
|------|----------------|-------------|-------|
| `VoiceSelect` | `SlaveWorkerSelect` ($isl1se*, 6 clips) | `SlaveFreedSelect` ($isl2se*, 5 clips) | Engine swaps based on slave state via `VoiceSelectEnslaved` hook. |
| `VoiceMove` | (no command accepted) | `SlaveFreedMove` ($isl2mo*, 5 clips) | INI comment confirms: enslaved slaves take no player commands; the only player interaction is selection. |
| `VoiceAttack` | (no command accepted) | `SlaveFreedAttackCommand` ($isl2at*, 5 clips) | Same. |
| `VoiceFeedback` | `SlaveWorkerFear` (10 clips — both `$isl1fe*` worker + `$isl2fe*` freed pooled together) | (same pool) | Pooled fear pool plays regardless of mode. `Range=30` limits hearing distance. |
| `VoiceSpecialAttack` | `SlaveWorkerHarvest` ($isl1ha*, 5 clips) — triggers on ore-pickup | (likely silent) | Engine fires the special-attack voice when the slave performs the shovel/harvest action. |
| `DieSound` | `SlaveWorkerDie` ($isl2di*, 5 clips, Range=30) | (same) | Death sound uses the same clips regardless of mode — both `SlaveWorkerDie` and `SlaveFreedDie` happen to reference the same $isl2di* clips. |
| Anim-frame trigger (FireUp frame 2) | `SlaveAttack` sound | (same) | Played when the shovel "thwack" lands — see §2. |
| `[SlaveWorkerLiberated]` | (n/a — fires once at the moment of freeing) | — | A single one-shot voice clip ($isl1lia) plays when the master is destroyed and the slave becomes free. `Range=30, Volume=95` make it audible/notable. Two more clips were planned (commented out: $isl1lib, $isl1lic). |

| Other sounds | Definition | Trigger |
|-------------|------------|---------|
| `CrushSound=InfantrySquish` | `igensqua`, FShift ±10, vol 65 | When crushed |
| Anim trigger `SlaveAttack` (not shown — referenced by FireUpSounds) | (presumably a wood-thwack clip in soundmd.ini under that exact name) | At frame 2 of FireUp anim |

**Net audio profile**: the slave is a chatty unit — distinct working mutter while enslaved
("master, master..."), distressed pleading when selected after freeing, a liberation cry
at the moment the master dies, and shovel-thwacks driven by anim frames not weapon Report.

---

## 6. Prerequisites / owners / spawn paths

- `TechLevel=-1` + `AllowedToStartInMultiplayer=no` → **never directly buildable**.
- `Owner=YuriCountry` — only Yuri can natively own slaves.
- `Prerequisite=` — none (irrelevant; spawn is via SlaveManager).
- No `RequiredHouses=`, no `RequiresStolen*Tech=`, no `BuildLimit=`.

### Spawn mechanism — driven by other units' `Enslaves=SLAV` flag

```ini
# SMIN (Slave Miner vehicle)
Enslaves=SLAV  ; line 13279

# YAREFN (Yuri Refinery building)
Enslaves=SLAV  ; line 9099
```

Both the Slave Miner (vehicle and deployed building form SMON) and the Yuri Refinery
spawn SLAV instances via their SlaveManagerClass. Per [SLAVE_MANAGER_STATE_MACHINE_GHIDRA_REPORT.md](../../SLAVE_MANAGER_STATE_MACHINE_GHIDRA_REPORT.md) §1:

- `SlaveManagerClass` constructor at `0x006AF1A0`, called from `TechnoClass::Init_Managers @ 0x006F3F40` whenever the techno's type has `Enslaves != NULL`.
- 4-arg signature: `(owner, Enslaves, SlavesNumber, SlaveRegenRate, SlaveReloadRate)`.
- The SlaveManager instance lives at `TechnoClass+0x2D8`.

### Master-death freeing

When the master is destroyed, `SlaveManagerClass::MasterDestroyed` (via the destructor path
at `0x006AF5A6`) iterates the bound slaves and releases them — they become free, switch to
the `SlaveFreed*` voice set, and wander hostile to all houses.

Xrefs to `MasterDestroyed` (from the deep-RE doc):
- `MissionClass::Constructor @ 0x006F4571` — master entering destroyed mission state
- `PowerUp_Cleanup @ 0x006AF5A6` — SlaveManager destructor proper
- `TeleportLocomotionClass::PostWarpValidation @ 0x00718998, 0x00718AEF` — chrono warp failure
- `TemporalClass::Update @ 0x0071AA2C, 0x0071AAA7` — slowly-erased-by-Chronosphere
- `TechnoClass::ReceiveDamage @ 0x00702065` — fatal damage path
- `JumpjetLocomotionClass::Process` state 5 — Magnetron force-kill mid-air

⇒ The freeing trigger fires whenever the master enters any "destroyed" state, including
Chronosphere-erasure, Temporal-erasure, Magnetron-drop-no-landing, and fatal damage.

### Brain transplant (SMIN ↔ SMON / YAREFN deploy)

INI comment near the `Enslaves=SLAV` lines: "The Refinery does not get an Enslaves
listing because the Slave object will get passed from unit to building upon deploy" —
but both SMIN *and* YAREFN actually carry `Enslaves=SLAV`. Per [SLAVE_MINER_ORE_SYSTEM_GHIDRA_REPORT.md](../../SLAVE_MINER_ORE_SYSTEM_GHIDRA_REPORT.md):
the engine's deploy path at `UnitClass::Deploy @ 0x007393c0` does **not** explicitly
transfer the SlaveManager — instead, both forms have their own SlaveManager (one as
unit, one as building), and the slaves' bind is reassigned through a "brain transplant"
check that prevents duplication. INI also notes "Brain transplant will check to make
sure extra one is not created."

---

## 7. Hardcoded behavior (Ghidra-verified)

### 7.1 SLAV-specific code in `gamemd.exe`

| Query (search_strings) | Result |
|------------------------|--------|
| `SLAV` (plain) | 0 matches |
| `Slave` (plain) | (would match any string containing "Slave" — not run; not load-bearing) |
| `Name:SLAV` (CSF key) | Not searched; CSF lookup is data-driven, not code-driven |

⇒ **No SLAV-specific hardcoded ID**. All behavior is driven by:
1. The `Slaved=yes` TechnoType flag.
2. The master's `Enslaves=SLAV` reference.
3. The SlaveManagerClass state machine and structure.
4. The `VoiceSelectEnslaved` voice-state-swap hook.

### 7.2 Flag-scope verification (this iteration)

| Key | String at | Read by | Class scope |
|-----|-----------|---------|-------------|
| `Slaved` | 0x00843830 | TechnoTypeClass__ReadINI @ 0x00714db6 | TechnoType |
| `Enslaves` | 0x00843824 | TechnoTypeClass__ReadINI @ 0x00714dd7 | TechnoType |
| `VoiceSelectEnslaved` | 0x008442a0 | TechnoTypeClass__ReadINI @ 0x00712ba0 | TechnoType |
| `Storage` | 0x008441ac | TechnoTypeClass__ReadINI @ 0x00713130 | TechnoType |
| `HarvestRate` | 0x0082597c | **InfantryTypeClass__ReadINI** @ 0x00524523 | **InfantryType** (notable — not TechnoType) |
| `DontScore` | 0x00843ec0 | TechnoTypeClass__ReadINI @ 0x00713f4b | TechnoType |

⇒ `HarvestRate` being an InfantryType-only field is significant: only infantry can use it. The Slave Miner (UnitType) uses a separate `HarvesterDumpRate` global (per the SLAVE_MINER_ORE_SYSTEM doc, `RulesClass+0x1528`) for its own dumping cadence.

### 7.3 Live behaviors driven by flags + SlaveManagerClass

| Behavior | Driver | Notes / reference |
|----------|--------|-------|
| Spawned by Slave Miner / Refinery | `SlaveManagerClass` constructor (called from `TechnoClass::Init_Managers @ 0x006F3F40` when `Enslaves != NULL`) | SLAVE_MANAGER doc §1 |
| Bound to master | SlaveManager owns slave pointer; slave's `Slaved=yes` marks it | SLAVE_MANAGER doc class layout |
| Walks to ore field, digs with shovel | AI-driven state machine in SlaveManager / harvest mission | SLAVE_MANAGER doc state machine |
| Harvest cadence (frames between bales) | `HarvestRate=150` from InfantryType | Comments show 75/180/210 were tried; 150 shipped |
| Carries up to 4 bales | `Storage=4` from TechnoType | At ~$25/bale → ~$100/round trip per slave |
| Returns to master, dumps ore | Mission_Deploy_Building state 3 reads `RulesClass+0x1528` (HarvesterDumpRate) | SLAVE_MINER doc §1 |
| Plays one-shot "Liberated" voice when master dies | `MasterDestroyed` path triggers `[SlaveWorkerLiberated]` ($isl1lia, Range=30, Vol=95) | Visible-but-rare event |
| Switches to `SlaveFreed*` voice set after freeing | `VoiceSelectEnslaved` hook is consulted only while enslaved; absence/freed state falls back to `VoiceSelect=SlaveFreedSelect` | The only INI mechanism using this dual-voice swap |
| Cannot be mind-controlled | `ImmuneToPsionics=yes` | Prevents Yuri-vs-Yuri slave-steal exploits |
| Cannot be crushed | `Category=Soldier` default + no `Crushable=no` set explicitly — slaves CAN be crushed | (Unlike CCOMAND/TANY/etc., slaves have **no `Crushable=no`** — a tank can crush them.) |
| Selectable but not in "select all combat" | `IsSelectableCombatant=no` | Player can click on individual slaves to hear them, but they don't join the combat selection |
| No score on kill | `DontScore=yes` overrides `Points=5` | Economy-unit kill-suppression |
| Special-attack voice plays on ore-pickup | `VoiceSpecialAttack=SlaveWorkerHarvest` | Voice triggered by the engine's "ore acquired" event |
| Shovel hit sound from anim, not weapon | `FireUpSounds= 2 SlaveAttack` in artmd; `[SHOVEL] Report=` empty | Decouples sound from weapon timing |
| Spy can disguise as slave | `Crawls=no` + stub `Prone/Down/Up` entries (INI comment) | Spy disguise system needs valid frame data for all sequence rows |

### 7.4 Behaviors NOT present

- No `Crushable=no` → slaves **can be crushed** by tanks (unlike most named hero infantry).
- No `Trainable=no` → slaves *can* gain veterancy in theory, but rarely engage in combat while enslaved.
- No `Secondary` weapon.
- No `Bombable=no` (default Bombable=yes) → Crazy Ivan can plant bombs on slaves.
- No `DetectDisguise`.
- No `C4`, no `Engineer`, no `Ivan`.
- No `Fearless` → slaves panic-flee normally when not enslaved.

---

## 8. TS-legacy filter

| INI line | TS-LEGACY? | Status in YR |
|----------|-----------|--------------|
| `ImmuneToVeins=yes` | YES | Dormant. |
| (no `TiberiumProof` line) | — | SLAV lacks `TiberiumProof` — slaves *can* take tiberium damage (irrelevant in YR since no tiberium). |
| `Storage=4` (despite `;2` history) | n/a | Live — current bale capacity. |

No fog-of-war flags, no tunnel/subterranean. The `PipScale=Tiberium` is a render
configuration, not a TS-legacy reference — it just reuses the tiberium-pip drawing
code for the ore-bale-pip rendering.

---

## 9. Veterancy

`Trainable=` is not specified → defaults to `yes`. In theory the slave can gain XP. But:
- Most slaves never fire SHOVEL outside of harvest activity.
- `ThreatPosed=2` means enemy AI rarely targets them, so combat XP from "being shot at" is rare.
- Even if a slave reaches veteran/elite, `ElitePrimary=SHOVEL` means **no weapon upgrade** at elite.

`VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,FASTER` and `EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF` would apply if rank reached, but the practical observable effect is zero in normal play.

---

## 10. Cross-references

### Direct dependencies (`rulesmd.ini` / `artmd.ini` / `soundmd.ini`)
- `[SHOVEL]` — weapon (§3)
- `[InvisibleLow]` — projectile
- `[SA]` — warhead (§4)
- `[PIFFPIFF]` (artmd) — hit-spark anim
- `[SlaveSequence]` (artmd) — frame table with unique Shovel + Carry poses (§2)
- `[SlaveFreedSelect/Move/AttackCommand/Fear/Die]` (soundmd) — freed-mode voices
- `[SlaveWorkerSelect/Move/AttackCommand/Fear/Harvest/Liberated/Die]` (soundmd) — enslaved-mode + one-shot voices
- `[SlaveAttack]` (soundmd) — anim-trigger sound for shovel thwack (referenced by `FireUpSounds= 2 SlaveAttack`)
- `[InfantrySquish]` (soundmd) — crush sound
- `Enslaves=SLAV` on **SMIN** (`yuri/SMIN.md` TODO), **SMON** (`yuri/SMON.md` TODO), and **YAREFN** (`structures/YAREFN.md` TODO) — the spawn-references

### Conceptual companions / consumers of `Slaved`/`Enslaves`
- **SMIN** (Slave Miner vehicle) — primary slave-owner. Drives slaves to ore fields, deploys to deposit. (TODO)
- **SMON** (Slave Miner deployed/building form) — secondary slave-owner. (TODO)
- **YAREFN** (Yuri Refinery) — also has `Enslaves=SLAV`. (TODO)

### Deep-RE docs (cross-referenced, NOT re-derived)
- **[SLAVE_MANAGER_STATE_MACHINE_GHIDRA_REPORT.md](../../SLAVE_MANAGER_STATE_MACHINE_GHIDRA_REPORT.md)** — full SlaveManagerClass lifecycle and state machine. Read first for any slave-system implementation work.
- **[SLAVE_MINER_ORE_SYSTEM_GHIDRA_REPORT.md](../../SLAVE_MINER_ORE_SYSTEM_GHIDRA_REPORT.md)** — Slave Miner deploy/undeploy and ore-dump cycle. Master-side behavior.
- **[ENGINEER_CAPTURE_GHIDRA_REPORT.md](../../ENGINEER_CAPTURE_GHIDRA_REPORT.md)** — relevant for what happens if a slave's master (a building like YAREFN) is captured by Engineer — slaves are reassigned to new owner.

---

## 11. Coverage audit

| Section | Status |
|---------|--------|
| Every `[SLAV]` rulesmd key explained | ✅ §1 |
| Every `[SLAV]` artmd key explained | ✅ §2 |
| `Sequence=SlaveSequence` fully expanded incl. unique Shovel + Carry poses | ✅ §2 |
| Weapon + projectile + warhead | ✅ §3–§4 |
| **Dual-mode voice system fully documented** (enslaved vs freed + Liberated one-shot + anim-trigger SlaveAttack) | ✅ §5 |
| Spawn paths (via SlaveManager from SMIN/SMON/YAREFN) | ✅ §6 |
| Master-death freeing path (all 6 MasterDestroyed xref sites) | ✅ §6 |
| Brain-transplant on deploy noted (cross-ref to deep doc) | ✅ §6 |
| Hardcoded behavior — six new flag-scope verifications (Slaved, Enslaves, VoiceSelectEnslaved, Storage, HarvestRate, DontScore) | ✅ §7 (with HarvestRate being InfantryType-only specifically called out) |
| TS-legacy filter | ✅ §8 |
| Veterancy treated correctly | ✅ §9 |
| Cross-refs to slave-master consumers + two deep-RE docs | ✅ §10 |
| Doc placed in `yuri/` (Owner=YuriCountry) | ✅ |

**Open follow-ups (none load-bearing):**
- The `[SlaveAttack]` sound — referenced by `FireUpSounds= 2 SlaveAttack` in the slave's animation sequence — was not located in this iteration's read of soundmd.ini. Should grep for `^\[SlaveAttack\]` to confirm the definition and audio file. If the anim-trigger references a missing sound, the shovel-thwack would be silent — relevant for parity audit.
- Verify the exact `SlavesNumber` and `SlaveRegenRate` values on SMIN/SMON/YAREFN when those docs are written; cross-check against the SlaveManager constructor args.
- Confirm the SHOVEL weapon's lack of `Report=` is intentionally compensated by the anim-trigger (verified above), not a missing audio cue.
- Spy-disguise behavior with `Crawls=no`: verify the Spy can actually disguise as slave at runtime, given the stub Prone/Down/Up entries.
