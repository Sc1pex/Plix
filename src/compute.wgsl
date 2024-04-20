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

    var finalColor = vec3<f32>(0.);

    for (var i = 0; i < 4; i++) {
        uv = fract(uv * 1.69) - 0.5;

        var l = length(uv) * exp(-length(uv0));
        var color = palette(length(uv0) + f32(i) * 0.8 + data.t * 0.8);

        l = sin(l * 7. + data.t) / 7.;
        l = abs(l);

        l = pow(0.01 / l, 1.3);

        finalColor += (color * l);
    }

    textureStore(texture, global_id.xy, vec4<f32>(finalColor, 1));
    return;
}

fn raymarching() {
    let ro = vec3<f32>(0, 0, -3);
    let uv = vec2<f32>(0);
    let rd = normalize(vec3<f32>(uv, 1));

    var t = 0.;

    // ray marching
    for (var i = 0; i < 80; i++) {
        let p = ro + rd * t;
        let d = sdf(p);

        t += d;

        if d < 0.00001 || t > 100 {
            break;
        }
    }

    let color = vec3<f32>(t * 0.2);

    // textureStore(texture, vec2<u32>(global_id.xy), vec4<f32>(color, 1));
}

fn palette(t: f32) -> vec3<f32> {
    let a = vec3<f32>(0.5, 0.5, 0.5);
    let b = vec3<f32>(0.5, 0.34, 0.5);
    let c = vec3<f32>(1.1, 1.2, 1.0);
    let d = vec3<f32>(0.24, 0.4, 0.42);
    return a + b * cos(6.28318 * (c * t + d));
}

fn sdf(p: vec3<f32>) -> f32 {
    let spherePos = vec3<f32>(cos(data.t * 3), sin(data.t * 5), 0);
    let sphere = sdfSphere(p - spherePos, 0.5);
    let box = sdfBox(p, vec3<f32>(1.));

    let ground = p.y + 0.75;

    return smin(smin(sphere, box, 1.), ground, 0.1);
    // return sdfIntersect(sphere, box);
    // return sdfSubtract(sphere, box);
    // return sdfSubtract(box, sphere);
}

fn smin(a: f32, b: f32, k: f32) -> f32 {
    let h = max(k - abs(a - b), 0.) / k;
    return min(a, b) - h * h * h * k * (1. / 6.);
}

fn sdfSphere(p: vec3<f32>, r: f32) -> f32 {
    return length(p) - r;
}

fn sdfBox(p: vec3<f32>, size: vec3<f32>) -> f32 {
    let q = abs(p) - size;
    return length(max(q, vec3<f32>(0))) + min(max(q.x, max(q.y, q.z)), 0.0);
}

fn sdfUnion(p1: f32, p2: f32) -> f32 {
    return min(p1, p2);
}

fn sdfSubtract(p1: f32, p2: f32) -> f32 {
    return max(-p1, p2);
}

fn sdfIntersect(p1: f32, p2: f32) -> f32 {
    return max(p1, p2);
}

