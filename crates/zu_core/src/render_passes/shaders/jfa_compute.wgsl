@group(0) @binding(0) var scene_texture: texture_2d<f32>;

@group(1) @binding(0) var output_texture: texture_storage_2d<rgba8unorm, read_write>;

struct PushConstants {
    one_over_size: vec2<f32>,
    u_offset: f32,
    texture_size: vec2<f32>,
    passes: i32
}

var<push_constant> constants: PushConstants;

@compute @workgroup_size(8, 8)
fn fs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let pixelCoord = id.xy;
    let uv = (vec2<f32>(pixelCoord) + 0.5) / constants.texture_size;
    var alpha = textureLoad(scene_texture, pixelCoord, 0).a;
    textureStore(output_texture, vec2<i32>(pixelCoord), vec4(uv * alpha, 0.0, 1.0));
    for (var pass_i = 0; pass_i <= constants.passes; pass_i += 1) {
        var nearestSeed = vec4(-2.0);
        var nearestDist = 999999.9;

        for (var y = -1.0; y <= 1.0; y += 1.0) {
            for (var x = -1.0; x <= 1.0; x += 1.0) {
                var sample_uv: vec2<f32> = uv + vec2(x, y) * constants.u_offset * constants.one_over_size;

                if sample_uv.x < 0.0 || sample_uv.x > 1.0 || sample_uv.y < 0.0 || sample_uv.y > 1.0 { continue; }
                var texelCoord: vec2<i32> = vec2<i32>(sample_uv * constants.texture_size);
                var sample_value = textureLoad(output_texture, texelCoord);
                var sample_seed = sample_value.xy;
                if sample_seed.x != 0.0 || sample_seed.y != 0.0 {
                    var diff = sample_seed - uv;
                    var dist = dot(diff, diff);
                    if dist < nearestDist {
                        nearestDist = dist;
                        nearestSeed = sample_value;
                    }
                }
            }
        }
        textureStore(output_texture, vec2<i32>(pixelCoord), nearestSeed);
    }
}
