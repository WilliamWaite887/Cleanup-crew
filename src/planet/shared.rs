use bevy::prelude::*;
use super::{
    BackgroundSprite, BackgroundRes, PlanetWinScreen,
    BossHealthBarRoot, BossHealthBarFill, FinalBoss,
    BossArenaState, BossExitDoor, PlanetExitBeacon,
    TerminalSession, CodeEntryState, DialInteractState, DialTargets,
    MiniBossArenaState,
};
use crate::{
    EndScreenButtons, GameEntity, GameState, MainCamera, PlanetCount,
    PlanetLevelMarker, StationLevel, TestPlanetMode,
    FONT_PATH, WIN_H, WIN_W, Z_FLOOR,
};
use crate::player::{Player, aabb_overlap};
use crate::rewards::RewardRes;
use crate::settings::KeyBindings;

// ── Background tints & images ─────────────────────────────────────────────────

pub(super) fn load_background_assets(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(BackgroundRes {
        stars: assets.load("stars_background.png"),
        planet_station: assets.load("planet_background.png"),
    });
}

fn spawn_background(commands: &mut Commands, image: Handle<Image>, size: Vec2) {
    commands.spawn((
        Sprite {
            image,
            custom_size: Some(size),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, Z_FLOOR - 10.0),
        BackgroundSprite,
        GameEntity,
    ));
}

pub(super) fn spawn_stars_background(
    mut commands: Commands,
    bg: Res<BackgroundRes>,
    window_q: Query<&Window>,
) {
    let size = window_q.single()
        .map(|w| Vec2::new(w.width(), w.height()))
        .unwrap_or(Vec2::new(WIN_W, WIN_H));
    spawn_background(&mut commands, bg.stars.clone(), size);
}

pub(super) fn spawn_planet_station_background(
    mut commands: Commands,
    bg: Res<BackgroundRes>,
    window_q: Query<&Window>,
) {
    let size = window_q.single()
        .map(|w| Vec2::new(w.width(), w.height()))
        .unwrap_or(Vec2::new(WIN_W, WIN_H));
    spawn_background(&mut commands, bg.planet_station.clone(), size);
}

pub(super) fn update_background_position(
    camera_q: Query<&Transform, With<MainCamera>>,
    mut bg_q: Query<&mut Transform, (With<BackgroundSprite>, Without<MainCamera>)>,
) {
    let Ok(cam_tf) = camera_q.single() else { return };
    for mut bg_tf in &mut bg_q {
        bg_tf.translation.x = cam_tf.translation.x;
        bg_tf.translation.y = cam_tf.translation.y;
    }
}

pub(super) fn tint_station_background(mut clear_color: ResMut<ClearColor>) {
    clear_color.0 = Color::srgb(0.06, 0.02, 0.10);
}

pub(super) fn tint_planet_background(mut clear_color: ResMut<ClearColor>) {
    clear_color.0 = Color::srgb(0.03, 0.10, 0.04);
}

pub(super) fn restore_background(
    mut commands: Commands,
    mut clear_color: ResMut<ClearColor>,
    bar_q: Query<Entity, With<BossHealthBarRoot>>,
    bg_q: Query<Entity, With<BackgroundSprite>>,
) {
    clear_color.0 = Color::srgb(0.02, 0.02, 0.06);
    commands.remove_resource::<PlanetLevelMarker>();
    commands.remove_resource::<TestPlanetMode>();
    commands.remove_resource::<BossArenaState>();
    commands.remove_resource::<DialTargets>();
    commands.remove_resource::<DialInteractState>();
    commands.remove_resource::<MiniBossArenaState>();
    for e in &bar_q {
        commands.entity(e).despawn();
    }
    for e in &bg_q {
        commands.entity(e).despawn();
    }
}

// ── Boss health bar ───────────────────────────────────────────────────────────

pub(super) fn do_spawn_boss_health_bar(commands: &mut Commands, asset_server: &AssetServer) {
    let font: Handle<Font> = asset_server.load(FONT_PATH);

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(16.0),
                left: Val::Percent(10.0),
                width: Val::Percent(80.0),
                height: Val::Px(40.0),
                align_items: AlignItems::Center,
                column_gap: Val::Px(12.0),
                padding: UiRect::horizontal(Val::Px(12.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.75)),
            BorderRadius::all(Val::Px(6.0)),
            ZIndex(50),
            BossHealthBarRoot,
        ))
        .with_children(|root| {
            root.spawn((
                Text::new("BOSS"),
                TextFont { font, font_size: 18.0, ..default() },
                TextColor(Color::srgb(1.0, 0.3, 0.3)),
                Node { width: Val::Px(52.0), ..default() },
            ));

            root.spawn((
                Node {
                    flex_grow: 1.0,
                    height: Val::Px(24.0),
                    overflow: Overflow::clip(),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.25, 0.04, 0.04, 1.0)),
                BorderRadius::all(Val::Px(4.0)),
            ))
            .with_children(|bg| {
                bg.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.85, 0.12, 0.12)),
                    BorderRadius::all(Val::Px(4.0)),
                    BossHealthBarFill,
                ));
            });
        });
}

pub(super) fn update_boss_health_bar(
    boss_q: Query<(&crate::enemies::Health, &crate::enemies::MaxHealth), With<FinalBoss>>,
    mut fill_q: Query<&mut Node, With<BossHealthBarFill>>,
) {
    let Ok(mut fill_node) = fill_q.single_mut() else { return };
    let pct = boss_q
        .single()
        .map(|(hp, max)| (hp.0 / max.0).clamp(0.0, 1.0) * 100.0)
        .unwrap_or(0.0);
    fill_node.width = Val::Percent(pct);
}

// ── Boss death — spawn chest + open exit corridor ─────────────────────────────

pub(super) fn spawn_boss_chest(
    mut commands: Commands,
    boss_q: Query<(), With<FinalBoss>>,
    arena_state: Res<BossArenaState>,
    mut key_state: ResMut<crate::key_chest::LevelKeyState>,
    key_chest_res: Res<crate::key_chest::KeyChestRes>,
    exit_door_q: Query<Entity, With<BossExitDoor>>,
    asset_server: Res<AssetServer>,
) {
    if *arena_state != BossArenaState::Active { return; }
    if !boss_q.is_empty() { return; }
    if key_state.boss_chest_spawned { return; }

    commands.spawn((
        Sprite::from_image(key_chest_res.chest_img.clone()),
        Transform::from_translation(super::planet1::BOSS_CHEST_POS),
        crate::key_chest::Chest,
        crate::collidable::Collidable,
        crate::collidable::Collider { half_extents: Vec2::splat(crate::TILE_SIZE * 0.5) },
        crate::GameEntity,
    ));

    for entity in &exit_door_q {
        commands.entity(entity).despawn();
    }

    let font: Handle<Font> = asset_server.load(FONT_PATH);
    commands.spawn((
        Text2d::new("[ E ]  Leave Planet"),
        TextFont { font, font_size: 24.0, ..default() },
        TextColor(Color::srgb(0.3, 1.0, 0.4)),
        Transform::from_translation(super::planet1::PLANET_EXIT_BEACON_POS),
        PlanetExitBeacon,
        crate::GameEntity,
    ));

    key_state.boss_chest_spawned = true;
}

// ── Planet exit — player presses E near the beacon ───────────────────────────

pub(super) fn interact_with_exit_beacon(
    input: Res<ButtonInput<KeyCode>>,
    player_q: Query<&Transform, With<Player>>,
    beacon_q: Query<&Transform, With<PlanetExitBeacon>>,
    boss_arena_state: Res<BossArenaState>,
    session: Option<Res<TerminalSession>>,
    code_session: Option<Res<CodeEntryState>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut planet_count: ResMut<PlanetCount>,
    bindings: Res<KeyBindings>,
) {
    if *boss_arena_state != BossArenaState::Active { return; }
    if session.is_some() || code_session.is_some() { return; }
    if !input.just_pressed(bindings.interact) { return; }

    let Ok(player_tf) = player_q.single() else { return };
    let Ok(beacon_tf) = beacon_q.single() else { return };
    let pp = player_tf.translation;
    let bp = beacon_tf.translation;

    if aabb_overlap(pp.x, pp.y, Vec2::splat(crate::TILE_SIZE * 2.0), bp.x, bp.y, Vec2::splat(crate::TILE_SIZE * 1.5)) {
        planet_count.0 += 1;
        next_state.set(GameState::PlanetWin);
    }
}

// ── Vault rewards ─────────────────────────────────────────────────────────────

pub(super) fn spawn_vault_rewards(
    mut commands: Commands,
    reward_res: Res<RewardRes>,
    planet_count: Res<PlanetCount>,
) {
    for &pos in super::planet_vault_rewards(planet_count.0 as usize) {
        crate::rewards::spawn_reward(&mut commands, pos, &reward_res);
    }
}

// ── Planet win screen ─────────────────────────────────────────────────────────

pub(super) fn setup_planet_win_screen(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    station_level: Res<StationLevel>,
    planet_count: Res<PlanetCount>,
) {
    let font: Handle<Font> = asset_server.load(FONT_PATH);

    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(24.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.05, 0.0, 0.92)),
        ZIndex(20),
        PlanetWinScreen,
    ))
    .with_children(|root| {
        root.spawn((
            Text::new("Planet Cleared!"),
            TextFont { font: font.clone(), font_size: 48.0, ..default() },
            TextColor(Color::srgb(0.3, 1.0, 0.4)),
        ));

        root.spawn((
            Text::new(format!(
                "Planets cleared this run: {}   |   Station {}",
                planet_count.0,
                station_level.0 + 1,
            )),
            TextFont { font: font.clone(), font_size: 22.0, ..default() },
            TextColor(Color::srgb(0.7, 0.9, 0.7)),
        ));

        root.spawn((
            Button,
            EndScreenButtons::Continue,
            Node {
                width: Val::Px(420.0),
                height: Val::Px(60.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.4, 0.1, 0.9)),
            BorderColor(Color::srgba(0.2, 1.0, 0.3, 0.8)),
            BorderRadius::all(Val::Px(6.0)),
        ))
        .with_children(|b| {
            b.spawn((
                Text::new(format!("Continue to Station {}", station_level.0 + 2)),
                TextFont { font: font.clone(), font_size: 28.0, ..default() },
                TextColor(Color::WHITE),
            ));
        });

        root.spawn((
            Button,
            EndScreenButtons::Leave,
            Node {
                width: Val::Px(420.0),
                height: Val::Px(60.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.35, 0.05, 0.05, 0.9)),
            BorderColor(Color::srgba(1.0, 0.3, 0.3, 0.8)),
            BorderRadius::all(Val::Px(6.0)),
        ))
        .with_children(|b| {
            b.spawn((
                Text::new("Main Menu"),
                TextFont { font: font.clone(), font_size: 28.0, ..default() },
                TextColor(Color::WHITE),
            ));
        });
    });
}

pub(super) fn cleanup_planet_win_screen(
    mut commands: Commands,
    q: Query<Entity, With<PlanetWinScreen>>,
) {
    for e in &q {
        commands.entity(e).despawn();
    }
}
