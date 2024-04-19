struct Data {
    width: u32,
    height: u32,
    t: f32,
};

@group(0) @binding(0) var<uniform> data: Data;
@group(1) @binding(0) var texture: texture_storage_2d<rgba8unorm, write>; 

@compute @workgroup_size(1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let u = f32(global_id.x) / f32(data.width);
    let v = f32(global_id.y) / f32(data.height);

    let aspect_ratio = f32(data.width) / f32(data.height);
    var x = (u * 2 - 1) * aspect_ratio;
    let y = v * 2 - 1;
    var color = vec3<f32>(0);

    let ro = vec3<f32>(0, 0, -3);
    let rd = normalize(vec3<f32>(x, y, 1));

    var t = 0.;

    // ray marching
    for (var i = 0; i < 80; i++) {
        let p = ro + rd * t;
        let d = sdf(p);

        t += d;
        color = vec3<f32>(i) / 80;

        if d < 0.0001 || t > 100 {
            break;
        }
    }

    color = vec3<f32>(t * 0.2);

    textureStore(texture, vec2<u32>(global_id.xy), vec4<f32>(color, 1));
}

fn sdf(p: vec3<f32>) -> f32 {
    return length(p) - 1 + sin(data.t * 3) / 2;
}

