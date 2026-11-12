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
    mut world: ResMut<World>,
    mut dome: Query<&mut Transform, With<SkyDome>>,
    cam: Single<&GlobalTransform, With<Camera3d>>,
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
            clouds: Vec4::new(time.elapsed_secs(), o.x as f32, o.z as f32, 0.0),
        };
    }
    for h in [&blocks.opaque, &blocks.cutout, &blocks.water] {
        if let Some(mut m) = block_mats.get_mut(h) {
            m.params.sun_light = wt.sun_light();
            m.params.fog_color = hor.to_vec4();
        }
    }
    clear.0 = Color::from(hor);
    world.meta.time_ticks = wt.ticks;
    for mut t in &mut dome {
        t.translation = cam.translation();
    }
}
