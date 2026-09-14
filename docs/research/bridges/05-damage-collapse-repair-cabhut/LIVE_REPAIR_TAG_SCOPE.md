# Retail engineer Tag producer bounds

These bounds narrow the prerequisites of the attached engineer Tag callback at
`0x0051A010`, which raises event 48 before the engineer is uninitialized. They do
not establish that every possible attached Tag is inert.

Evidence uses the active `gamemd.exe`, SHA-256
`1cdd1180e49024fbda8ad568caac2e86e856063ff67ab38f62b7d2c7bb84298c`.
Addresses below refer to original instructions and data, independently read again
in September 2026; existing function names are not sufficient evidence.

## Ordinary vehicle crew

Unit's virtual slot `0x007F5F7C` points to `0x00740EE0`, an unconditional jump
to `0x00707D20`. That selector reads these Rules fields:

| Rules field | Original key | Reader store | Winning base type |
| --- | --- | --- | --- |
| `+0xF78` | AlliedCrew at `0x0083C60C` | `0x0066FCF1` | E1 |
| `+0xF7C` | SovietCrew at `0x0083C600` | `0x0066FD10` | E2 |
| `+0xF80` | ThirdCrew at `0x0083C5F4` | `0x0066FD2F` | INIT |
| `+0xF6C` | Technician at `0x0083C620` | `0x0066FCB3` | CTECH |

The side chooses among the three crew fields; fallback branches use Technician.
The separate General Engineer selector is Rules `+0xF70`, stored at
`0x0066FC94`; it is not read by this ordinary crew selector.

The winning RULESMD has SHA-256
`3d341ef8a13a4b5ab24af2eef48ac94931ac2bb87d950fe3330a07e2d25672ef`.
Merging its relevant fields with each of 184 extracted retail maps gives 740 crew
bindings. Only `sov06lmd.map` changes the three crew choices, to LUNR. None of the
five selected types has an Engineer assignment; the original InfantryType
constructor defaults that flag to false at `0x005237AD`.

All nine override filenames in the winning MPMODESMD were also inspected:
MPBattleMD, MPTeamMD, MPMWMD, MPDuelMD, MPMeatMD, MPNavalMD, MPFreeForAllMD,
MPUnholyMD and MPCoopMD. They contain no crew-selector, Engineer, Thief or
VehicleThief assignments and no InfantryTypes additions. Optional LANGRULE did
not resolve by name in this installation. These are bounded retail-input facts,
not a claim about other installations, arbitrary modifications or saved memory.

## Retained captured type is a different producer

Unit damage checks instance `+0x338` before the ordinary crew branch. An index
other than -1 selects an InfantryType at `0x0073823B`; the survivor can receive
the Unit's Tag at `0x0073837A`. The ordinary crew bounds alone cannot exclude this.

The direct capture writer at `0x00520465` lies in `0x005202F0`. Its first gate,
at `0x005202FF`, reads InfantryType `+0xEC5`. Original ReadINI binds that field
to **Thief**, key `0x0082594C`, with its store at `0x005245D2`.
**Engineer is instead `+0xEC3`**, key `0x0082596C`, stored at `0x00524584`.
Older reports naming this function an engineer capture path confuse those flags.

The same function requires a nonnull NavCom and target kind 1. Unit's actual kind
slot `0x007F5C9C` points to `0x00746E20`, which returns 1. Its outward Tag
transfer at `0x0052043F` and retained-type write follow those gates.
Thief defaults false at `0x005237BC`; the inspected Rules, maps and modes contain
no Thief or VehicleThief assignment.

The other two direct capture writes, `0x00519A42` and `0x00519F94`, receive
Buildings: the first follows the kind-6 gate at `0x005199BB`; the second retains
the result of `0x0047C520`, which returns only kind 6. Building sale's Tag transfer
at `0x0044A0BE` targets its configured undeployed Unit, created at
`0x00449E44`, rather than an engineer.

This does not exclude all computed field writes, bulk copies, later type changes,
Building survivor paths, Team membership changes or Tag attachment lifetimes.

## Building crews and AI Team inputs

Building crews have an additional Engineer producer. Building's `+0x30C` slot
`0x007E41C8` points to `0x0044EB10`. If instance `+0x6E3` is zero, the selector
draws from 0 through 99; a result below 25 and BuildingType `+0xEB8 == 7` selects
Rules `+0xF70`. Other branches use the ordinary crew selector above.
This is a conditional selector rule, not a universal survivor probability.

ReadINI stores Factory at `+0xEB8` at `0x00460545`. Its text reader and enum table
map BuildingType to 7 (`0x00474FF0`, `0x0040DCE0`, table entry `0x00816F18`).
Winning RULESMD gives GACNST, NACNST and YACNST `Factory=BuildingType` and
`Crewed=yes`, establishing retail instances of that type gate.

The survivor loop calls this selector at `0x004430B6` and constructs the selected
Infantry with the Building's owner at `0x004430DD`. Placement, Unlimbo and Scatter
follow conditionally. This caller contains no explicit instance Tag copy or direct
call to the Tag setter `0x005F5B50`; callee effects and later Team attachment remain
separate questions. Ordinary Unit crew exclusions cannot exclude these Engineers.

The winning AIMD from `ra2md.mix -> localmd.mix` has 163 named TeamTypes, all with
present sections, and no Tag assignments anywhere in that file (138,538 bytes;
asset provenance and an independent raw-input census checked). This bounds that
authored input only; constructor defaults, map or mode overrides, runtime Team
changes and attachment lifetimes are not established by absence of those keys.

## Trigger latch and enabled state survive a complete stream round-trip

A stored event mismatch is insufficient to prove a Tag callback inert:
`0x007264C0` checks Trigger enabled `+0x44` and retiring `+0x30`, then can skip
event evaluation when the corresponding `+0x40` bit is already latched.
Repeat mode 2 permits latching, including event 29. In `all04dmd.map`, the two
known Engineer death Tags use event 29 and their action lists disable their own
Trigger. Prior latches and subsequent enabling still require lifetime reasoning.

The enable entry `0x007268F0` sets `+0x44` and tail-jumps to `0x00726400`.
That reset selects timer events 13 and 51 for latch-bit clearing; the
single-event-29 lists used by these two Triggers retain their latch. This
does not exclude bit aliasing in larger mixed lists: the native 32-bit shift
wraps event indices modulo 32. The disable entry `0x00726900` only clears
`+0x44`. Thus disabling and enabling are not a general latch reset.

A literal census of all 142 `all04dmd.map` action lists, validating each
declared count against eight-token action records, finds three references to
the two Engineer death Trigger IDs: all are self-disable actions 54. No
authored action directly enables either ID. This narrows the authored
re-enabling path; it does not establish all runtime attachment, construction
or latch lifetimes.

Trigger's main table `0x007F5858` binds Load to `0x00726860`, Save to
`0x007268D0`, and object size to `0x00726930`, which returns `0x48`.
The base stream routines `0x00410320` and `0x00410380` request that whole object,
including both `+0x40` and `+0x44`. Load restores the old `+0x1C` only.
The subsequent no-init constructor and table restoration change interface pointers;
the three swizzles address `+0x24`, `+0x28` and `+0x2C`.

Consequently a **complete, valid** paired stream round-trip preserves latch and
enabled state together. The wrappers inspect negative HRESULT but request no
returned byte count: a nonnegative result from a truncated stream does not prove
a complete read. This bound does not establish general save validity or close the
engineer's event-48 callback.
