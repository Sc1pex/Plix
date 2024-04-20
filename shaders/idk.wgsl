struct Data {
    width: u32,
    height: u32,
    t: f32,
};

@group(0) @binding(0) var<uniform> data: Data;
@group(1) @binding(0) var texture: texture_storage_2d<rgba8unorm, write>; 

@compute @workgroup_size(1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let size = vec2<f32>(f32(data.width), f32(data.height));
    let coord = vec2<f32>(f32(global_id.x), f32(global_id.y));
    var uv = (coord * 2. - size) / size.y;
    let uv0 = uv;

    var finalColor = vec3<f32>(0.2);

    textureStore(texture, global_id.xy, vec4<f32>(finalColor, 1));
    return;
}

fn palette(t: f32) -> vec3<f32> {
    let a = vec3<f32>(0.5, 0.5, 0.5);
    let b = vec3<f32>(0.5, 0.34, 0.5);
    let c = vec3<f32>(1.1, 1.2, 1.0);
    let d = vec3<f32>(0.24, 0.4, 0.42);
    return a + b * cos(6.28318 * (c * t + d));
}
