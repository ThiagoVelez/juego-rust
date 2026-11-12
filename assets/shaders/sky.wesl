// Cielo de SurvivalForest: degradado cénit-horizonte, sol, luna, estrellas y
// nubes planas de bloques que avanzan con el viento.
import bevy_pbr::render::forward_io::VertexOutput;
import bevy_pbr::render::mesh_view_bindings::view;

struct SkyParams {
    zenith: vec4<f32>,
    horizon: vec4<f32>,
    sun: vec4<f32>,
    clouds: vec4<f32>,
};

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> sky: SkyParams;

fn hash2(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

// Ruido de valor suave: da forma a las nubes; luego se cuantiza por celdas.
fn vnoise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash2(i);
    let b = hash2(i + vec2<f32>(1.0, 0.0));
    let c = hash2(i + vec2<f32>(0.0, 1.0));
    let d = hash2(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let dir = normalize(in.world_position.xyz - view.world_position);
    let up = clamp(dir.y, 0.0, 1.0);
    var col = mix(sky.horizon.rgb, sky.zenith.rgb, pow(up, 0.5));
    if (dir.y < 0.0) {
        col = sky.horizon.rgb * (1.0 + dir.y * 0.35);
    }
    let night = sky.sun.w;
    let sun_dir = normalize(sky.sun.xyz);

    // Estrellas: celdas de la cúpula proyectada.
    if (night > 0.01 && dir.y > 0.0) {
        let p = dir.xz / (dir.y + 0.35) * 140.0;
        let cell = floor(p);
        let h = hash2(cell);
        if (h > 0.9965) {
            let twinkle = 0.6 + 0.4 * sin(sky.clouds.x * 2.0 + h * 50.0);
            col += vec3<f32>(0.9, 0.92, 1.0) * night * twinkle;
        }
    }

    // Sol: disco redondo con halo cálido.
    let toward = dot(dir, sun_dir);
    col += vec3<f32>(1.0, 0.85, 0.55) * pow(max(toward, 0.0), 64.0) * 0.35 * (1.0 - night);
    let side = normalize(cross(sun_dir, vec3<f32>(0.0, 0.0, 1.0)));
    let upv = cross(side, sun_dir);
    if (toward > 0.0) {
        let q = vec2<f32>(dot(dir, side), dot(dir, upv)) / toward;
        let r = length(q);
        let disc = 1.0 - smoothstep(0.040, 0.046, r);
        col = mix(col, vec3<f32>(1.0, 0.96, 0.82) * 1.6, disc * (1.0 - night * 0.9));
    }
    // Luna, opuesta al sol.
    if (toward < 0.0) {
        let q = vec2<f32>(dot(dir, side), dot(dir, upv)) / -toward;
        let r = length(q);
        let disc = 1.0 - smoothstep(0.028, 0.032, r);
        // Media luna: se recorta con un disco desplazado.
        let cut = 1.0 - smoothstep(0.024, 0.028, length(q - vec2<f32>(0.012, 0.006)));
        col = mix(col, vec3<f32>(0.88, 0.9, 0.97), disc * (1.0 - cut * 0.85) * (0.35 + night * 0.65));
    }

    // Nubes planas a 170 bloques por encima de la cámara.
    if (dir.y > 0.015) {
        let dist = 170.0 / dir.y;
        if (dist < 1600.0) {
            let wind = vec2<f32>(sky.clouds.x * 1.2, 0.0);
            let p = view.world_position.xz + sky.clouds.yz + dir.xz * dist + wind;
            // Celdas de 16 bloques: la forma sale del ruido suave en el centro de cada celda.
            let cell = floor(p / 16.0);
            let c = cell + vec2<f32>(0.5, 0.5);
            let cover = vnoise(c / 7.0) * 0.75 + vnoise(c / 2.5) * 0.25;
            if (cover > 0.64) {
                let lit = mix(vec3<f32>(0.22, 0.25, 0.36), vec3<f32>(1.0, 1.0, 1.0), 1.0 - night * 0.85);
                let fade = 1.0 - smoothstep(450.0, 1100.0, dist);
                col = mix(col, lit, 0.88 * fade);
            }
        }
    }
    return vec4<f32>(col, 1.0);
}
