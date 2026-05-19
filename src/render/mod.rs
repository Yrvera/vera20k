//! GPU rendering subsystem using wgpu (WebGPU API).
//!
//! Handles all visual output: terrain tiles, sprites, voxel models, UI overlays.
//! Uses wgpu for cross-platform GPU access (Vulkan, DX12, Metal backends).
//!
//! ## Architecture
//! - `gpu.rs` — wgpu device/queue/surface initialization and frame management
//! - `batch.rs` — instanced sprite batch renderer (textured quads via GPU instancing)
//! - `sprite_atlas.rs` — SHP sprite atlas builder (infantry, buildings)
//! - `unit_atlas.rs` — VXL unit atlas builder (vehicles, voxel models)
//! - `vxl_raster.rs` — software voxel rasterizer (renders .vxl to sprite textures)
//!
//! ## Dependency rules
//! - render/ may READ from: assets/, map/, sim/
//! - render/ NEVER mutates sim state — strictly read-only access
//! - render/ does NOT depend on: ui/, sidebar/, audio/, net/

pub mod batch;
pub mod bink_movie;
pub mod bit_font;
pub mod bridge_atlas;
pub mod bridge_railing_atlas;
pub mod cursor_atlas;
pub mod egui_integration;
pub mod gpu;
pub mod main_menu_shell_chrome;
pub mod minimap;
mod minimap_helpers;
pub mod overlay_atlas;
pub mod palette_textures;
pub mod pixel_fx_sparkles;
pub mod radar_anim;
pub mod screenshot;
pub mod selection_overlay;
pub mod shell_text;
pub mod shroud_buffer;
pub mod sidebar_cameo_atlas;
pub mod sidebar_chrome;
pub mod sidebar_text;
pub mod skirmish_shell_chrome;
pub mod smudge;
pub mod sprite_atlas;
pub mod tile_atlas;
pub mod unit_atlas;
pub mod upscale_pass;
#[cfg(test)]
mod voxel_parity_tests;
pub mod vxl_compute;
pub mod vxl_normals;
pub mod vxl_raster;
