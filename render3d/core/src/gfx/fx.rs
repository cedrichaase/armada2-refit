//! Pipelines the effects share.

/// An effect's pipeline into the view: depth-tested (`LESSEQUAL`) and not written, red,
/// green and blue, no culling, blended as `blend`.
pub fn fx_desc(vs: gpu::Code, ps: gpu::Code, layout: &'static [gpu::VertexAttr], stride: u32, topology: gpu::Topology, blend: gpu::Blend) -> gpu::PipelineDesc {
    let mut d = gpu::PipelineDesc::new(vs, ps, layout, stride, topology, gpu::Format::Unknown);
    d.depth = gpu::Depth { test: true, write: false, compare: gpu::Compare::LessEqual, bias: 0.0, slope_bias: 0.0 };
    d.blend = Some(blend);
    d.write_mask = 7;
    d
}
