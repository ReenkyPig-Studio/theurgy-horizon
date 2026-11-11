// SPDX-License-Identifier: MIT
// The End portal surface for LogisticsNexusPortalLayer (Java 26.3
// content/apparatus/logisticsnexus/render/LogisticsNexusPortalLayer.java:
// AbstractEndPortalRenderer.submitSpecial(RenderTypes.endPortal(), ...)).
//
// Horizon draws vanilla's End portal and gateway in the block entity batch as
// position-only quads on `END_PORTAL_LAYER` (block_entity_render.rs
// `portal_cube`); entity.wgsl projects end_sky/end_portal from the screen for
// them. The public quad helpers only take atlas sprites, so this module lives
// beside `BlockEntityDraw` to add the same vertices under the draw's pose.

use super::*;

/// One `rendertype_end_portal` quad of `corners` (blocks, under `draw`'s
/// pose), wound outward as a model face: the render type culls back faces.
pub fn end_portal_quad(draw: &mut BlockEntityDraw, corners: [[f32; 3]; 4]) {
    let pose = draw.pose();
    for corner in corners {
        draw.vertices.push(Vertex {
            normal: horizon_client::mesher::CULL_BACK,
            pos: pose.transform_point3(Vec3::from_array(corner)).to_array(),
            uv: [0.0; 2],
            layer: horizon_client::block_entity_render::END_PORTAL_LAYER,
            color: 0,
            light: 0,
            glint_uv: [0.0, 0.0],
        });
    }
}
