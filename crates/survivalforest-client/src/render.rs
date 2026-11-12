//! Material de bloques: array de texturas, AO, luz de cielo/bloque y niebla.

use crate::textures::{T, paint_tiles};
use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::mesh::{Indices, MeshVertexAttribute, MeshVertexBufferLayoutRef, PrimitiveTopology};
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, Extent3d, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError,
    TextureDimension, TextureFormat, TextureViewDescriptor, TextureViewDimension, VertexFormat,
};
use bevy::shader::ShaderRef;
use survivalforest_core::block::TILE_NAMES;
use survivalforest_world::MeshData;

pub const ATTRIBUTE_LAYER: MeshVertexAttribute =
    MeshVertexAttribute::new("BlockLayer", 917_331_201, VertexFormat::Uint32);
pub const ATTRIBUTE_LIGHT: MeshVertexAttribute =
    MeshVertexAttribute::new("BlockLight", 917_331_202, VertexFormat::Float32x2);
pub const ATTRIBUTE_AO: MeshVertexAttribute =
    MeshVertexAttribute::new("BlockAo", 917_331_203, VertexFormat::Float32);

const SHADER: &str = "shaders/blocks.wesl";

#[derive(ShaderType, Clone, Copy, Debug)]
pub struct BlockParams {
    /// Color de la niebla (lineal).
    pub fog_color: Vec4,
    pub sun_light: f32,
    pub fog_near: f32,
    pub fog_far: f32,
    /// 0 opaco, 1 recortado (hojas), 2 agua.
    pub mode: u32,
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct BlockMaterial {
    #[texture(0, dimension = "2d_array")]
    #[sampler(1)]
    pub tiles: Handle<Image>,
    #[uniform(2)]
    pub params: BlockParams,
}

impl Material for BlockMaterial {
    fn vertex_shader() -> ShaderRef {
        SHADER.into()
    }
    fn fragment_shader() -> ShaderRef {
        SHADER.into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        match self.params.mode {
            2 => AlphaMode::Blend,
            1 => AlphaMode::Mask(0.5),
            _ => AlphaMode::Opaque,
        }
    }
    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let vertex_layout = layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_NORMAL.at_shader_location(1),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(2),
            ATTRIBUTE_LAYER.at_shader_location(3),
            ATTRIBUTE_LIGHT.at_shader_location(4),
            ATTRIBUTE_AO.at_shader_location(5),
        ])?;
        descriptor.vertex.buffers = vec![vertex_layout];
        Ok(())
    }
}

/// Los tres materiales de bloques, que comparten el array de texturas.
#[derive(Resource)]
pub struct BlockMaterials {
    pub opaque: Handle<BlockMaterial>,
    pub cutout: Handle<BlockMaterial>,
    pub water: Handle<BlockMaterial>,
}

/// Crea la imagen del array de texturas (una capa por nombre de `TILE_NAMES`).
pub fn tiles_image() -> Image {
    let tiles = paint_tiles();
    let mut data = Vec::with_capacity(tiles.len() * T * T * 4);
    for (i, t) in tiles.iter().enumerate() {
        let custom = std::path::Path::new("assets/textures").join(format!("{}.png", TILE_NAMES[i]));
        match load_pack_png(&custom) {
            Some(px) => data.extend_from_slice(&px),
            None => data.extend_from_slice(t),
        }
    }
    let mut img = Image::new(
        Extent3d {
            width: T as u32,
            height: T as u32,
            depth_or_array_layers: tiles.len() as u32,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    img.texture_view_descriptor = Some(TextureViewDescriptor {
        dimension: Some(TextureViewDimension::D2Array),
        ..default()
    });
    img.sampler = ImageSampler::nearest();
    img
}

/// PNG de un TexturePack (32×32, o 16×16 ampliado); `None` si no existe o no encaja.
fn load_pack_png(path: &std::path::Path) -> Option<Vec<u8>> {
    let bytes = std::fs::read(path).ok()?;
    let img = Image::from_buffer(
        &bytes,
        bevy::image::ImageType::Extension("png"),
        bevy::image::CompressedImageFormats::NONE,
        true,
        ImageSampler::nearest(),
        RenderAssetUsages::RENDER_WORLD,
    )
    .ok()?;
    let rgba = img.convert(TextureFormat::Rgba8UnormSrgb)?;
    (rgba.width() == T as u32 && rgba.height() == T as u32)
        .then(|| rgba.data.clone())
        .flatten()
}

/// Color del horizonte (niebla y fondo) de día.
pub const HORIZON: Color = Color::srgb(0.843, 0.929, 0.965);

/// Niebla según la distancia de visión: empieza al 55 % y cubre el borde de lo cargado.
pub fn fog_range(view_distance: u32) -> (f32, f32) {
    let far = (view_distance as f32 * 16.0 - 10.0).max(24.0);
    (far * 0.55, far)
}

pub fn setup_materials(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut mats: ResMut<Assets<BlockMaterial>>,
    vd: Res<crate::streaming::ViewDistance>,
) {
    let (near, far) = fog_range(vd.0);
    let tiles = images.add(tiles_image());
    let p = |mode| BlockParams {
        fog_color: LinearRgba::from(HORIZON).to_vec4(),
        sun_light: 1.0,
        fog_near: near,
        fog_far: far,
        mode,
    };
    commands.insert_resource(BlockMaterials {
        opaque: mats.add(BlockMaterial {
            tiles: tiles.clone(),
            params: p(0),
        }),
        cutout: mats.add(BlockMaterial {
            tiles: tiles.clone(),
            params: p(1),
        }),
        water: mats.add(BlockMaterial {
            tiles,
            params: p(2),
        }),
    });
}

/// Convierte el mallado del mundo en un `Mesh` de Bevy.
pub fn build_mesh(d: &MeshData) -> Mesh {
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, d.positions.clone())
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, d.normals.clone())
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, d.uvs.clone())
    .with_inserted_attribute(ATTRIBUTE_LAYER, d.layer.clone())
    .with_inserted_attribute(ATTRIBUTE_LIGHT, d.light.clone())
    .with_inserted_attribute(ATTRIBUTE_AO, d.ao.clone())
    .with_inserted_indices(Indices::U32(d.indices.clone()))
}
