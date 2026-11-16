//! Dibuja los objetos tirados en el suelo (cubos pequeños con su icono que giran) y los orbes
//! de experiencia (esferas verde café brillantes).

use crate::hud::ItemIcons;
use crate::streaming::FloatingOrigin;
use crate::survival::WorldExtras;
use bevy::prelude::*;
use std::collections::HashMap;
use survivalforest_core::item::ItemId;

#[derive(Component)]
pub struct DropVisual(pub Option<ItemId>);

#[derive(Resource, Default)]
pub struct DropAssets {
    cube: Option<Handle<Mesh>>,
    orb: Option<Handle<Mesh>>,
    orb_mat: Option<Handle<StandardMaterial>>,
    mats: HashMap<ItemId, Handle<StandardMaterial>>,
}

#[allow(clippy::too_many_arguments)]
pub fn sync_drops(
    mut commands: Commands,
    time: Res<Time>,
    ex: Res<WorldExtras>,
    origin: Res<FloatingOrigin>,
    icons: Res<ItemIcons>,
    mut assets: Local<DropAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut q: Query<(
        Entity,
        &mut DropVisual,
        &mut Transform,
        &mut Mesh3d,
        &mut MeshMaterial3d<StandardMaterial>,
    )>,
) {
    let cube = assets
        .cube
        .get_or_insert_with(|| meshes.add(Cuboid::new(0.28, 0.28, 0.28)))
        .clone();
    let orb = assets
        .orb
        .get_or_insert_with(|| meshes.add(Sphere::new(0.16)))
        .clone();
    let orb_mat = assets
        .orb_mat
        .get_or_insert_with(|| {
            materials.add(StandardMaterial {
                base_color: Color::srgb(0.45, 0.75, 0.3),
                emissive: LinearRgba::rgb(0.6, 1.2, 0.3),
                unlit: true,
                ..default()
            })
        })
        .clone();
    let mut ents: Vec<_> = q.iter_mut().collect();
    let items = &ex.items.0;
    // Sobran entidades: se quitan las del final.
    while ents.len() > items.len() {
        let (e, ..) = ents.pop().expect("entidad");
        commands.entity(e).despawn();
    }
    let o = origin.0.origin();
    let t = time.elapsed_secs();
    let place = |i: usize, it: &survivalforest_sim::drops::GroundItem| {
        let bob = (t * 2.0 + i as f32).sin() * 0.06 + 0.2;
        Transform::from_xyz(
            (it.pos.x - o.x as f64) as f32,
            it.pos.y as f32 + bob,
            (it.pos.z - o.z as f64) as f32,
        )
        .with_rotation(Quat::from_rotation_y(t * 1.5 + i as f32))
    };
    for (i, it) in items.iter().enumerate() {
        let kind = it.stack.as_ref().map(|s| s.item);
        let mat = match kind {
            Some(id) => assets
                .mats
                .entry(id)
                .or_insert_with(|| {
                    let tex = icons.0.get(&id).cloned();
                    materials.add(StandardMaterial {
                        base_color_texture: tex,
                        alpha_mode: AlphaMode::Mask(0.5),
                        unlit: true,
                        ..default()
                    })
                })
                .clone(),
            None => orb_mat.clone(),
        };
        let mesh = if kind.is_some() {
            cube.clone()
        } else {
            orb.clone()
        };
        if let Some((_, vis, tf, m, mm)) = ents.get_mut(i) {
            *tf.as_mut() = place(i, it);
            if vis.0 != kind {
                vis.0 = kind;
                m.0 = mesh;
                mm.0 = mat;
            }
        } else {
            commands.spawn((
                DropVisual(kind),
                place(i, it),
                Mesh3d(mesh),
                MeshMaterial3d(mat),
            ));
        }
    }
}

/// Al salir del mundo se quitan todos.
pub fn despawn_drops(mut commands: Commands, q: Query<Entity, With<DropVisual>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}
