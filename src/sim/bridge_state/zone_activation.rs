//! Original56DB70: record owner and synchronous activation callbacks.
//! Evidence: spatial_oracle/bridge_repair_zones and bridge_records.
use super::*;
use crate::sim::pathfinding::zone_build::find_high_bridge_record_index;

impl BridgeRuntimeState {
    /// Initial miss recomputes the native record vector once. Every matching
    /// inactive record is activated before its edge/reachability callback;
    /// an unsupported callback retains all preceding native-visible writes.
    pub(crate) fn validate_repaired_zones(
        &mut self,
        terrain: &ResolvedTerrainGrid,
        query: (i16, i16),
        mut activated: impl FnMut(&BridgeEndpointRecord) -> Result<bool, String>,
    ) -> Result<bool, String> {
        let query = (query.0 as u16, query.1 as u16);
        let mut next = find_high_bridge_record_index(&self.endpoint_records, 0, query, 3);
        if next.is_none() {
            self.endpoint_records = record_scan::compute_bridge_endpoints(
                terrain,
                self.native_zone_source_size,
                &self.cells,
            );
            next = find_high_bridge_record_index(&self.endpoint_records, 0, query, 3);
        }
        let mut connectivity = false;
        while let Some(index) = next {
            let record = &mut self.endpoint_records[index];
            if !record.active {
                record.active = true;
                connectivity |= activated(record)?;
            }
            next = find_high_bridge_record_index(&self.endpoint_records, index + 1, query, 3);
        }
        Ok(connectivity)
    }
}
