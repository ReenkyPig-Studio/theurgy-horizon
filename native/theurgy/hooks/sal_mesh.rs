fn hook(
    mesher: &mut Mesher<'_>,
    region: &[[Option<&Chunk>; 3]; 3],
    sy: i32,
    out: &mut SectionMesh,
    scratch: &mut MeshScratch,
) -> ::core::ops::ControlFlow<SectionMesh> {
    // Wraps the original: run it, then add the sal ammoniac it cannot draw.
    let Some(wrapped) = fluid::theurgy_sal::Wrapped::enter() else {
        return ::core::ops::ControlFlow::Continue(());
    };
    let mut mesh = mesher.mesh_reusing(region, sy, std::mem::take(out), scratch);
    drop(wrapped);
    // An all-air section returns before the padded copy is filled: it would be a stale one.
    if region[1][1].is_some_and(|chunk| !chunk.sections[sy as usize].is_uniform(0)) {
        fluid::theurgy_sal::add(mesher, &scratch.padded, &mut mesh);
    }
    ::core::ops::ControlFlow::Break(mesh)
}
