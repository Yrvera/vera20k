//! Assembly of shared tactical palette and stored-depth shader mechanisms.

pub(crate) fn source(body: &str) -> String {
    super::palette_light::shader_source(&format!("{}\n{}", include_str!("native_z.wgsl"), body))
}
