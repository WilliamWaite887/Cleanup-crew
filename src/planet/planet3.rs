use bevy::prelude::*;
use super::{MiniBoss, MiniBossArenaState, PlanetSignals};
use super::planet1::{
    P3_MINI_ARENA_TLC, P3_MINI_ARENA_BRC,
    P1_EROOM_TC_TLC,  P1_EROOM_TC_BRC,  P1_EROOM_TC_TILE_TLC,  P1_EROOM_TC_TILE_BRC,
    P1_EROOM_MR1_TLC, P1_EROOM_MR1_BRC, P1_EROOM_MR1_TILE_TLC, P1_EROOM_MR1_TILE_BRC,
    P1_EROOM_ML_TLC,  P1_EROOM_ML_BRC,  P1_EROOM_ML_TILE_TLC,  P1_EROOM_ML_TILE_BRC,
    P1_EROOM_MR2_TLC, P1_EROOM_MR2_BRC, P1_EROOM_MR2_TILE_TLC, P1_EROOM_MR2_TILE_BRC,
    P1_EROOM_BL_TLC,  P1_EROOM_BL_BRC,  P1_EROOM_BL_TILE_TLC,  P1_EROOM_BL_TILE_BRC,
    P1_EROOM_BC_TLC,  P1_EROOM_BC_BRC,  P1_EROOM_BC_TILE_TLC,  P1_EROOM_BC_TILE_BRC,
    P1_SPAWN_TLC, P1_SPAWN_BRC, P1_SPAWN_TILE_TLC, P1_SPAWN_TILE_BRC,
    P1_EXIT_TLC,  P1_EXIT_BRC,  P1_EXIT_TILE_TLC,  P1_EXIT_TILE_BRC,
    make_empty_layout, planet_enemy_room,
};
use crate::{GameEntity, FONT_PATH, TILE_SIZE, Z_ENTITIES};
use crate::collidable::{Collidable, Collider};
use crate::enemies::{
    ActiveEnemy, AnimationTimer, Enemy, EnemyFrames, EnemyMoveSpeed, EnemyRes,
    HitAnimation, MeleeEnemy, Velocity, ENEMY_SPEED,
};
use crate::map::{Door, TileRes};
use crate::player::Player;
use crate::room::{Room, RoomVec};
use crate::StationLevel;
use crate::PlanetCount;
use rand::random_range;

// ── Planet 3 room builder ─────────────────────────────────────────────────────

pub(super) fn build_planet3_rooms() -> RoomVec {
    let mut rv = RoomVec(Vec::new());

    // Top-right room is the mini-boss arena — omitted from RoomVec so enemies don't
    // pre-populate it; mini_boss_arena_trigger handles it instead.
    rv.0.push(planet_enemy_room(P1_EROOM_TC_TLC,  P1_EROOM_TC_BRC,  P1_EROOM_TC_TILE_TLC,  P1_EROOM_TC_TILE_BRC,  71, 20));
    rv.0.push(planet_enemy_room(P1_EROOM_MR1_TLC, P1_EROOM_MR1_BRC, P1_EROOM_MR1_TILE_TLC, P1_EROOM_MR1_TILE_BRC, 33, 20));
    rv.0.push(planet_enemy_room(P1_EROOM_ML_TLC,  P1_EROOM_ML_BRC,  P1_EROOM_ML_TILE_TLC,  P1_EROOM_ML_TILE_BRC,  33, 20));
    rv.0.push(planet_enemy_room(P1_EROOM_MR2_TLC, P1_EROOM_MR2_BRC, P1_EROOM_MR2_TILE_TLC, P1_EROOM_MR2_TILE_BRC, 33, 22));
    rv.0.push(planet_enemy_room(P1_EROOM_BL_TLC,  P1_EROOM_BL_BRC,  P1_EROOM_BL_TILE_TLC,  P1_EROOM_BL_TILE_BRC,  33, 20));
    rv.0.push(planet_enemy_room(P1_EROOM_BC_TLC,  P1_EROOM_BC_BRC,  P1_EROOM_BC_TILE_TLC,  P1_EROOM_BC_TILE_BRC,  33, 20));

    let mut spawn = Room::new(P1_SPAWN_TLC, P1_SPAWN_BRC, P1_SPAWN_TILE_TLC, P1_SPAWN_TILE_BRC, make_empty_layout());
    spawn.cleared = true;
    spawn.visited = true;
    rv.0.push(spawn);

    let mut exit = Room::new(P1_EXIT_TLC, P1_EXIT_BRC, P1_EXIT_TILE_TLC, P1_EXIT_TILE_BRC, make_empty_layout());
    exit.cleared = true;
    rv.0.push(exit);

    rv
}

// ── Planet 3 mini boss arena trigger ─────────────────────────────────────────

pub(super) fn mini_boss_arena_trigger(
    mut commands: Commands,
    player_q: Query<&Transform, With<Player>>,
    door_q: Query<(Entity, &Transform), With<Door>>,
    arena_state: Res<MiniBossArenaState>,
    enemy_res: Res<EnemyRes>,
    station_level: Res<StationLevel>,
    planet_count: Res<PlanetCount>,
    tiles: Res<TileRes>,
) {
    if planet_count.0 as usize != 2 { return; }
    if *arena_state != MiniBossArenaState::Idle { return; }

    let Ok(player_tf) = player_q.single() else { return };
    let pp = player_tf.translation.truncate();

    let inside = pp.x > P3_MINI_ARENA_TLC.x + 64.0
        && pp.x < P3_MINI_ARENA_BRC.x - 64.0
        && pp.y < P3_MINI_ARENA_TLC.y - 64.0
        && pp.y > P3_MINI_ARENA_BRC.y + 64.0;
    if !inside { return; }

    let hp = 750.0 + station_level.0 as f32 * 250.0;
    let mini_boss_pos = Vec3::new(
        (P3_MINI_ARENA_TLC.x + P3_MINI_ARENA_BRC.x) * 0.5,
        (P3_MINI_ARENA_TLC.y + P3_MINI_ARENA_BRC.y) * 0.5,
        Z_ENTITIES,
    );
    commands.spawn((
        (
            Sprite::from_image(enemy_res.frames[0].clone()),
            Transform { translation: mini_boss_pos, scale: Vec3::splat(2.5), ..default() },
            Enemy,
            Velocity::new(),
            MeleeEnemy,
            AnimationTimer(Timer::from_seconds(0.3, TimerMode::Repeating)),
            EnemyFrames { handles: enemy_res.frames.clone(), index: 0 },
            ActiveEnemy,
        ),
        (
            HitAnimation { timer: Timer::from_seconds(0.15, TimerMode::Once) },
            crate::enemies::Health(hp),
            crate::enemies::MaxHealth(hp),
            EnemyMoveSpeed(ENEMY_SPEED * 0.7),
            Collidable,
            Collider { half_extents: Vec2::splat(TILE_SIZE * 1.5) },
            MiniBoss,
            GameEntity,
        ),
    ));

    for (entity, door_tf) in &door_q {
        let x = door_tf.translation.x;
        let y = door_tf.translation.y;
        if x >= P3_MINI_ARENA_TLC.x && x <= P3_MINI_ARENA_BRC.x
            && y <= P3_MINI_ARENA_TLC.y && y >= P3_MINI_ARENA_BRC.y
        {
            commands.entity(entity).insert((
                Collidable,
                Collider { half_extents: Vec2::splat(TILE_SIZE * 0.5) },
                Sprite::from_image(tiles.closed_door.clone()),
            ));
        }
    }

    commands.insert_resource(MiniBossArenaState::Active);
}

// ── Planet 3 watch mini boss death ────────────────────────────────────────────

pub(super) fn watch_mini_boss_death(
    mut commands: Commands,
    mini_boss_q: Query<(), With<MiniBoss>>,
    arena_state: Res<MiniBossArenaState>,
    planet_count: Res<PlanetCount>,
    mut signals: ResMut<PlanetSignals>,
    player_q: Query<&Transform, With<Player>>,
    asset_server: Res<AssetServer>,
) {
    if planet_count.0 as usize != 2 { return; }
    if *arena_state != MiniBossArenaState::Active { return; }
    if !mini_boss_q.is_empty() { return; }

    let a = random_range(1u8..=5u8);
    let b = random_range(1u8..=5u8);
    let c = random_range(1u8..=5u8);
    signals.signals = [Some(a), Some(b), Some(c)];

    let popup_pos = player_q.single()
        .map(|tf| tf.translation + Vec3::new(0.0, TILE_SIZE * 3.0, 100.0))
        .unwrap_or(Vec3::new(0.0, 60.0, 100.0));

    let font: Handle<Font> = asset_server.load(FONT_PATH);
    commands.spawn((
        Text2d::new(format!("Signals acquired!  {}  {}  {}", a, b, c)),
        TextFont { font, font_size: 22.0, ..default() },
        TextColor(Color::srgb(0.3, 0.8, 1.0)),
        Transform::from_translation(popup_pos),
        crate::rewards::RewardPopup { timer: Timer::from_seconds(4.0, TimerMode::Once) },
        GameEntity,
    ));

    commands.insert_resource(MiniBossArenaState::Done);
}
