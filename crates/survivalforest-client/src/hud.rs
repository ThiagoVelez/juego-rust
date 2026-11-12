//! HUD: punto de mira, barra de bloques (creativo) y panel F3.

use crate::interact::{BUILD_BLOCKS, Hotbar};
use crate::player::{Mode, Player, WorldPos};
use crate::sky::GameTime;
use crate::streaming::{Streaming, World};
use crate::textures::{T, paint_tiles};
use bevy::asset::RenderAssetUsages;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use survivalforest_core::{BlockPos, GameMode};

#[derive(Resource)]
pub struct BlockIcons(pub Vec<Handle<Image>>);

#[derive(Component)]
pub struct HudRoot;

#[derive(Component)]
pub struct Slot(pub usize);

#[derive(Component)]
pub struct DebugText;

#[derive(Resource, Default)]
pub struct ShowDebug(pub bool);

pub fn setup_icons(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let icons = paint_tiles()
        .into_iter()
        .map(|t| {
            let mut img = Image::new(
                Extent3d {
                    width: T as u32,
                    height: T as u32,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                t,
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::RENDER_WORLD,
            );
            img.sampler = ImageSampler::nearest();
            images.add(img)
        })
        .collect();
    commands.insert_resource(BlockIcons(icons));
}

pub fn spawn_hud(mut commands: Commands, icons: Res<BlockIcons>, mode: Res<Mode>) {
    commands
        .spawn((
            HudRoot,
            Node {
                width: percent(100),
                height: percent(100),
                position_type: PositionType::Absolute,
                ..default()
            },
        ))
        .with_children(|r| {
            // Punto de mira
            r.spawn(Node {
                position_type: PositionType::Absolute,
                left: percent(50),
                top: percent(50),
                margin: UiRect::new(px(-9), px(0), px(-9), px(0)),
                width: px(18),
                height: px(18),
                ..default()
            })
            .with_children(|c| {
                c.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(8),
                        width: px(2),
                        height: px(18),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.9)),
                ));
                c.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        top: px(8),
                        width: px(18),
                        height: px(2),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.9)),
                ));
            });
            // Barra de bloques (solo creativo en la etapa 1)
            if mode.0 == GameMode::Creative {
                r.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        bottom: px(18),
                        left: percent(50),
                        margin: UiRect::left(px(-(BUILD_BLOCKS.len() as f32 * 54.0 + 20.0) / 2.0)),
                        padding: UiRect::axes(px(10), px(8)),
                        column_gap: px(6),
                        border_radius: BorderRadius::all(px(26)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.08, 0.06, 0.05, 0.6)),
                ))
                .with_children(|bar| {
                    for (i, b) in BUILD_BLOCKS.iter().enumerate() {
                        let tex = b.def().tex[1] as usize;
                        bar.spawn((
                            Slot(i),
                            Node {
                                width: px(48),
                                height: px(48),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                border: UiRect::all(px(3)),
                                border_radius: BorderRadius::MAX,
                                ..default()
                            },
                            BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.18)),
                        ))
                        .with_child((
                            ImageNode::new(icons.0[tex].clone()),
                            Node {
                                width: px(28),
                                height: px(28),
                                border_radius: BorderRadius::all(px(6)),
                                ..default()
                            },
                        ));
                    }
                });
            }
            // Panel F3
            r.spawn((
                DebugText,
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(Color::WHITE),
                Node {
                    position_type: PositionType::Absolute,
                    left: px(10),
                    top: px(10),
                    padding: UiRect::all(px(8)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
            ));
        });
}

pub fn despawn_hud(mut commands: Commands, q: Query<Entity, With<HudRoot>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}

pub fn update_slots(bar: Res<Hotbar>, mut q: Query<(&Slot, &mut BorderColor)>) {
    for (s, mut b) in &mut q {
        *b = BorderColor::all(if s.0 == bar.selected {
            Color::srgb(1.0, 0.85, 0.6)
        } else {
            Color::srgba(1.0, 1.0, 1.0, 0.18)
        });
    }
}

/// Hora del juego como reloj: el tick 0 es el amanecer (06:00).
pub fn clock(ticks: u64) -> String {
    let minutes = ((ticks % 24_000) as f64 / 24_000.0 * 1440.0) as u64 + 6 * 60;
    format!("{:02}:{:02}", (minutes / 60) % 24, minutes % 60)
}

pub fn update_debug(
    keys: Res<ButtonInput<KeyCode>>,
    mut show: ResMut<ShowDebug>,
    diag: Res<DiagnosticsStore>,
    world: Res<World>,
    st: Res<Streaming>,
    gt: Res<GameTime>,
    player: Single<&WorldPos, With<Player>>,
    mut q: Single<(&mut Text, &mut BackgroundColor), With<DebugText>>,
) {
    if keys.just_released(KeyCode::F3) && !keys.pressed(KeyCode::KeyT) {
        show.0 = !show.0;
    }
    let (ref mut text, ref mut bg) = *q;
    if !show.0 {
        text.0.clear();
        bg.0 = Color::NONE;
        return;
    }
    let p = player.pos;
    let bp = BlockPos::new(p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32);
    let c = bp.chunk();
    let fps = diag
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0);
    text.0 = format!(
        "SurvivalForest - etapa 1\nFPS: {fps:.0}\nXYZ: {:.2} / {:.2} / {:.2}\nChunk: {} {}\nBioma: {}\nChunks cargados: {} - mallados: {}\nHora: {}{}",
        p.x,
        p.y,
        p.z,
        c.x,
        c.z,
        world.gen_.biome_at(bp.x, bp.z).name_es(),
        world.columns.len(),
        st.meshed,
        clock(gt.time.ticks),
        if gt.fast { " (×100)" } else { "" }
    );
    bg.0 = Color::srgba(0.0, 0.0, 0.0, 0.45);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_starts_at_dawn() {
        assert_eq!(clock(0), "06:00");
        assert_eq!(clock(6000), "12:00");
        assert_eq!(clock(18000), "00:00");
    }
}
