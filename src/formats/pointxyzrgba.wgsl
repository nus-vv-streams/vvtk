struct Camera {
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
}

@group(0) @binding(0) var<uniform> camera: Camera;

struct AntiAlias {
    x: f32,
    y: f32,
    z: f32,
    scale: f32,
}

@group(1) @binding(0) var<uniform> antialias: AntiAlias;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

fn linear_transform(f: f32) -> f32 {
    let c = f / f32(255.0);
    if (c <= 0.04045) {
        return c / f32(12.92);
    } else {
        return pow(((c + f32(0.055)) / f32(1.055)), f32(2.4));
    }
}

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    let red = extractBits(model.color, u32(0), u32(8));
    let cast_red = linear_transform(f32(red));

    let green = extractBits(model.color, u32(8), u32(8));
    let cast_green =  linear_transform(f32(green));

    let blue = extractBits(model.color, u32(16), u32(8));
    let cast_blue =  linear_transform(f32(blue));
    let position = vec3<f32>(model.position[0] - antialias.x, model.position[1] - antialias.y, model.position[2] - antialias.z);
    let pos = position / antialias.scale;
    out.color = vec4<f32>(cast_red, cast_green, cast_blue, 1.0);
    out.clip_position = camera.view_proj * vec4<f32>(pos, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
