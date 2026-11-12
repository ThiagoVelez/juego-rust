//! Carga y descarga de chunks alrededor del jugador, en tareas asíncronas.

use crate::render::{BlockMaterials, build_mesh};
use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task, futures::check_ready};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use survivalforest_core::block::AIR;
use survivalforest_core::{BlockId, BlockPos, CHUNK, ChunkPos};
use survivalforest_world::{
    ChunkAccess, ChunkColumn, ChunkMeshes, RegionStore, WorldGen, WorldMeta, light_column,
    mesh_column, relight_around,
};

/// Chunks en espiral desde el centro, del más cercano al más lejano.
pub fn spiral(center: ChunkPos, r: i32) -> Vec<ChunkPos> {
    let mut v = Vec::with_capacity(((2 * r + 1) * (2 * r + 1)) as usize);
    for dz in -r..=r {
        for dx in -r..=r {
            v.push(ChunkPos::new(center.x + dx, center.z + dz));
        }
    }
    v.sort_by_key(|p| (center.dist2(*p), p.z, p.x));
    v
}

/// Chunks cargados que quedan fuera del radio `keep` (en Chebyshev) y hay que descargar.
pub fn to_unload(
    loaded: impl Iterator<Item = ChunkPos>,
    center: ChunkPos,
    keep: i32,
) -> Vec<ChunkPos> {
    loaded
        .filter(|p| (p.x - center.x).abs() > keep || (p.z - center.z).abs() > keep)
        .collect()
}

/// Posición de render (f32) de un bloque relativa al origen flotante.
pub fn to_render(p: BlockPos, origin: ChunkPos) -> Vec3 {
    let o = origin.origin();
    Vec3::new((p.x - o.x) as f32, p.y as f32, (p.z - o.z) as f32)
}

/// Mundo cargado.
#[derive(Resource)]
pub struct World {
    pub gen_: Arc<WorldGen>,
    pub columns: HashMap<ChunkPos, ChunkColumn>,
    pub store: RegionStore,
    pub meta: WorldMeta,
    pub dir: PathBuf,
}

impl ChunkAccess for World {
    fn column(&self, p: ChunkPos) -> Option<&ChunkColumn> {
        self.columns.get(&p)
    }
    fn column_mut(&mut self, p: ChunkPos) -> Option<&mut ChunkColumn> {
        self.columns.get_mut(&p)
    }
}

impl World {
    /// Guarda en disco todos los chunks modificados.
    pub fn save_all(&mut self) {
        for c in self.columns.values_mut() {
            if c.dirty {
                if let Err(e) = self.store.save(c) {
                    warn!("No se pudo guardar el chunk: {e}");
                }
                c.dirty = false;
            }
        }
        if let Err(e) = self.store.flush() {
            warn!("No se pudo escribir el mundo: {e}");
        }
    }
}

#[derive(Resource)]
pub struct ViewDistance(pub u32);

#[derive(Resource, Default)]
pub struct FloatingOrigin(pub ChunkPos);

/// Pide volver a mallar un chunk.
#[derive(Message, Clone, Copy)]
pub struct RemeshRequest(pub ChunkPos);

/// Posición del jugador en el mundo (f64), la usan la transmisión y la cámara.
#[derive(Resource, Default, Clone, Copy)]
pub struct Focus(pub bevy::math::DVec3);

#[derive(Component)]
pub struct ChunkMesh(pub ChunkPos);

#[derive(Resource, Default)]
pub struct Streaming {
    gen_tasks: HashMap<ChunkPos, Task<ChunkColumn>>,
    mesh_tasks: HashMap<ChunkPos, (u64, Task<ChunkMeshes>)>,
    needs_mesh: HashSet<ChunkPos>,
    entities: HashMap<ChunkPos, Vec<Entity>>,
    /// Versión del último mallado pedido por chunk (descarta resultados viejos).
    version: HashMap<ChunkPos, u64>,
    pub meshed: usize,
}

impl Streaming {
    pub fn is_meshed(&self, p: ChunkPos) -> bool {
        self.entities.contains_key(&p)
    }
}

const MAX_TASKS: usize = 8;

/// Cambia un bloque: actualiza la luz y pide el remallado del chunk y de los vecinos tocados.
pub fn set_block(
    world: &mut World,
    pos: BlockPos,
    id: BlockId,
    remesh: &mut MessageWriter<RemeshRequest>,
) -> bool {
    if !pos.in_height() {
        return false;
    }
    let Some(col) = world.columns.get_mut(&pos.chunk()) else {
        return false;
    };
    col.set(pos.local(), id);
    relight_around(world, pos);
    for c in remesh_targets(pos) {
        remesh.write(RemeshRequest(c));
    }
    true
}

/// Chunks a remallar tras editar `pos`: el suyo, los vecinos de borde y, por la luz, todos los vecinos.
pub fn remesh_targets(pos: BlockPos) -> Vec<ChunkPos> {
    let c = pos.chunk();
    let l = pos.local();
    let mut v = vec![c];
    if l.x == 0 {
        v.push(ChunkPos::new(c.x - 1, c.z));
    }
    if l.x as i32 == CHUNK - 1 {
        v.push(ChunkPos::new(c.x + 1, c.z));
    }
    if l.z == 0 {
        v.push(ChunkPos::new(c.x, c.z - 1));
    }
    if l.z as i32 == CHUNK - 1 {
        v.push(ChunkPos::new(c.x, c.z + 1));
    }
    // La luz puede cambiar hasta 15 bloques alrededor.
    for n in c.neighbors() {
        if !v.contains(&n) {
            v.push(n);
        }
    }
    v
}

pub fn chunk_transform(p: ChunkPos, origin: ChunkPos) -> Transform {
    Transform::from_translation(to_render(p.origin(), origin).with_y(0.0))
}

#[allow(clippy::too_many_arguments)]
pub fn stream_chunks(
    mut commands: Commands,
    mut world: ResMut<World>,
    mut st: ResMut<Streaming>,
    vd: Res<ViewDistance>,
    focus: Res<Focus>,
    origin: Res<FloatingOrigin>,
    mut remesh: MessageReader<RemeshRequest>,
    mut meshes: ResMut<Assets<Mesh>>,
    mats: Res<BlockMaterials>,
) {
    let center = BlockPos::new(focus.0.x.floor() as i32, 0, focus.0.z.floor() as i32).chunk();
    let vd = vd.0 as i32;
    let pool = AsyncComputeTaskPool::get();

    // 1. Generar o cargar lo que falta (radio vd + 1 para tener vecinos al mallar).
    for p in spiral(center, vd + 1) {
        if st.gen_tasks.len() >= MAX_TASKS {
            break;
        }
        if world.columns.contains_key(&p) || st.gen_tasks.contains_key(&p) {
            continue;
        }
        if let Some(mut col) = world.store.load(p) {
            light_column(&mut col);
            world.columns.insert(p, col);
            mark_around(&mut st, p);
            continue;
        }
        let g = world.gen_.clone();
        st.gen_tasks.insert(
            p,
            pool.spawn(async move {
                let mut c = g.generate(p);
                light_column(&mut c);
                c
            }),
        );
    }
    let done: Vec<ChunkPos> = st
        .gen_tasks
        .iter_mut()
        .filter_map(|(p, t)| check_ready(t).map(|c| (*p, c)))
        .map(|(p, c)| {
            world.columns.insert(p, c);
            p
        })
        .collect();
    for p in done {
        st.gen_tasks.remove(&p);
        mark_around(&mut st, p);
    }

    // 2. Remallados pedidos (ediciones).
    for RemeshRequest(p) in remesh.read() {
        if world.columns.contains_key(p) {
            st.needs_mesh.insert(*p);
        }
    }

    // 3. Mallar lo que tiene sus 4 vecinos, dentro de la distancia de visión.
    let mut want: Vec<ChunkPos> = st
        .needs_mesh
        .iter()
        .copied()
        .filter(|p| (p.x - center.x).abs() <= vd && (p.z - center.z).abs() <= vd)
        .filter(|p| p.neighbors().iter().all(|n| world.columns.contains_key(n)))
        .collect();
    want.sort_by_key(|p| center.dist2(*p));
    for p in want {
        if st.mesh_tasks.len() >= MAX_TASKS {
            break;
        }
        st.needs_mesh.remove(&p);
        let col = world.columns[&p].clone();
        let nb: Vec<ChunkColumn> = p
            .neighbors()
            .iter()
            .map(|n| world.columns[n].clone())
            .collect();
        let ver = st.version.entry(p).or_insert(0);
        *ver += 1;
        let v = *ver;
        let task = pool.spawn(async move {
            let n: [Option<&ChunkColumn>; 4] =
                [Some(&nb[0]), Some(&nb[1]), Some(&nb[2]), Some(&nb[3])];
            mesh_column(&col, n)
        });
        st.mesh_tasks.insert(p, (v, task));
    }
    let ready: Vec<(ChunkPos, u64, ChunkMeshes)> = st
        .mesh_tasks
        .iter_mut()
        .filter_map(|(p, (v, t))| check_ready(t).map(|m| (*p, *v, m)))
        .collect();
    for (p, v, m) in ready {
        st.mesh_tasks.remove(&p);
        if st.version.get(&p) != Some(&v) || !world.columns.contains_key(&p) {
            continue;
        }
        if let Some(old) = st.entities.remove(&p) {
            for e in old {
                commands.entity(e).despawn();
            }
        }
        let mut ents = Vec::new();
        let tf = chunk_transform(p, origin.0);
        for (data, mat) in [
            (&m.opaque, &mats.opaque),
            (&m.cutout, &mats.cutout),
            (&m.water, &mats.water),
        ] {
            if data.is_empty() {
                continue;
            }
            let e = commands
                .spawn((
                    Mesh3d(meshes.add(build_mesh(data))),
                    MeshMaterial3d(mat.clone()),
                    tf,
                    ChunkMesh(p),
                ))
                .id();
            ents.push(e);
        }
        st.entities.insert(p, ents);
        st.meshed += 1;
    }

    // 4. Descargar (y guardar) lo lejano.
    let gone = to_unload(world.columns.keys().copied(), center, vd + 3);
    for p in gone {
        if let Some(c) = world.columns.remove(&p)
            && c.dirty
        {
            let _ = world.store.save(&c);
        }
        st.needs_mesh.remove(&p);
        if let Some(es) = st.entities.remove(&p) {
            for e in es {
                commands.entity(e).despawn();
            }
        }
    }
}

fn mark_around(st: &mut Streaming, p: ChunkPos) {
    st.needs_mesh.insert(p);
    for n in p.neighbors() {
        if st.entities.contains_key(&n) || st.mesh_tasks.contains_key(&n) {
            st.needs_mesh.insert(n);
        }
    }
}

/// Mueve el origen flotante cuando el jugador se aleja más de 2 chunks.
pub fn rebase_origin(
    focus: Res<Focus>,
    mut origin: ResMut<FloatingOrigin>,
    mut q: Query<(&ChunkMesh, &mut Transform)>,
) {
    let c = BlockPos::new(focus.0.x.floor() as i32, 0, focus.0.z.floor() as i32).chunk();
    if (c.x - origin.0.x).abs() > 2 || (c.z - origin.0.z).abs() > 2 {
        origin.0 = c;
        for (cm, mut t) in &mut q {
            *t = chunk_transform(cm.0, c);
        }
    }
}

/// ¿Hay un bloque sólido? (los chunks sin cargar cuentan como sólidos)
pub fn solid_at(world: &World, p: BlockPos) -> bool {
    match world.block_at(p) {
        Some(b) => b.def().solid,
        None => p.in_height(),
    }
}

pub fn is_air(world: &World, p: BlockPos) -> bool {
    world.block_at(p) == Some(AIR)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spiral_order_nearest_first() {
        let c = ChunkPos::new(10, -4);
        let s = spiral(c, 3);
        assert_eq!(s[0], c);
        assert_eq!(s.len(), 49);
        let set: std::collections::HashSet<_> = s.iter().collect();
        assert_eq!(set.len(), 49);
        for w in s.windows(2) {
            assert!(c.dist2(w[0]) <= c.dist2(w[1]));
        }
    }

    #[test]
    fn unload_set_beyond_margin() {
        let loaded = [
            ChunkPos::new(0, 0),
            ChunkPos::new(4, -4),
            ChunkPos::new(5, 0),
            ChunkPos::new(-7, 7),
        ];
        let mut out = to_unload(loaded.iter().copied(), ChunkPos::new(0, 0), 4);
        out.sort();
        assert_eq!(out, vec![ChunkPos::new(-7, 7), ChunkPos::new(5, 0)]);
    }

    #[test]
    fn floating_origin_rebase() {
        let origin = ChunkPos::new(1_874_937, 0);
        let v = to_render(BlockPos::new(29_999_000, 70, 5), origin);
        assert!(v.x.abs() < 64.0, "{v:?}");
        assert_eq!(v.y, 70.0);
    }
}
