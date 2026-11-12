// Bloques de SurvivalForest: textura del array, oclusión ambiental,
// luz de cielo (por la hora del día) y de bloque (cálida), sombreado por cara y niebla.
import bevy_pbr::render::mesh_functions::{get_world_from_local, mesh_position_local_to_clip};
import bevy_pbr::render::mesh_view_bindings::view;

struct BlockParams {
    fog_color: vec4<f32>,
    sun_light: f32,
    fog_near: f32,
    fog_far: f32,
    mode: u32,
};

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var tiles: texture_2d_array<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var tiles_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var<uniform> params: BlockParams;

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) layer: u32,
    @location(4) light: vec2<f32>,
    @location(5) ao: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) @interpolate(flat) layer: u32,
    @location(4) light: vec2<f32>,
    @location(5) ao: f32,
};

@vertex
fn vertex(v: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let world_from_local = get_world_from_local(v.instance_index);
    out.clip_position = mesh_position_local_to_clip(world_from_local, vec4<f32>(v.position, 1.0));
    out.world_position = (world_from_local * vec4<f32>(v.position, 1.0)).xyz;
    out.normal = v.normal;
    out.uv = v.uv;
    out.layer = v.layer;
    out.light = v.light;
    out.ao = v.ao;
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex = textureSample(tiles, tiles_sampler, in.uv, in.layer);
    if (params.mode == 1u && tex.a < 0.5) {
        discard;
    }
    // Sombreado por cara, como un sol alto y algo lateral.
    var face = 0.8;
    if (in.normal.y > 0.5) { face = 1.0; }
    else if (in.normal.y < -0.5) { face = 0.55; }
    else if (abs(in.normal.z) > 0.5) { face = 0.68; }
    let sky = pow(in.light.x, 1.4) * params.sun_light;
    let blk = pow(in.light.y, 1.4);
    let warm = vec3<f32>(1.0, 0.82, 0.58) * blk;
    let lit = max(vec3<f32>(sky), warm) + vec3<f32>(0.035);
    var color = tex.rgb * lit * face * in.ao * vec3<f32>(1.04, 1.0, 0.94);
    let d = distance(in.world_position, view.world_position);
    let fog = smoothstep(params.fog_near, params.fog_far, d);
    color = mix(color, params.fog_color.rgb, fog);
    var alpha = 1.0;
    if (params.mode == 2u) { alpha = 0.74; }
    return vec4<f32>(color, alpha);
}
