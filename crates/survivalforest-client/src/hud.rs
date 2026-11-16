//! HUD: punto de mira, barra de 9 casillas, corazones, jamones, burbujas, experiencia,
//! destello de daño y panel F3.

use crate::player::{Mode, Player, WorldPos};
use crate::sky::GameTime;
use crate::streaming::{Streaming, World};
use crate::textures::{T, paint_tiles};
use bevy::asset::RenderAssetUsages;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use survivalforest_core::item::{ItemId, items};
use survivalforest_core::{BlockPos, GameMode};

/// Iconos del HUD por nombre (ver `icons::HUD_ICONS`).
#[derive(Resource)]
pub struct HudIcons(pub std::collections::HashMap<&'static str, Handle<Image>>);

#[derive(Component)]
pub struct SlotIcon(pub usize);
#[derive(Component)]
pub struct SlotCount(pub usize);
#[derive(Component)]
pub struct SlotWear(pub usize);
#[derive(Component)]
pub struct Heart(pub usize);
#[derive(Component)]
pub struct Ham(pub usize);
#[derive(Component)]
pub struct Bubble(pub usize);
#[derive(Component)]
pub struct XpFill;
#[derive(Component)]
pub struct XpLevel;
#[derive(Component)]
pub struct HurtFlash;
/// Filas que solo se ven en supervivencia.
#[derive(Component)]
pub struct SurvivalOnly;

/// Icono de cada objeto: los bloques usan su textura de lado; el resto, su icono pintado.
#[derive(Resource)]
pub struct ItemIcons(pub std::collections::HashMap<ItemId, Handle<Image>>);

#[derive(Component)]
pub struct HudRoot;

#[derive(Component)]
pub struct Slot(pub usize);

#[derive(Component)]
pub struct DebugText;

#[derive(Resource, Default)]
pub struct ShowDebug(pub bool);

fn icon_image(t: Vec<u8>) -> Image {
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
    img
}

pub fn setup_icons(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let icons: Vec<Handle<Image>> = paint_tiles()
        .into_iter()
        .map(|t| images.add(icon_image(t)))
        .collect();
    let mut by_item = std::collections::HashMap::new();
    for it in items() {
        let h = match it.places {
            Some(b) => icons[b.def().tex[1] as usize].clone(),
            None => match crate::icons::item_icon(&it.key) {
                Some(t) => images.add(icon_image(t)),
                None => continue,
            },
        };
        by_item.insert(it.id, h);
    }
    commands.insert_resource(ItemIcons(by_item));
    let hud = crate::icons::HUD_ICONS
        .iter()
        .map(|n| (*n, images.add(icon_image(crate::icons::hud_icon(n)))))
        .collect();
    commands.insert_resource(HudIcons(hud));
}

pub fn spawn_hud(mut commands: Commands, hud: Res<HudIcons>, mode: Res<Mode>) {
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
            // Destello rojo al recibir daño.
            r.spawn((
                HurtFlash,
                Node {
                    position_type: PositionType::Absolute,
                    width: percent(100),
                    height: percent(100),
                    ..default()
                },
                BackgroundColor(Color::NONE),
            ));
            // Abajo en el centro: corazones y jamones, experiencia y barra de 9 casillas.
            r.spawn(Node {
                position_type: PositionType::Absolute,
                bottom: px(14),
                left: percent(50),
                margin: UiRect::left(px(-245)),
                width: px(490),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: px(4),
                ..default()
            })
            .with_children(|col| {
                let survival = mode.0 == GameMode::Survival;
                let vis = if survival {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };
                // Burbujas de aire (encima de los jamones).
                col.spawn((
                    SurvivalOnly,
                    vis,
                    Node {
                        width: percent(100),
                        justify_content: JustifyContent::FlexEnd,
                        column_gap: px(1),
                        height: px(18),
                        ..default()
                    },
                ))
                .with_children(|row| {
                    for i in 0..10 {
                        row.spawn((
                            Bubble(i),
                            ImageNode::new(hud.0["bubble"].clone()),
                            Node {
                                width: px(18),
                                height: px(18),
                                ..default()
                            },
                        ));
                    }
                });
                col.spawn((
                    SurvivalOnly,
                    vis,
                    Node {
                        width: percent(100),
                        justify_content: JustifyContent::SpaceBetween,
                        ..default()
                    },
                ))
                .with_children(|row| {
                    row.spawn(Node {
                        column_gap: px(1),
                        ..default()
                    })
                    .with_children(|h| {
                        for i in 0..10 {
                            h.spawn((
                                Heart(i),
                                ImageNode::new(hud.0["heart_full"].clone()),
                                Node {
                                    width: px(20),
                                    height: px(20),
                                    ..default()
                                },
                            ));
                        }
                    });
                    row.spawn(Node {
                        column_gap: px(1),
                        flex_direction: FlexDirection::RowReverse,
                        ..default()
                    })
                    .with_children(|h| {
                        for i in 0..10 {
                            h.spawn((
                                Ham(i),
                                ImageNode::new(hud.0["ham_full"].clone()),
                                Node {
                                    width: px(20),
                                    height: px(20),
                                    ..default()
                                },
                            ));
                        }
                    });
                });
                // Barra de experiencia: café y verde cafetal, con el nivel en un grano de café.
                col.spawn((
                    SurvivalOnly,
                    vis,
                    Node {
                        width: percent(100),
                        height: px(22),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                ))
                .with_children(|xp| {
                    xp.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: px(0),
                            right: px(0),
                            height: px(8),
                            border: UiRect::all(px(1)),
                            border_radius: BorderRadius::all(px(4)),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.24, 0.16, 0.1)),
                        BorderColor::all(Color::srgb(0.12, 0.08, 0.05)),
                    ))
                    .with_child((
                        XpFill,
                        Node {
                            width: percent(0),
                            height: percent(100),
                            border_radius: BorderRadius::all(px(3)),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.36, 0.66, 0.27)),
                    ));
                    xp.spawn((
                        ImageNode::new(hud.0["bean"].clone()),
                        Node {
                            width: px(26),
                            height: px(26),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                    ))
                    .with_child((
                        XpLevel,
                        Text::new("0"),
                        TextFont {
                            font_size: FontSize::Px(13.0),
                            ..default()
                        },
                        TextColor(Color::srgb(1.0, 0.94, 0.8)),
                    ));
                });
                // Barra de 9 casillas.
                col.spawn((
                    Node {
                        padding: UiRect::axes(px(8), px(6)),
                        column_gap: px(6),
                        border_radius: BorderRadius::all(px(12)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.12, 0.08, 0.05, 0.72)),
                ))
                .with_children(|bar| {
                    for i in 0..9 {
                        bar.spawn((
                            Slot(i),
                            Node {
                                width: px(46),
                                height: px(46),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                border: UiRect::all(px(3)),
                                border_radius: BorderRadius::all(px(8)),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.45, 0.3, 0.2, 0.45)),
                            BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.18)),
                        ))
                        .with_children(|s| {
                            s.spawn((
                                SlotIcon(i),
                                ImageNode::default(),
                                Visibility::Hidden,
                                Node {
                                    width: px(32),
                                    height: px(32),
                                    ..default()
                                },
                            ));
                            s.spawn((
                                SlotCount(i),
                                Text::new(""),
                                TextFont {
                                    font_size: FontSize::Px(13.0),
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                                Node {
                                    position_type: PositionType::Absolute,
                                    right: px(2),
                                    bottom: px(0),
                                    ..default()
                                },
                            ));
                            s.spawn((
                                SlotWear(i),
                                Visibility::Hidden,
                                Node {
                                    position_type: PositionType::Absolute,
                                    left: px(5),
                                    bottom: px(3),
                                    width: px(30),
                                    height: px(3),
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(0.4, 0.85, 0.3)),
                            ));
                        });
                    }
                });
            });
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

/// Actualiza la barra, los corazones, los jamones, el aire, la experiencia y el destello.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn update_hud(
    st: Res<crate::survival::PlayerState>,
    items: Res<ItemIcons>,
    hud: Res<HudIcons>,
    mut slots: Query<(&Slot, &mut BorderColor)>,
    mut icons: Query<
        (&SlotIcon, &mut ImageNode, &mut Visibility),
        (Without<SlotWear>, Without<Bubble>),
    >,
    mut counts: Query<(&SlotCount, &mut Text), Without<XpLevel>>,
    mut wear: Query<
        (&SlotWear, &mut Node, &mut BackgroundColor, &mut Visibility),
        (Without<SlotIcon>, Without<Bubble>, Without<HurtFlash>),
    >,
    mut hearts: Query<(&Heart, &mut ImageNode), (Without<SlotIcon>, Without<Ham>, Without<Bubble>)>,
    mut hams: Query<(&Ham, &mut ImageNode), (Without<SlotIcon>, Without<Heart>, Without<Bubble>)>,
    mut bubbles: Query<(&Bubble, &mut Visibility), (Without<SlotIcon>, Without<SlotWear>)>,
    mut xp_fill: Query<&mut Node, (With<XpFill>, Without<SlotWear>)>,
    mut xp_level: Query<&mut Text, (With<XpLevel>, Without<SlotCount>)>,
    mut flash: Query<&mut BackgroundColor, (With<HurtFlash>, Without<SlotWear>)>,
) {
    for (s, mut b) in &mut slots {
        *b = BorderColor::all(if s.0 == st.inv.selected as usize {
            Color::srgb(1.0, 0.85, 0.6)
        } else {
            Color::srgba(1.0, 1.0, 1.0, 0.18)
        });
    }
    for (s, mut img, mut vis) in &mut icons {
        match st.inv.slots.0[s.0]
            .as_ref()
            .and_then(|it| items.0.get(&it.item))
        {
            Some(h) => {
                img.image = h.clone();
                *vis = Visibility::Inherited;
            }
            None => *vis = Visibility::Hidden,
        }
    }
    for (s, mut t) in &mut counts {
        t.0 = st.inv.slots.0[s.0]
            .as_ref()
            .filter(|it| it.count > 1)
            .map(|it| it.count.to_string())
            .unwrap_or_default();
    }
    for (s, mut n, mut bg, mut vis) in &mut wear {
        let tool = st.inv.slots.0[s.0].as_ref().and_then(|it| {
            survivalforest_core::item::item(it.item)
                .tool
                .as_ref()
                .map(|t| (it.wear, t.durability))
        });
        match tool {
            Some((w, d)) if w > 0 => {
                let left = 1.0 - w as f32 / d as f32;
                n.width = px(30.0 * left);
                bg.0 = Color::srgb(1.0 - left, 0.3 + left * 0.55, 0.25);
                *vis = Visibility::Inherited;
            }
            _ => *vis = Visibility::Hidden,
        }
    }
    let pick = |v: f32, i: usize, full: &'static str, half: &'static str, empty: &'static str| {
        let pts = v - i as f32 * 2.0;
        hud.0[if pts >= 2.0 {
            full
        } else if pts >= 1.0 {
            half
        } else {
            empty
        }]
        .clone()
    };
    for (h, mut img) in &mut hearts {
        img.image = pick(
            st.vitals.health.ceil(),
            h.0,
            "heart_full",
            "heart_half",
            "heart_empty",
        );
    }
    for (h, mut img) in &mut hams {
        img.image = pick(
            st.vitals.hunger.ceil(),
            h.0,
            "ham_full",
            "ham_half",
            "ham_empty",
        );
    }
    let air = st.vitals.air_bubbles() as usize;
    for (b, mut vis) in &mut bubbles {
        *vis = if air < 10 && b.0 < air {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    for mut n in &mut xp_fill {
        n.width = percent(st.xp.progress() * 100.0);
    }
    for mut t in &mut xp_level {
        t.0 = st.xp.level.to_string();
    }
    for mut bg in &mut flash {
        bg.0 = Color::srgba(0.8, 0.05, 0.02, st.hurt_flash.clamp(0.0, 0.3));
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
        "SurvivalForest - etapa 2\nFPS: {fps:.0}\nXYZ: {:.2} / {:.2} / {:.2}\nChunk: {} {}\nBioma: {}\nChunks cargados: {} - mallados: {}\nHora: {}{}",
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
