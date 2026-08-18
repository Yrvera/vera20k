# Miner / Refinery Research Docs

This folder groups miner, harvester, Chrono Miner, refinery dock, refinery storage,
and related trace reports.

Canonical current finding for stock `CMIN/HARV -> GAREFN/NAREFN` unload:

- normal stock refinery unload does not use reciprocal `unit+0x2E4 <-> building+0x2E4`
  dock links;
- admission uses refinery radio logic and accepted dock cell `building NW + (3,1)`;
- far/fallback staging uses `QueueingCell`, not the accepted dock anchor;
- unload runs through the mission `0x10` / `Mission_Deploy_Building` FSM;
- the unload FSM rediscovers the refinery by adjacent-cell lookup using the
  `DAT_0089F6A0` offset pair equivalent;
- post-unload exit clears the dock-active state and returns the miner to Harvest.

Reports that discuss `ReleaseDockedHarvester` or reciprocal `+0x2E4` links are still
useful evidence, but that path is not the normal stock refinery unload completion
path unless a nonzero dock link already exists.
