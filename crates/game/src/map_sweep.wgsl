#import bevy_pbr::{forward_io::VertexOutput, pbr_fragment::pbr_input_from_standard_material, pbr_functions::alpha_discard, mesh_view_bindings::view}
@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> bad: vec4<f32>;
fn linear(v: vec3<f32>) -> vec3<f32> {
    return select(pow((v+vec3<f32>(0.055))/1.055,vec3<f32>(2.4)),v/12.92,v<=vec3<f32>(0.04045));
}
@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) front: bool) -> @location(0) vec4<f32> {
    let pbr = pbr_input_from_standard_material(in,front);
    let base = alpha_discard(pbr.material,pbr.material.base_color);
    let distance = length(in.world_position.xyz-view.world_position);
    let code = floor(clamp(log2(1.0+distance)/16.0,0.0,1.0)*65534.0)+1.0;
    let bytes = vec3<f32>(floor(code/256.0),code%256.0,bad.x*255.0)/255.0;
    return vec4<f32>(linear(bytes),1.0+base.a*0.0);
}
