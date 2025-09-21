// Bindings for the texture and sampler
@group(0) @binding(0) var my_sampler: sampler;
@group(0) @binding(1) var my_texture: texture_2d<f32>;

@group(0) @binding(2) var<uniform> ray_count: i32;
@group(0) @binding(3) var<uniform> size: vec2<f32>;
@group(0) @binding(4) var<uniform> accum_radiance: i32;
@group(0) @binding(5) var<uniform> max_steps: i32;
@group(0) @binding(6) var<uniform> enable_noise: i32;


struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

const PI: f32 = 3.14159265;
const TAU: f32 = 2.0 * PI;

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(my_texture, my_sampler, input.uv);
    return vec4(raymarch(color, input.uv).xyz, 1.0);
}

fn outOfBounds(uv: vec2<f32>) -> bool {
    return uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0;
}

fn rand22(n: vec2f) -> f32 { return fract(sin(dot(n, vec2f(12.9898, 4.1414))) * 43758.5453); }

fn raymarch(light: vec4<f32>, uv: vec2<f32>) -> vec4<f32> {
    if light.a > 0.1 {
        return light;
    }
    var one_over_ray_count: f32 = 1.0 / f32(ray_count);
    var tau_over_ray_count: f32 = TAU * one_over_ray_count;

    // Distinct random value for every pixel
    var noise: f32 = select(0.1, rand22(uv), bool(enable_noise));
    var radiance = vec4(0.0);

    for (var i = 0; i < ray_count; i++) {
        var angle = tau_over_ray_count * (f32(i) + noise);
        var rayDirectionUv = vec2(cos(angle), -sin(angle)) / size;
        var traveled = vec2(0.0);

        let initial_step: i32 = select(max(0, max_steps - 1), 0, bool(accum_radiance));
        for (var step = initial_step; step < max_steps; step++) {
              // Go the direction we're traveling (with noise)
            var sampleUv = uv + rayDirectionUv * f32(step);

            if sampleUv.x < 0.0 || sampleUv.x > 1.0 || sampleUv.y < 0.0 || sampleUv.y > 1.0 {
                break;
            }

            var sampleLight = textureSample(my_texture, my_sampler, sampleUv);
            if sampleLight.a > 0.5 {
                radiance += sampleLight;
                break;
            }
        }
    }

    return radiance * one_over_ray_count;
}
