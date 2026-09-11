//! Resident pristine inputs for the active-retail middle-bridge Recalc callers.
//!
//! 576BA0/56EB80 and the terminal586990 rectangle need both middle families,
//! including M+4. The source set is an admission boundary, not a native registry:
//! a missing cached head must never become native's invalid/sparse fallback.

use super::*;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum BridgeRecalcCatalogError {
    #[error("bridge Recalc has no resident metadata for registered tile {tile}")]
    Uncached { tile: i32 },
    #[error("bridge Recalc tile {tile} is unavailable: {reason}")]
    Unavailable { tile: u16, reason: String },
    #[error("bridge Recalc tile {tile} requires unsupported {effect}")]
    Unsupported { tile: u16, effect: &'static str },
    #[error(
        "bridge tile replacement requires a valid resident tile/subtile, received {tile}/{sub}"
    )]
    InvalidReplacement { tile: i32, sub: u8 },
}

#[derive(Debug)]
pub(crate) struct BridgeRecalcCatalog {
    heads: HashMap<u16, Result<Box<[TileMetadata]>, BridgeRecalcCatalogError>>,
    permissions: Vec<(bool, bool)>,
    pub(super) terrain_rules: TerrainRules,
    pub(super) lat_config: Option<lat::LatConfig>,
    pub(super) slope_config: Option<lat::SlopeFixupConfig>,
    pub(super) cliff_back_impassability: u8,
    automatic_tube_bases: [i32; 4],
    wood_bridge_start: Option<u16>,
}

impl BridgeRecalcCatalog {
    pub(super) fn for_middle_bridges(
        theater: &TheaterData,
        assets: &crate::assets::asset_manager::AssetManager,
        rules: &TerrainRules,
        lat_enabled: bool,
        cliff_back_impassability: u8,
    ) -> Self {
        let family = super::super::bridge_rim_tiles::HighBridgeRimTiles::from_theater(theater);
        // The stock Lunar map has no middle-family aliases before or after
        // ordinary loading. Preserve an empty source catalog there, rather
        // than interpreting unrelated ZMM ramp aliases as bridge assets.
        let ids = theater.bridge_set.into_iter().flat_map(|_| {
            family.middle.into_iter().flat_map(move |middle| {
                (0..5).map(move |variant| {
                    family
                        .base
                        .wrapping_add(middle)
                        .wrapping_sub(1)
                        .wrapping_add(variant)
                })
            })
        });
        Self::for_tiles(
            theater,
            assets,
            rules,
            lat_enabled,
            cliff_back_impassability,
            ids,
        )
    }

    pub(super) fn for_tiles(
        theater: &TheaterData,
        assets: &crate::assets::asset_manager::AssetManager,
        rules: &TerrainRules,
        lat_enabled: bool,
        cliff_back_impassability: u8,
        ids: impl IntoIterator<Item = i32>,
    ) -> Self {
        let permissions = (0..theater.lookup.len())
            .map(|id| {
                (
                    theater.lookup.is_morphable(id as u16),
                    theater.lookup.allows_tiberium(id as u16),
                )
            })
            .collect();
        let ini = crate::rules::ini_parser::IniFile::from_bytes(&theater.ini_data);
        let mut heads = HashMap::new();
        let mut warned = HashSet::new();
        for id in ids {
            let Ok(tile) = u16::try_from(id) else {
                continue;
            };
            if usize::from(tile) >= theater.lookup.len() || heads.contains_key(&tile) {
                continue;
            }
            let result = (|| {
                let ini = ini
                    .as_ref()
                    .map_err(|error| BridgeRecalcCatalogError::Unavailable {
                        tile,
                        reason: format!("theater INI: {error}"),
                    })?;
                let set = theater.lookup.tileset_index(tile);
                let set_name = set.and_then(|set| theater.lookup.set_name(set));
                let set_section = set.map(|set| format!("TileSet{set:04}"));
                // This narrow adapter supports the independently proved
                // absence of shadow policy. It does not interpret TS shadow
                // options or silently suppress a declared native callback.
                if [set_section.as_deref(), set_name]
                    .into_iter()
                    .flatten()
                    .filter_map(|name| ini.section(name))
                    .any(|section| {
                        section.get("ShadowCaster").is_some()
                            || section.get("ShadowTiles").is_some()
                    })
                {
                    return Err(BridgeRecalcCatalogError::Unsupported {
                        tile,
                        effect: "declared shadow policy",
                    });
                }
                if theater.lookup.tile_anim(tile).is_some() {
                    return Err(BridgeRecalcCatalogError::Unsupported {
                        tile,
                        effect: "terrain animation attachment",
                    });
                }
                // Radar47C060 and tactical draw choose files independently of
                // Recalc's pristine receiver. Stock middle heads have one file;
                // a multi-file type needs its native coordinate selector.
                if theater.lookup.total_file_count(tile) > 1 {
                    return Err(BridgeRecalcCatalogError::Unsupported {
                        tile,
                        effect: "coordinate-selected TMP file variants",
                    });
                }
                let filename = theater.lookup.filename(id).ok_or_else(|| {
                    BridgeRecalcCatalogError::Unavailable {
                        tile,
                        reason: "no pristine filename".into(),
                    }
                })?;
                let bytes =
                    assets
                        .get(filename)
                        .ok_or_else(|| BridgeRecalcCatalogError::Unavailable {
                            tile,
                            reason: format!("missing {filename}"),
                        })?;
                let tmp = TmpFile::from_bytes(&bytes).map_err(|error| {
                    BridgeRecalcCatalogError::Unavailable {
                        tile,
                        reason: format!("{filename}: {error}"),
                    }
                })?;
                let mut entries = Vec::with_capacity(256);
                // Parse the file once. All unsigned Cell+11A values retain
                // their exact registered-entry validity, including holes and
                // positive out-of-bounds subtiles; no modulo normalization.
                for sub in 0..=u8::MAX {
                    let mut metadata = metadata_from_set_name(set_name, set);
                    metadata.tmp_file_valid = true;
                    merge_tmp_file_metadata(&mut metadata, &tmp, sub, Some(rules), &mut warned);
                    if metadata.yr_cell_land_type == YR_CELL_LAND_TUNNEL {
                        return Err(BridgeRecalcCatalogError::Unsupported {
                            tile,
                            effect: "Tube land",
                        });
                    }
                    if metadata.has_damaged_data {
                        return Err(BridgeRecalcCatalogError::Unsupported {
                            tile,
                            effect: "damaged TMP data",
                        });
                    }
                    entries.push(metadata);
                }
                Ok(entries.into_boxed_slice())
            })();
            heads.insert(tile, result);
        }
        Self {
            heads,
            permissions,
            terrain_rules: rules.clone(),
            lat_config: lat_enabled
                .then(|| lat::parse_lat_config(&theater.ini_data, &theater.lookup)),
            slope_config: lat_enabled.then_some(lat::SlopeFixupConfig {
                ramp_base: theater.rmg_tiles.ramp_base.map_or(-1, i32::from),
                ramp_smooth: theater.rmg_tiles.ramp_smooth.map_or(-1, i32::from),
            }),
            cliff_back_impassability,
            automatic_tube_bases: theater.automatic_tube_bases,
            wood_bridge_start: theater
                .wood_bridge_set
                .and_then(|set| theater.lookup.bounds().get(usize::from(set)))
                .map(|bounds| bounds.start),
        }
    }

    pub(super) fn metadata(
        &self,
        tile: i32,
        sub: u8,
    ) -> Result<Option<TileMetadata>, BridgeRecalcCatalogError> {
        if tile < 0 || tile == 0xFFFF || tile as usize >= self.permissions.len() {
            return Ok(None);
        }
        let entries = self
            .heads
            .get(&(tile as u16))
            .ok_or(BridgeRecalcCatalogError::Uncached { tile })?
            .as_ref()
            .map_err(Clone::clone)?;
        Ok(Some(entries[usize::from(sub)].clone()))
    }

    pub(super) fn current_permissions(&self, tile: i32) -> (bool, bool) {
        if tile < 0 || tile as usize >= self.permissions.len() {
            (self.permissions.first().is_some_and(|row| row.0), true)
        } else {
            self.permissions[tile as usize]
        }
    }

    pub(super) fn automatic_tube_direction(&self, tile: i32) -> Option<u8> {
        auto_tube_direction_from_bases(tile, self.automatic_tube_bases)
    }

    pub(super) fn is_wood_bridge_repair_tile(&self, tile: i32) -> bool {
        tile >= 0
            && tile != 0xFFFF
            && self
                .wood_bridge_start
                .is_some_and(|start| tile >= i32::from(start) && tile < i32::from(start) + 16)
    }
}

impl ResolvedTerrainGrid {
    /// The native56EB80 raw +38 store, admitted before changing live state.
    /// This live bridge adapter accepts the stock valid middle entries. The
    /// shared scalar Recalc still models native invalid/sparse branches.
    pub(crate) fn write_resident_bridge_tile(
        &mut self,
        index: usize,
        tile: i32,
        overlay: FinalizedOverlayCell,
        overlay_types: &OverlayTypeRegistry,
    ) -> Result<(), LoadCellRecalcError<&'static str>> {
        let catalog = self
            .bridge_recalc_catalog
            .as_ref()
            .ok_or(LoadCellRecalcError::MissingResidentInputs)?;
        let cell = self
            .cells
            .get(index)
            .ok_or(LoadCellRecalcError::CellIndexOutOfBounds { index })?;
        let sub = cell.final_sub_tile;
        let metadata = catalog
            .metadata(tile, sub)
            .map_err(LoadCellRecalcError::ResidentInput)?;
        if !metadata.is_some_and(|metadata| metadata.subtile_entry_valid == Some(true)) {
            return Err(LoadCellRecalcError::ResidentInput(
                BridgeRecalcCatalogError::InvalidReplacement { tile, sub },
            ));
        }
        match overlay.identity() {
            NO_OVERLAY_IDENTITY => {}
            identity @ 0..=254 => {
                let overlay_id = identity as u8;
                if overlay_types.flags(overlay_id).is_none() {
                    return Err(LoadCellRecalcError::MissingOverlayType { overlay_id });
                }
            }
            identity => return Err(LoadCellRecalcError::MalformedOverlayIdentity { identity }),
        }
        self.cells[index].final_tile_index = tile;
        Ok(())
    }

    /// Refresh final-type queries after47D2B0, without replacing its retained
    /// pristine land/slope/dimensions. Native radar47C060 reads current+38/+11A.
    /// The admitted live bridge set is valid, single-file and has no damaged
    /// plane; ClearTile radar/render fallback and file selection are separate
    /// native paths, never approximated here by stale colors or sub-tile zero.
    pub(crate) fn refresh_resident_bridge_presentation(
        &mut self,
        index: usize,
    ) -> Result<(), LoadCellRecalcError<&'static str>> {
        let catalog = self
            .bridge_recalc_catalog
            .as_ref()
            .ok_or(LoadCellRecalcError::MissingResidentInputs)?;
        let cell = self
            .cells
            .get(index)
            .ok_or(LoadCellRecalcError::CellIndexOutOfBounds { index })?;
        let tile = cell.final_tile_index;
        let sub = cell.final_sub_tile;
        let metadata = catalog
            .metadata(tile, sub)
            .map_err(LoadCellRecalcError::ResidentInput)?;
        let Some(metadata) = metadata.filter(|metadata| metadata.subtile_entry_valid == Some(true))
        else {
            return Err(LoadCellRecalcError::ResidentInput(
                BridgeRecalcCatalogError::InvalidReplacement { tile, sub },
            ));
        };
        let cell = &mut self.cells[index];
        cell.radar_left = metadata.radar_left;
        cell.radar_right = metadata.radar_right;
        cell.render_offset_x = metadata.render_offset_x;
        cell.render_offset_y = metadata.render_offset_y;
        cell.has_damaged_data = metadata.has_damaged_data;
        cell.variant = 0;
        cell.filled_clear = false;
        self.radar_color_valid[index] = true;
        self.damaged_radar_metadata[index] = None;
        Ok(())
    }

    /// Resident scalar47D2B0 adapter for the admitted stock bridge source set.
    /// Caller retains ownership of ordered overlay/radar/navigation publication.
    /// The grid is never moved out of its Simulation owner for a callback.
    pub(crate) fn recalc_resident_bridge_cell(
        &mut self,
        index: usize,
        overlay: FinalizedOverlayCell,
        level_override: i32,
        overlay_types: &OverlayTypeRegistry,
        current_bounds: Option<PlayfieldBounds>,
    ) -> Result<LoadCellRecalcOutcome, LoadCellRecalcError<&'static str>> {
        let catalog = self
            .bridge_recalc_catalog
            .clone()
            .ok_or(LoadCellRecalcError::MissingResidentInputs)?;
        let cell = self
            .cells
            .get(index)
            .ok_or(LoadCellRecalcError::CellIndexOutOfBounds { index })?;
        // The early-overlay branch writes land before querying pristine TMP
        // slope. Reject missing/unadmitted resident inputs before that write;
        // native invalid/sparse inputs remain admitted fallback cases.
        catalog
            .metadata(cell.final_tile_index, cell.final_sub_tile)
            .map_err(LoadCellRecalcError::ResidentInput)?;
        let mut state =
            LoadCellRecalcState::for_resident_bridge(&catalog, overlay_types, current_bounds);
        self.recalc_cell_attributes(
            &mut state,
            index,
            overlay,
            level_override,
            &mut RejectUnadmittedEffects,
        )
    }
}

/// The admitted stock source set has no lifecycle callbacks. Preserve an error
/// boundary if a future policy change invalidates that proof; never no-op one.
struct RejectUnadmittedEffects;

impl LoadCellRecalcEffects for RejectUnadmittedEffects {
    type Error = &'static str;

    fn allocate_automatic_tube(
        &mut self,
        _: AutomaticTubeRequest,
    ) -> Result<AutomaticTubeAllocation, Self::Error> {
        Err("runtime bridge Recalc requires an unadmitted Tube constructor")
    }

    fn construct_terrain_attached_anim(
        &mut self,
        _: &TerrainTileAnimation,
    ) -> Result<(), Self::Error> {
        Err("runtime bridge Recalc requires an unadmitted terrain animation")
    }
}
