struct Camera {
    view_pos: vec4<f32>;
    view_proj: mat4x4<f32>;
};
[[group(0), binding(0)]]
var<uniform> camera: Camera;

struct VertexInput {
    [[location(0)]] position: vec3<f32>;
    [[location(1)]] color: u32;
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] color: vec4<f32>;
};

fn linear(f: f32) -> f32 {
    let c = f / f32(255.0);
    if (c <= 0.04045) {
        return c / f32(12.92);
    } else {
        return pow(((c + f32(0.055)) / f32(1.055)), f32(2.4));
    }
}

[[stage(vertex)]]
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    let red = extractBits(model.color, u32(0), u32(8));
    let cast_red = linear(f32(red));

    let green = extractBits(model.color, u32(8), u32(8));
    let cast_green =  linear(f32(green));

    let blue = extractBits(model.color, u32(16), u32(8));
    let cast_blue =  linear(f32(blue));

    let pos = model.position / f32(180);
    out.color = vec4<f32>(cast_red, cast_green, cast_blue, 1.0);
    out.clip_position = camera.view_proj * vec4<f32>(pos, 1.0);
    return out;
}

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    return in.color;
}
