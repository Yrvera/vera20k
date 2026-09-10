//! Map577D90/577AB0 bulk transitions bracketed by4ADEE0/4ADCD0.
//! Native callbacks visit Techno registration order, ordinary sight then Gap on
//! each object. House240 idempotence is distinct from House577A SpySatActive.
use super::{FogState, InternedId, OwnerVisibility, gap_footprint};

impl FogState {
    pub(crate) fn transition_whole_map_for_owner(
        &mut self,
        owner: InternedId,
        cells: Vec<(u16, u16)>,
        reset: bool,
        old_spy_sat_active: bool,
    ) {
        // Synthetic/map-bootstrap callers have no live object registry. Only
        // own-viewer admitted sources belong to their represented callback set.
        let source_ids = self
            .sight_admissions
            .iter()
            .filter_map(|(&(id, viewer), source)| {
                (viewer == owner && source.owner == owner).then_some(id)
            })
            .collect();
        self.transition_whole_map_with_sources(
            owner,
            cells,
            reset,
            old_spy_sat_active,
            &source_ids,
        );
    }

    pub(crate) fn transition_whole_map_with_sources(
        &mut self,
        owner: InternedId,
        cells: Vec<(u16, u16)>,
        reset: bool,
        old_spy_sat_active: bool,
        source_ids: &std::collections::BTreeSet<u64>,
    ) {
        if self.width == 0
            || self.height == 0
            || (!reset && self.whole_map_revealed_owners.contains(&owner))
        {
            return;
        }
        let sources: std::collections::BTreeMap<_, _> = self
            .sight_admissions
            .iter()
            .filter(|(key, _)| key.1 == owner && source_ids.contains(&key.0))
            .map(|(key, value)| (key.0, value.clone()))
            .collect();
        let gaps = self.gap_sources.get(&owner).cloned().unwrap_or_default();
        let mut gaps_by_id: std::collections::BTreeMap<u64, Vec<_>> =
            std::collections::BTreeMap::new();
        for gap in gaps {
            gaps_by_id.entry(gap.stable_id).or_default().push(gap);
        }
        let ids: std::collections::BTreeSet<_> = sources
            .keys()
            .copied()
            .chain(gaps_by_id.keys().copied())
            .collect();
        let width = usize::from(self.width);
        let height = usize::from(self.height);
        self.by_owner
            .entry(owner)
            .or_insert_with(|| OwnerVisibility::new(self.width, self.height));
        //4ADEE0, per-object release then remove-gap. Activation sees OLD577A
        //false, deactivation OLD577Atrue; the House owner changes it afterwards.
        for &id in &ids {
            if sources.contains_key(&id) {
                self.release_sight_for_viewer((id, owner));
            }
            for &gap in gaps_by_id.get(&id).into_iter().flatten() {
                let vis = self.by_owner.get_mut(&owner).unwrap();
                for index in gap_footprint(gap, width, height) {
                    vis.shroud_knowledge[index].remove_gap(old_spy_sat_active);
                }
            }
        }
        self.gap_sources.remove(&owner);
        if !reset {
            self.whole_map_revealed_owners.insert(owner);
        }
        {
            let vis = self.by_owner.get_mut(&owner).unwrap();
            vis.ensure_cell_runtime();
            for (x, y) in cells {
                let Some(index) = vis.index(x, y) else {
                    continue;
                };
                let state = &mut vis.shroud_knowledge[index];
                state.counter = if reset { 1 } else { 0 };
                state.gap_counter = 0;
                state.open = !reset;
                // Both original bulk stores preserve Cell140 pending20.
                vis.publish_knowledge(index);
            }
        }
        //4ADCD0 repeats registration order, ordinary sight then gap. Stable IDs
        //are monotonic Techno construction identities in the current owner.
        for id in ids {
            if let Some(source) = sources.get(&id) {
                self.reconcile_sight_admission(id, owner, source.clone(), false, source.fog_of_war);
            }
            for &gap in gaps_by_id.get(&id).into_iter().flatten() {
                let vis = self.by_owner.get_mut(&owner).unwrap();
                for index in gap_footprint(gap, width, height) {
                    vis.shroud_knowledge[index].add_gap();
                    vis.publish_knowledge(index);
                }
                self.gap_sources.entry(owner).or_default().insert(gap);
            }
        }
        //577AB0 clears240 only after callback re-admission.
        if reset {
            self.whole_map_revealed_owners.remove(&owner);
        }
        self.view_cache.merged = None;
    }
}
