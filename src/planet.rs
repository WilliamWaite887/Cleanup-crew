use bevy::prelude::*;
use crate::collidable::{Collidable, Collider};
use crate::enemies::{
    ActiveEnemy, AnimationTimer, Enemy, EnemyFrames, EnemyMoveSpeed, EnemyRes,
    HitAnimation, MeleeEnemy, Velocity, ENEMY_SPEED,
};
use crate::fluiddynamics::PulledByFluid;
use crate::map::GeneratedLevel;
use crate::procgen::ProcgenSet;
use crate::room::{Room, RoomVec};
use crate::{
    EndScreenButtons, GameEntity, GameState, PlanetCount, PlanetLevelMarker,
    StationLevel, FONT_PATH, TILE_SIZE, Z_ENTITIES,
};

// ── Components ───────────────────────────────────────────────────────────────

#[derive(Component)]
pub struct FinalBoss;

#[derive(Component)]
struct PlanetWinScreen;

// ── Plugin ───────────────────────────────────────────────────────────────────

pub struct PlanetPlugin;

impl Plugin for PlanetPlugin {
    fn build(&self, app: &mut App) {
        app
            // Inject planet map before load_map reads GeneratedLevel
            .add_systems(
                OnEnter(GameState::Loading),
                setup_planet_level
                    .in_set(ProcgenSet::BuildFullLevel)
                    .run_if(resource_exists::<PlanetLevelMarker>),
            )
            // On 3rd-station Playing: tint background to hint at nearby planet
            .add_systems(
                OnEnter(GameState::Playing),
                tint_station_background
                    .run_if(|sl: Res<StationLevel>, m: Option<Res<PlanetLevelMarker>>|
                        sl.0 % 3 == 2 && m.is_none()),
            )
            // On planet Playing: green surface tint + spawn boss
            .add_systems(
                OnEnter(GameState::Playing),
                (tint_planet_background, spawn_final_boss)
                    .run_if(resource_exists::<PlanetLevelMarker>),
            )
            // Detect boss death → trigger PlanetWin
            .add_systems(
                Update,
                check_planet_win
                    .run_if(in_state(GameState::Playing))
                    .run_if(resource_exists::<PlanetLevelMarker>),
            )
            // Planet win screen
            .add_systems(OnEnter(GameState::PlanetWin), setup_planet_win_screen)
            .add_systems(OnExit(GameState::PlanetWin), cleanup_planet_win_screen)
            // Restore background colour and remove marker when leaving Playing
            .add_systems(OnExit(GameState::Playing), restore_background);
    }
}

// ── Map layout ───────────────────────────────────────────────────────────────

// 50 cols × 38 rows.
// Boss arena:    rows 1-27, cols 1-48.
// Entry corridor: rows 28-36, cols 19-30. Player spawns at row 32, col 24 ('S').
const PLANET_MAP_FILE: &str = "assets/planet/planet_level.txt";

// Pre-computed world-space room bounds for a 50×38 map (TILE_SIZE = 32).
//   x0 = -(50*32)/2 + 16 = -784
//   y0 = -(38*32)/2 + 16 = -592
//   world_y(row) = y0 + (37 - row) * 32
const ARENA_TLC: Vec2 = Vec2::new(-752.0, 560.0);   // col=1,  row=1
const ARENA_BRC: Vec2 = Vec2::new(752.0, -272.0);   // col=48, row=27
const CORR_TLC:  Vec2 = Vec2::new(-176.0, -304.0);  // col=19, row=28
const CORR_BRC:  Vec2 = Vec2::new(176.0, -560.0);   // col=30, row=36

// Tile-grid coordinates used by generate_enemies_for_all_rooms.
const ARENA_TILE_TLC: Vec2 = Vec2::new(1.0, 1.0);
const ARENA_TILE_BRC: Vec2 = Vec2::new(48.0, 27.0);
const CORR_TILE_TLC:  Vec2 = Vec2::new(19.0, 28.0);
const CORR_TILE_BRC:  Vec2 = Vec2::new(30.0, 36.0);

// Boss spawn centre (row 13, col 25 ≈ arena middle).
const BOSS_SPAWN: Vec3 = Vec3::new(16.0, 176.0, Z_ENTITIES);

// ── Setup planet level ────────────────────────────────────────────────────────

fn setup_planet_level(mut commands: Commands) {
    // Inject the planet map so load_map reads it instead of the procgen grid.
    use std::io::{BufRead, BufReader};
    let file = std::fs::File::open(PLANET_MAP_FILE).expect("planet_level.txt not found");
    let rows: Vec<String> = BufReader::new(file)
        .lines()
        .map(|l| l.expect("line read error"))
        .collect();
    commands.insert_resource(GeneratedLevel(rows));

    // Build the two rooms manually.
    // Boss arena room_layout has no '#' tiles so generate_enemies_in_room
    // returns None and the room is auto-cleared on player entry.
    let boss_layout: Vec<String> = vec!["............".to_string(); 12];

    let mut rv = RoomVec(Vec::new());

    // Boss arena — not pre-cleared; auto-clears via the no-enemy path.
    rv.0.push(Room::new(
        ARENA_TLC, ARENA_BRC,
        ARENA_TILE_TLC, ARENA_TILE_BRC,
        boss_layout,
    ));

    // Entry corridor — pre-cleared; acts as the player's starting area.
    let corr_layout: Vec<String> = vec!["............".to_string(); 12];
    let mut corridor = Room::new(
        CORR_TLC, CORR_BRC,
        CORR_TILE_TLC, CORR_TILE_BRC,
        corr_layout,
    );
    corridor.cleared = true;
    corridor.visited = true;
    rv.0.push(corridor);

    commands.insert_resource(rv);
}

// ── Background tints ─────────────────────────────────────────────────────────

fn tint_station_background(mut clear_color: ResMut<ClearColor>) {
    // Slightly warmer purple — hints that a planet is nearby.
    // Replace this with a background sprite once planet_bg.png is ready.
    clear_color.0 = Color::srgb(0.06, 0.02, 0.10);
}

fn tint_planet_background(mut clear_color: ResMut<ClearColor>) {
    // Muted green — placeholder for planet surface atmosphere.
    // Replace with a proper background sprite once the asset exists.
    clear_color.0 = Color::srgb(0.03, 0.10, 0.04);
}

fn restore_background(
    mut commands: Commands,
    mut clear_color: ResMut<ClearColor>,
) {
    clear_color.0 = Color::srgb(0.02, 0.02, 0.06);
    commands.remove_resource::<PlanetLevelMarker>();
}

// ── Boss spawn ────────────────────────────────────────────────────────────────

fn spawn_final_boss(
    mut commands: Commands,
    enemy_res: Res<EnemyRes>,
    station_level: Res<StationLevel>,
) {
    let health_mult = 8.0 + station_level.0 as f32 * 3.0;
    let hp = 50.0 * health_mult;

    commands.spawn((
        Sprite::from_image(enemy_res.frames[0].clone()),
        Transform {
            translation: BOSS_SPAWN,
            scale: Vec3::splat(3.0),
            ..default()
        },
        Enemy,
        Velocity::new(),
        MeleeEnemy,
        AnimationTimer(Timer::from_seconds(0.3, TimerMode::Repeating)),
        EnemyFrames { handles: enemy_res.frames.clone(), index: 0 },
        ActiveEnemy,
        HitAnimation { timer: Timer::from_seconds(0.15, TimerMode::Once) },
        crate::enemies::Health(hp),
        crate::enemies::MaxHealth(hp),
        EnemyMoveSpeed(ENEMY_SPEED * 0.6),
        PulledByFluid { mass: 200.0 },
    )).insert((
        Collidable,
        Collider { half_extents: Vec2::splat(TILE_SIZE * 1.5) },
        FinalBoss,
        GameEntity,
    ));
}

// ── Win detection ─────────────────────────────────────────────────────────────

fn check_planet_win(
    mut next_state: ResMut<NextState<GameState>>,
    mut planet_count: ResMut<PlanetCount>,
    boss_q: Query<(), With<FinalBoss>>,
    mut boss_spawned: Local<bool>,
) {
    if !boss_q.is_empty() {
        *boss_spawned = true;
        return;
    }
    if *boss_spawned {
        planet_count.0 += 1;
        next_state.set(GameState::PlanetWin);
    }
}

// ── Planet win screen ─────────────────────────────────────────────────────────

pub fn setup_planet_win_screen(
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
        // Title
        root.spawn((
            Text::new("Planet Cleared!"),
            TextFont { font: font.clone(), font_size: 48.0, ..default() },
            TextColor(Color::srgb(0.3, 1.0, 0.4)),
        ));

        // Subtitle
        root.spawn((
            Text::new(format!(
                "Planets cleared this run: {}   |   Station {}",
                planet_count.0,
                station_level.0 + 1,
            )),
            TextFont { font: font.clone(), font_size: 22.0, ..default() },
            TextColor(Color::srgb(0.7, 0.9, 0.7)),
        ));

        // Continue button → next station
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

        // Leave button → main menu
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

fn cleanup_planet_win_screen(
    mut commands: Commands,
    q: Query<Entity, With<PlanetWinScreen>>,
) {
    for e in &q {
        commands.entity(e).despawn();
    }
}
