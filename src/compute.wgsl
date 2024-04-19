struct Data {
    width: u32,
    height: u32,
};

@group(0) @binding(0) var<uniform> data: Data;
@group(1) @binding(0) var texture: texture_storage_2d<rgba8unorm, write>; 

@compute @workgroup_size(1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let u = f32(global_id.x) / f32(data.width);
    let v = f32(global_id.y) / f32(data.height);
    textureStore(texture, global_id.xy, vec4<f32>(u, v, 0, 1));
}
