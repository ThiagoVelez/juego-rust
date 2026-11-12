//! Cielo y ciclo de día y noche: cúpula con degradado, sol, luna, estrellas y
//! nubes de bloques; la hora ajusta la luz de los bloques y la niebla.

use crate::render::{BlockMaterial, BlockMaterials};
use crate::streaming::{FloatingOrigin, World};
use bevy::light::NotShadowCaster;
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError,
};
use bevy::shader::ShaderRef;
use survivalforest_sim::time::WorldTime;

#[derive(ShaderType, Clone, Copy, Debug, Default)]
pub struct SkyParams {
    pub zenith: Vec4,
    pub horizon: Vec4,
    /// xyz: dirección al sol; w: noche (0..1).
    pub sun: Vec4,
    /// x: segundos; y, z: desplazamiento del origen flotante (bloques).
    pub clouds: Vec4,
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct SkyMaterial {
    #[uniform(0)]
    pub params: SkyParams,
}

impl Material for SkyMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/sky.wesl".into()
    }
    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // Se ve desde dentro de la esfera.
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}

/// Hora del mundo y velocidad (F3+T la multiplica por 100).
#[derive(Resource, Default)]
pub struct GameTime {
    pub time: WorldTime,
    pub fast: bool,
}

#[derive(Component)]
pub struct SkyDome;

#[derive(Resource)]
pub struct SkyHandle(pub Handle<SkyMaterial>);

pub fn setup_sky(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<SkyMaterial>>,
) {
    let m = mats.add(SkyMaterial {
        params: SkyParams::default(),
    });
    commands.insert_resource(SkyHandle(m.clone()));
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(900.0).mesh().uv(48, 24))),
        MeshMaterial3d(m),
        Transform::default(),
        NotShadowCaster,
        SkyDome,
    ));
}

pub fn advance_time(mut t: ResMut<GameTime>) {
    t.time.ticks += if t.fast { 100 } else { 1 };
}

pub fn toggle_fast(keys: Res<ButtonInput<KeyCode>>, mut t: ResMut<GameTime>) {
    if keys.pressed(KeyCode::F3) && keys.just_pressed(KeyCode::KeyT) {
        t.fast = !t.fast;
    }
}

/// Periodo con el que se envuelve el origen de las nubes (múltiplo de su patrón:
/// celdas de 16 bloques y ruido de 7 celdas), para no perder precisión lejos del origen.
pub const CLOUD_WRAP: i64 = 16 * 7 * 1024;

pub fn cloud_origin(o: survivalforest_core::BlockPos) -> Vec2 {
    Vec2::new(
        (o.x as i64).rem_euclid(CLOUD_WRAP) as f32,
        (o.z as i64).rem_euclid(CLOUD_WRAP) as f32,
    )
}

/// La luz del cielo se redondea a 1/128: cambia pocas veces por minuto y no fuerza
/// a reespecializar todos los chunks en cada fotograma.
pub fn quantize(v: f32) -> f32 {
    (v * 128.0).round() / 128.0
}

fn lin(c: [f32; 3]) -> LinearRgba {
    LinearRgba::from(Color::srgb(c[0], c[1], c[2]))
}

/// Aplica la hora: cielo, luz de los bloques, niebla y color de fondo.
pub fn apply_time(
    time: Res<Time>,
    gt: Res<GameTime>,
    origin: Res<FloatingOrigin>,
    sky: Res<SkyHandle>,
    blocks: Res<BlockMaterials>,
    mut sky_mats: ResMut<Assets<SkyMaterial>>,
    mut block_mats: ResMut<Assets<BlockMaterial>>,
    mut clear: ResMut<ClearColor>,
    world: Option<ResMut<World>>,
    vd: Res<crate::streaming::ViewDistance>,
    mut dome: Query<&mut Transform, With<SkyDome>>,
    cams: Query<&GlobalTransform, (With<Camera3d>, Without<SkyDome>)>,
) {
    let wt = gt.time;
    let (zen, hor) = wt.sky_colors();
    let (zen, hor) = (lin(zen), lin(hor));
    let sun = wt.sun_dir();
    let o = origin.0.origin();
    if let Some(mut m) = sky_mats.get_mut(&sky.0) {
        m.params = SkyParams {
            zenith: zen.to_vec4(),
            horizon: hor.to_vec4(),
            sun: sun.extend(wt.night()),
            clouds: {
                let c = cloud_origin(o);
                Vec4::new(time.elapsed_secs(), c.x, c.y, 0.0)
            },
        };
    }
    // Solo se escriben los materiales de bloques cuando algo cambia de verdad.
    let (near, far) = crate::render::fog_range(vd.0);
    let sun_light = quantize(wt.sun_light());
    let fog = Vec4::new(
        quantize(hor.red),
        quantize(hor.green),
        quantize(hor.blue),
        1.0,
    );
    for h in [&blocks.opaque, &blocks.cutout, &blocks.water] {
        let same = block_mats.get(h).is_some_and(|m| {
            m.params.sun_light == sun_light
                && m.params.fog_color == fog
                && m.params.fog_near == near
                && m.params.fog_far == far
        });
        if !same && let Some(mut m) = block_mats.get_mut(h) {
            m.params.sun_light = sun_light;
            m.params.fog_color = fog;
            m.params.fog_near = near;
            m.params.fog_far = far;
        }
    }
    clear.0 = Color::from(hor);
    if let Some(mut w) = world {
        w.meta.time_ticks = wt.ticks;
    }
    if let Some(cam) = cams.iter().next() {
        for mut t in &mut dome {
            t.translation = cam.translation();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cloud_origin_stays_small_far_away() {
        let v = cloud_origin(survivalforest_core::BlockPos::new(
            29_999_000,
            0,
            -29_999_000,
        ));
        assert!(v.x.abs() < 120_000.0 && v.y.abs() < 120_000.0, "{v:?}");
        // Cerca del origen no cambia nada.
        assert_eq!(
            cloud_origin(survivalforest_core::BlockPos::new(1600, 0, -320)),
            Vec2::new(1600.0, CLOUD_WRAP as f32 - 320.0)
        );
    }

    #[test]
    fn sun_light_is_quantized() {
        assert_eq!(quantize(0.500_01), quantize(0.500_02));
        assert!((quantize(0.73) - 0.73).abs() <= 1.0 / 128.0);
    }
}
