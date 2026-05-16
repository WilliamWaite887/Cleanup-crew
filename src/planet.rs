use bevy::prelude::*;
use crate::collidable::{Collidable, Collider};
use crate::enemies::{
    ActiveEnemy, AnimationTimer, Enemy, EnemyFrames, EnemyMoveSpeed, EnemyRes,
    HitAnimation, MeleeEnemy, Velocity, ENEMY_SPEED,
};
use crate::map::{Door, GeneratedLevel, TileRes};
use crate::procgen::ProcgenSet;
use crate::room::{Room, RoomVec};
use crate::station_code::StationCodes;
use crate::station_color::StationColors;
use crate::station_symbol::StationSymbols;
use crate::station_symbol::SYMBOL_CHARS;
use crate::rewards::RewardRes;
use rand::random_range;
use crate::{
    EndScreenButtons, GameEntity, GameState, MainCamera, PlanetCount, PlanetLevelMarker,
    StationLevel, FONT_PATH, SYMBOL_FONT_PATH, TILE_SIZE, WIN_H, WIN_W, Z_ENTITIES, Z_FLOOR,
};
use crate::player::{Player, aabb_overlap};

// ── Components ───────────────────────────────────────────────────────────────

#[derive(Component)]
struct BackgroundSprite;

#[derive(Resource)]
struct BackgroundRes {
    stars: Handle<Image>,
    planet_station: Handle<Image>,
}

#[derive(Component)]
pub struct FinalBoss;

#[derive(Component)]
struct PlanetWinScreen;

#[derive(Component)]
struct BossHealthBarRoot;

#[derive(Component)]
struct BossHealthBarFill;

/// A door on the planet level that requires the 3-digit station code to open.
#[derive(Component)]
pub struct CodeDoor {
    pub unlocked: bool,
}

/// Marker for the floating "[E] Enter Code" prompt near a locked door.
#[derive(Component)]
struct CodeDoorPrompt;

/// Marker for the keypad UI overlay.
#[derive(Component)]
struct CodeEntryUi;

/// Marker for the individual digit Text nodes inside the keypad.
#[derive(Component)]
struct CodeDigitSlot(usize);

/// Marker for the keypad status line ("INCORRECT CODE" / "ENTER CODE").
#[derive(Component)]
struct CodeStatusText;

/// Tracks whether the boss arena has been entered and the boss spawned.
#[derive(Resource, PartialEq, Eq)]
enum BossArenaState {
    Idle,
    Active,
}

/// Collidable wall that seals the exit corridor until the boss is defeated.
#[derive(Component)]
struct BossExitDoor;

/// The "Leave Planet" interactable that spawns in the exit room after the boss dies.
#[derive(Component)]
struct PlanetExitBeacon;

/// Active code-entry session.
#[derive(Resource)]
pub struct CodeEntryState {
    door_entity: Entity,
    entered: [u8; 3],
    cursor: usize,
    wrong_timer: Option<Timer>,
}

/// Color terminal — requires the 3 station color clues to unlock.
#[derive(Component)]
pub struct ColorTerminal {
    pub unlocked: bool,
}

/// Symbol terminal — requires the 3 station symbol clues to unlock.
#[derive(Component)]
pub struct SymbolTerminal {
    pub unlocked: bool,
}

/// Frequency Master — requires all 3 signal strengths; opens the boss arena gate.
#[derive(Component)]
pub struct FreqMaster {
    pub unlocked: bool,
}

/// Signal strengths revealed by solving the 3 sub-puzzles.
/// [0] = Signal A (CodeDoor), [1] = Signal B (ColorTerminal), [2] = Signal C (SymbolTerminal).
/// Generated fresh each planet; not persisted.
#[derive(Resource, Default)]
pub struct PlanetSignals {
    pub signals: [Option<u8>; 3],
}

#[derive(Clone, Copy, PartialEq)]
enum TerminalKind {
    Color,
    Symbol,
    Freq,
}

/// Active terminal session for Color / Symbol / Freq Master keypads.
#[derive(Resource)]
pub struct TerminalSession {
    terminal_entity: Entity,
    kind: TerminalKind,
    entered: [u8; 3],
    cursor: usize,
    wrong_timer: Option<Timer>,
}

/// Marker for the floating prompt near a terminal.
#[derive(Component)]
struct TerminalPrompt;

/// Marker for the terminal keypad UI overlay.
#[derive(Component)]
struct TerminalUi;

/// Marker for individual value slots inside the terminal keypad.
#[derive(Component)]
struct TerminalSlot(usize);

/// Marker for the terminal status text line.
#[derive(Component)]
struct TerminalStatusText;

// ── Plugin ───────────────────────────────────────────────────────────────────

pub struct PlanetPlugin;

impl Plugin for PlanetPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, load_background_assets)
            .add_systems(
                OnEnter(GameState::Loading),
                setup_planet_level
                    .in_set(ProcgenSet::BuildFullLevel)
                    .after(ProcgenSet::LoadRooms)
                    .run_if(resource_exists::<PlanetLevelMarker>),
            )
            .add_systems(
                OnEnter(GameState::Playing),
                tint_station_background
                    .run_if(|sl: Res<StationLevel>, m: Option<Res<PlanetLevelMarker>>|
                        sl.0 % 3 == 2 && m.is_none()),
            )
            .add_systems(
                OnEnter(GameState::Playing),
                spawn_stars_background
                    .run_if(|sl: Res<StationLevel>, m: Option<Res<PlanetLevelMarker>>|
                        sl.0 % 3 != 2 && m.is_none()),
            )
            .add_systems(
                OnEnter(GameState::Playing),
                spawn_planet_station_background
                    .run_if(|sl: Res<StationLevel>, m: Option<Res<PlanetLevelMarker>>|
                        sl.0 % 3 == 2 && m.is_none()),
            )
            .add_systems(
                OnEnter(GameState::Playing),
                (tint_planet_background, init_boss_arena_state, spawn_vault_rewards, spawn_boss_exit_door)
                    .run_if(resource_exists::<PlanetLevelMarker>),
            )
            .add_systems(
                Update,
                update_background_position.run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                Update,
                (
                    boss_arena_trigger,
                    spawn_boss_chest,
                    interact_with_exit_beacon,
                    update_boss_health_bar,
                )
                    .run_if(in_state(GameState::Playing))
                    .run_if(resource_exists::<PlanetLevelMarker>),
            )
            .add_systems(
                Update,
                (code_door_proximity, update_code_entry_ui)
                    .run_if(in_state(GameState::Playing))
                    .run_if(resource_exists::<PlanetLevelMarker>),
            )
            .add_systems(
                OnEnter(GameState::Playing),
                init_planet_signals
                    .run_if(resource_exists::<PlanetLevelMarker>),
            )
            .add_systems(
                Update,
                (terminal_proximity, update_terminal_ui)
                    .run_if(in_state(GameState::Playing))
                    .run_if(resource_exists::<PlanetLevelMarker>),
            )
            .add_systems(OnEnter(GameState::PlanetWin), setup_planet_win_screen)
            .add_systems(OnExit(GameState::PlanetWin), cleanup_planet_win_screen)
            .add_systems(OnExit(GameState::Playing), restore_background);
    }
}

// ── Per-planet data ───────────────────────────────────────────────────────────
//
// Map dimensions: 300 cols × 200 rows  (TILE_SIZE = 32)
//   x0 = -(300*32)/2 + 16 = -4784
//   y0 = -(200*32)/2 + 16 = -3184
//   world_x(col) = -4784 + col * 32
//   world_y(row) = -3184 + (199 - row) * 32

fn planet_map_file(planet_idx: usize) -> &'static str {
    match planet_idx {
        0 => "assets/planet/planet1_level.txt",
        // Planets 2 and 3 reuse planet 1 until their maps are built.
        _ => "assets/planet/planet1_level.txt",
    }
}

fn planet_boss_spawn(planet_idx: usize) -> Vec3 {
    match planet_idx {
        _ => P1_BOSS_SPAWN,
    }
}

fn planet_vault_rewards(planet_idx: usize) -> &'static [Vec3] {
    match planet_idx {
        _ => &P1_VAULT_REWARDS,
    }
}

fn build_planet_rooms(planet_idx: usize) -> RoomVec {
    match planet_idx {
        _ => build_planet1_rooms(),
    }
}

// ── Planet 1 constants ────────────────────────────────────────────────────────
//
// Layout (300×200 map):
//
//   Boss Arena         : rows  4-51,  cols   2-59   (top-left)
//   E Room Top-Right   : rows  6-49,  cols 201-249  (top-right)
//   E Room Top-Center  : rows 21-40,  cols  79-149  (top-center)
//   E Room Mid-Right 1 : rows 99-118, cols 211-243
//   E Room Mid-Left    : rows128-147, cols  65-97
//   E Room Mid-Right 2 : rows139-160, cols 211-243
//   E Room Bot-Left    : rows167-186, cols  56-88
//   E Room Bot-Center  : rows167-186, cols 114-146
//   Spawn Room (S)     : rows178-187, cols 211-243  (pre-cleared)
//   Vault (V)          : rows133-142, cols   5-23   (behind code door at col 23)

// Boss Arena
const P1_ARENA_TLC: Vec2 = Vec2::new(-4720.0, 3056.0);
const P1_ARENA_BRC: Vec2 = Vec2::new(-2896.0, 1552.0);

// E Room — Top Right
const P1_EROOM_TR_TLC:      Vec2 = Vec2::new(1648.0, 2992.0);
const P1_EROOM_TR_BRC:      Vec2 = Vec2::new(3184.0, 1616.0);
const P1_EROOM_TR_TILE_TLC: Vec2 = Vec2::new(201.0,  6.0);
const P1_EROOM_TR_TILE_BRC: Vec2 = Vec2::new(249.0, 49.0);

// E Room — Top Center
const P1_EROOM_TC_TLC:      Vec2 = Vec2::new(-2256.0, 2512.0);
const P1_EROOM_TC_BRC:      Vec2 = Vec2::new(-16.0,   1904.0);
const P1_EROOM_TC_TILE_TLC: Vec2 = Vec2::new(79.0,  21.0);
const P1_EROOM_TC_TILE_BRC: Vec2 = Vec2::new(149.0, 40.0);

// E Room — Mid Right 1
const P1_EROOM_MR1_TLC:      Vec2 = Vec2::new(1968.0,  16.0);
const P1_EROOM_MR1_BRC:      Vec2 = Vec2::new(2992.0, -592.0);
const P1_EROOM_MR1_TILE_TLC: Vec2 = Vec2::new(211.0,  99.0);
const P1_EROOM_MR1_TILE_BRC: Vec2 = Vec2::new(243.0, 118.0);

// E Room — Mid Left
const P1_EROOM_ML_TLC:      Vec2 = Vec2::new(-2704.0,  -912.0);
const P1_EROOM_ML_BRC:      Vec2 = Vec2::new(-1680.0, -1520.0);
const P1_EROOM_ML_TILE_TLC: Vec2 = Vec2::new(65.0,  128.0);
const P1_EROOM_ML_TILE_BRC: Vec2 = Vec2::new(97.0,  147.0);

// E Room — Mid Right 2
const P1_EROOM_MR2_TLC:      Vec2 = Vec2::new(1968.0, -1264.0);
const P1_EROOM_MR2_BRC:      Vec2 = Vec2::new(2992.0, -1936.0);
const P1_EROOM_MR2_TILE_TLC: Vec2 = Vec2::new(211.0, 139.0);
const P1_EROOM_MR2_TILE_BRC: Vec2 = Vec2::new(243.0, 160.0);

// E Room — Bottom Left
const P1_EROOM_BL_TLC:      Vec2 = Vec2::new(-2992.0, -2160.0);
const P1_EROOM_BL_BRC:      Vec2 = Vec2::new(-1968.0, -2768.0);
const P1_EROOM_BL_TILE_TLC: Vec2 = Vec2::new(56.0,  167.0);
const P1_EROOM_BL_TILE_BRC: Vec2 = Vec2::new(88.0,  186.0);

// E Room — Bottom Center
const P1_EROOM_BC_TLC:      Vec2 = Vec2::new(-1136.0, -2160.0);
const P1_EROOM_BC_BRC:      Vec2 = Vec2::new(-112.0,  -2768.0);
const P1_EROOM_BC_TILE_TLC: Vec2 = Vec2::new(114.0, 167.0);
const P1_EROOM_BC_TILE_BRC: Vec2 = Vec2::new(146.0, 186.0);

// Spawn Room
const P1_SPAWN_TLC:      Vec2 = Vec2::new(1968.0, -2512.0);
const P1_SPAWN_BRC:      Vec2 = Vec2::new(2992.0, -2800.0);
const P1_SPAWN_TILE_TLC: Vec2 = Vec2::new(211.0, 178.0);
const P1_SPAWN_TILE_BRC: Vec2 = Vec2::new(243.0, 187.0);

// Boss spawns near the centre of the arena (col 30, row 20).
const P1_BOSS_SPAWN: Vec3 = Vec3::new(-3824.0, 2544.0, Z_ENTITIES);

// Chest spawns 4 tiles below the boss spawn after the boss is defeated.
const BOSS_CHEST_POS: Vec3 = Vec3::new(-3824.0, 2416.0, Z_ENTITIES);

// Exit corridor — 3-tile breach in the south wall of the boss arena (row 52, col 30 centre).
const BOSS_EXIT_DOOR_POS: Vec3 = Vec3::new(-3824.0, 1520.0, Z_ENTITIES);

// "Leave Planet" beacon — centre of the exit room (row 60, col 30).
const PLANET_EXIT_BEACON_POS: Vec3 = Vec3::new(-3824.0, 1264.0, Z_ENTITIES);

// Exit room (rows 54-66, cols 23-37).
const P1_EXIT_TLC:      Vec2 = Vec2::new(-4048.0, 1456.0);
const P1_EXIT_BRC:      Vec2 = Vec2::new(-3600.0, 1072.0);
const P1_EXIT_TILE_TLC: Vec2 = Vec2::new(23.0, 54.0);
const P1_EXIT_TILE_BRC: Vec2 = Vec2::new(37.0, 66.0);

// Vault reward positions (cols 5-23, rows 133-142); code door at col 23.
static P1_VAULT_REWARDS: [Vec3; 3] = [
    Vec3::new(-4528.0, -1200.0, Z_ENTITIES),  // col=8,  row=137
    Vec3::new(-4400.0, -1200.0, Z_ENTITIES),  // col=12, row=137
    Vec3::new(-4272.0, -1200.0, Z_ENTITIES),  // col=16, row=137
];

// ── Room builder ──────────────────────────────────────────────────────────────

fn make_enemy_layout(width: usize, height: usize) -> Vec<String> {
    let row = "#".repeat(width);
    vec![row; height]
}

fn make_empty_layout() -> Vec<String> {
    vec!["............".to_string(); 4]
}

fn planet_enemy_room(tlc: Vec2, brc: Vec2, tile_tlc: Vec2, tile_brc: Vec2, w: usize, h: usize) -> Room {
    let mut r = Room::new(tlc, brc, tile_tlc, tile_brc, make_enemy_layout(w, h));
    r.base_enemies = 6;
    r.health_mult = 2.5;
    r
}

fn build_planet1_rooms() -> RoomVec {
    let mut rv = RoomVec(Vec::new());

    // Seven enemy rooms (boss arena is NOT in RoomVec — handled by boss_arena_trigger).
    rv.0.push(planet_enemy_room(P1_EROOM_TR_TLC,  P1_EROOM_TR_BRC,  P1_EROOM_TR_TILE_TLC,  P1_EROOM_TR_TILE_BRC,  49, 44));
    rv.0.push(planet_enemy_room(P1_EROOM_TC_TLC,  P1_EROOM_TC_BRC,  P1_EROOM_TC_TILE_TLC,  P1_EROOM_TC_TILE_BRC,  71, 20));
    rv.0.push(planet_enemy_room(P1_EROOM_MR1_TLC, P1_EROOM_MR1_BRC, P1_EROOM_MR1_TILE_TLC, P1_EROOM_MR1_TILE_BRC, 33, 20));
    rv.0.push(planet_enemy_room(P1_EROOM_ML_TLC,  P1_EROOM_ML_BRC,  P1_EROOM_ML_TILE_TLC,  P1_EROOM_ML_TILE_BRC,  33, 20));
    rv.0.push(planet_enemy_room(P1_EROOM_MR2_TLC, P1_EROOM_MR2_BRC, P1_EROOM_MR2_TILE_TLC, P1_EROOM_MR2_TILE_BRC, 33, 22));
    rv.0.push(planet_enemy_room(P1_EROOM_BL_TLC,  P1_EROOM_BL_BRC,  P1_EROOM_BL_TILE_TLC,  P1_EROOM_BL_TILE_BRC,  33, 20));
    rv.0.push(planet_enemy_room(P1_EROOM_BC_TLC,  P1_EROOM_BC_BRC,  P1_EROOM_BC_TILE_TLC,  P1_EROOM_BC_TILE_BRC,  33, 20));

    // Spawn room — player starts here; pre-cleared so no enemy trigger fires.
    let mut spawn = Room::new(
        P1_SPAWN_TLC, P1_SPAWN_BRC,
        P1_SPAWN_TILE_TLC, P1_SPAWN_TILE_BRC,
        make_empty_layout(),
    );
    spawn.cleared = true;
    spawn.visited = true;
    rv.0.push(spawn);

    // Exit room — accessible after boss is defeated; pre-cleared, revealed on entry.
    let mut exit = Room::new(
        P1_EXIT_TLC, P1_EXIT_BRC,
        P1_EXIT_TILE_TLC, P1_EXIT_TILE_BRC,
        make_empty_layout(),
    );
    exit.cleared = true;
    rv.0.push(exit);

    rv
}

// ── Setup planet level ────────────────────────────────────────────────────────

fn setup_planet_level(mut commands: Commands, planet_count: Res<PlanetCount>) {
    use std::io::{BufRead, BufReader};
    let planet_idx = planet_count.0 as usize;
    let map_path = planet_map_file(planet_idx);

    let file = match std::fs::File::open(map_path) {
        Ok(f) => f,
        Err(e) => {
            warn!("Could not open planet level file '{}': {e}", map_path);
            return;
        }
    };
    let rows: Vec<String> = BufReader::new(file)
        .lines()
        .filter_map(|l| l.ok())
        .collect();
    commands.insert_resource(GeneratedLevel(rows));
    commands.insert_resource(build_planet_rooms(planet_idx));
}

// ── Background tints & images ─────────────────────────────────────────────────

fn load_background_assets(mut commands: Commands, assets: Res<AssetServer>) {
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

fn spawn_stars_background(
    mut commands: Commands,
    bg: Res<BackgroundRes>,
    window_q: Query<&Window>,
) {
    let size = window_q.single()
        .map(|w| Vec2::new(w.width(), w.height()))
        .unwrap_or(Vec2::new(WIN_W, WIN_H));
    spawn_background(&mut commands, bg.stars.clone(), size);
}

fn spawn_planet_station_background(
    mut commands: Commands,
    bg: Res<BackgroundRes>,
    window_q: Query<&Window>,
) {
    let size = window_q.single()
        .map(|w| Vec2::new(w.width(), w.height()))
        .unwrap_or(Vec2::new(WIN_W, WIN_H));
    spawn_background(&mut commands, bg.planet_station.clone(), size);
}

fn update_background_position(
    camera_q: Query<&Transform, With<MainCamera>>,
    mut bg_q: Query<&mut Transform, (With<BackgroundSprite>, Without<MainCamera>)>,
) {
    let Ok(cam_tf) = camera_q.single() else { return };
    for mut bg_tf in &mut bg_q {
        bg_tf.translation.x = cam_tf.translation.x;
        bg_tf.translation.y = cam_tf.translation.y;
    }
}

fn tint_station_background(mut clear_color: ResMut<ClearColor>) {
    clear_color.0 = Color::srgb(0.06, 0.02, 0.10);
}

fn tint_planet_background(mut clear_color: ResMut<ClearColor>) {
    clear_color.0 = Color::srgb(0.03, 0.10, 0.04);
}

fn restore_background(
    mut commands: Commands,
    mut clear_color: ResMut<ClearColor>,
    bar_q: Query<Entity, With<BossHealthBarRoot>>,
    bg_q: Query<Entity, With<BackgroundSprite>>,
) {
    clear_color.0 = Color::srgb(0.02, 0.02, 0.06);
    commands.remove_resource::<PlanetLevelMarker>();
    commands.remove_resource::<BossArenaState>();
    for e in &bar_q {
        commands.entity(e).despawn();
    }
    for e in &bg_q {
        commands.entity(e).despawn();
    }
}

// ── Boss arena state ──────────────────────────────────────────────────────────

fn init_boss_arena_state(mut commands: Commands) {
    commands.insert_resource(BossArenaState::Idle);
}

/// Spawns a collidable wall entity sealing the exit corridor south of the boss arena.
/// Despawned by `spawn_boss_chest` when the boss is defeated.
fn spawn_boss_exit_door(mut commands: Commands, tiles: Res<TileRes>) {
    commands.spawn((
        Sprite::from_image(tiles.closed_door.clone()),
        Transform::from_translation(BOSS_EXIT_DOOR_POS),
        Collidable,
        Collider { half_extents: Vec2::new(TILE_SIZE * 1.5, TILE_SIZE * 0.5) },
        BossExitDoor,
        crate::GameEntity,
    ));
}

/// Detects when the player steps inside the boss arena and triggers the encounter.
fn boss_arena_trigger(
    mut commands: Commands,
    player_q: Query<&Transform, With<Player>>,
    door_q: Query<(Entity, &Transform), With<Door>>,
    boss_arena_state: Res<BossArenaState>,
    enemy_res: Res<EnemyRes>,
    station_level: Res<StationLevel>,
    planet_count: Res<PlanetCount>,
    asset_server: Res<AssetServer>,
    tiles: Res<TileRes>,
) {
    if *boss_arena_state != BossArenaState::Idle { return; }

    let Ok(player_tf) = player_q.single() else { return };
    let pp = player_tf.translation.truncate();

    // 64px inset so the trigger doesn't fire in the doorway itself
    let inside = pp.x > P1_ARENA_TLC.x + 64.0
        && pp.x < P1_ARENA_BRC.x - 64.0
        && pp.y < P1_ARENA_TLC.y - 64.0
        && pp.y > P1_ARENA_BRC.y + 64.0;
    if !inside { return; }

    // Spawn boss
    let hp = 1500.0 + station_level.0 as f32 * 500.0;
    let boss_pos = planet_boss_spawn(planet_count.0 as usize);
    commands.spawn((
        Sprite::from_image(enemy_res.frames[0].clone()),
        Transform { translation: boss_pos, scale: Vec3::splat(3.0), ..default() },
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
    )).insert((
        Collidable,
        Collider { half_extents: Vec2::splat(TILE_SIZE * 1.5) },
        FinalBoss,
        GameEntity,
    ));

    // Close any door entities inside or at the entrance of the arena
    for (entity, door_tf) in &door_q {
        let x = door_tf.translation.x;
        let y = door_tf.translation.y;
        if x >= P1_ARENA_TLC.x
            && x <= P1_ARENA_BRC.x + 128.0
            && y <= P1_ARENA_TLC.y
            && y >= P1_ARENA_BRC.y
        {
            commands.entity(entity).insert((
                Collidable,
                Collider { half_extents: Vec2::splat(TILE_SIZE * 0.5) },
                Sprite::from_image(tiles.closed_door.clone()),
            ));
        }
    }

    // Spawn the boss health bar UI
    do_spawn_boss_health_bar(&mut commands, &asset_server);

    commands.insert_resource(BossArenaState::Active);
}

// ── Boss health bar ───────────────────────────────────────────────────────────

fn do_spawn_boss_health_bar(commands: &mut Commands, asset_server: &AssetServer) {
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

fn update_boss_health_bar(
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

fn spawn_boss_chest(
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

    // Chest always spawns — key is needed to open it.
    commands.spawn((
        Sprite::from_image(key_chest_res.chest_img.clone()),
        Transform::from_translation(BOSS_CHEST_POS),
        crate::key_chest::Chest,
        Collidable,
        Collider { half_extents: Vec2::splat(TILE_SIZE * 0.5) },
        crate::GameEntity,
    ));

    // Open the exit corridor.
    for entity in &exit_door_q {
        commands.entity(entity).despawn();
    }

    // Spawn the "Leave Planet" beacon in the exit room.
    let font: Handle<Font> = asset_server.load(FONT_PATH);
    commands.spawn((
        Text2d::new("[ E ]  Leave Planet"),
        TextFont { font, font_size: 24.0, ..default() },
        TextColor(Color::srgb(0.3, 1.0, 0.4)),
        Transform::from_translation(PLANET_EXIT_BEACON_POS),
        PlanetExitBeacon,
        crate::GameEntity,
    ));

    key_state.boss_chest_spawned = true;
}

// ── Planet exit — player presses E near the beacon ───────────────────────────

fn interact_with_exit_beacon(
    input: Res<ButtonInput<KeyCode>>,
    player_q: Query<&Transform, With<Player>>,
    beacon_q: Query<&Transform, With<PlanetExitBeacon>>,
    boss_arena_state: Res<BossArenaState>,
    session: Option<Res<TerminalSession>>,
    code_session: Option<Res<CodeEntryState>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut planet_count: ResMut<PlanetCount>,
    bindings: Res<crate::settings::KeyBindings>,
) {
    if *boss_arena_state != BossArenaState::Active { return; }
    if session.is_some() || code_session.is_some() { return; }
    if !input.just_pressed(bindings.interact) { return; }

    let Ok(player_tf) = player_q.single() else { return };
    let Ok(beacon_tf) = beacon_q.single() else { return };
    let pp = player_tf.translation;
    let bp = beacon_tf.translation;

    if aabb_overlap(pp.x, pp.y, Vec2::splat(TILE_SIZE * 2.0), bp.x, bp.y, Vec2::splat(TILE_SIZE * 1.5)) {
        planet_count.0 += 1;
        next_state.set(GameState::PlanetWin);
    }
}

// ── Vault rewards ─────────────────────────────────────────────────────────────

fn spawn_vault_rewards(
    mut commands: Commands,
    reward_res: Res<RewardRes>,
    planet_count: Res<PlanetCount>,
) {
    for &pos in planet_vault_rewards(planet_count.0 as usize) {
        crate::rewards::spawn_reward(&mut commands, pos, &reward_res);
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

fn cleanup_planet_win_screen(
    mut commands: Commands,
    q: Query<Entity, With<PlanetWinScreen>>,
) {
    for e in &q {
        commands.entity(e).despawn();
    }
}

// ── Code door systems ─────────────────────────────────────────────────────────

fn code_door_proximity(
    mut commands: Commands,
    player_q: Query<&Transform, With<Player>>,
    door_q: Query<(Entity, &Transform, &CodeDoor)>,
    prompt_q: Query<Entity, With<CodeDoorPrompt>>,
    entry_state: Option<Res<CodeEntryState>>,
    input: Res<ButtonInput<KeyCode>>,
    bindings: Res<crate::settings::KeyBindings>,
    asset_server: Res<AssetServer>,
) {
    if entry_state.is_some() { return; }

    let Ok(player_tf) = player_q.single() else { return };
    let pp = player_tf.translation;
    let interact_half = Vec2::splat(TILE_SIZE * 2.5);
    let door_half = Vec2::splat(TILE_SIZE * 0.5);

    let mut near_door: Option<(Entity, Vec3)> = None;
    for (entity, door_tf, door) in &door_q {
        if door.unlocked { continue; }
        let dp = door_tf.translation;
        if aabb_overlap(pp.x, pp.y, interact_half, dp.x, dp.y, door_half) {
            near_door = Some((entity, dp));
            break;
        }
    }

    for e in &prompt_q {
        commands.entity(e).despawn();
    }

    if let Some((door_entity, door_pos)) = near_door {
        let font: Handle<Font> = asset_server.load(FONT_PATH);
        commands.spawn((
            Text2d::new("[E] Enter Code"),
            TextFont { font, font_size: 18.0, ..default() },
            TextColor(Color::srgb(0.2, 1.0, 1.0)),
            Transform::from_translation(door_pos + Vec3::new(0.0, TILE_SIZE * 1.5, 10.0)),
            CodeDoorPrompt,
            GameEntity,
        ));

        if input.just_pressed(bindings.interact) {
            commands.insert_resource(CodeEntryState {
                door_entity,
                entered: [0; 3],
                cursor: 0,
                wrong_timer: None,
            });
            spawn_code_entry_ui(&mut commands, &asset_server);
        }
    }
}

fn spawn_code_entry_ui(commands: &mut Commands, asset_server: &AssetServer) {
    let font: Handle<Font> = asset_server.load(FONT_PATH);

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(320.0),
                height: Val::Auto,
                left: Val::Percent(50.0),
                top: Val::Percent(40.0),
                margin: UiRect {
                    left: Val::Px(-160.0),
                    ..default()
                },
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(12.0),
                padding: UiRect::all(Val::Px(20.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.05, 0.1, 0.95)),
            BorderColor(Color::srgba(0.2, 0.8, 1.0, 0.8)),
            BorderRadius::all(Val::Px(8.0)),
            ZIndex(30),
            CodeEntryUi,
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new("ENTER CODE"),
                TextFont { font: font.clone(), font_size: 22.0, ..default() },
                TextColor(Color::srgb(0.2, 1.0, 1.0)),
            ));

            panel
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(12.0),
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|row| {
                    for i in 0..3usize {
                        row.spawn((
                            Text::new("> 0 <"),
                            TextFont { font: font.clone(), font_size: 28.0, ..default() },
                            TextColor(if i == 0 { Color::WHITE } else { Color::srgb(0.5, 0.5, 0.5) }),
                            CodeDigitSlot(i),
                        ));
                    }
                });

            panel.spawn((
                Text::new("W/S change  A/D move  Enter=submit  E=close"),
                TextFont { font: font.clone(), font_size: 14.0, ..default() },
                TextColor(Color::srgb(0.6, 0.6, 0.6)),
                CodeStatusText,
            ));
        });
}

fn update_code_entry_ui(
    mut commands: Commands,
    entry_state: Option<ResMut<CodeEntryState>>,
    codes: Res<StationCodes>,
    mut signals: ResMut<PlanetSignals>,
    input: Res<ButtonInput<KeyCode>>,
    bindings: Res<crate::settings::KeyBindings>,
    time: Res<Time>,
    mut digit_q: Query<(&CodeDigitSlot, &mut Text, &mut TextColor)>,
    mut status_q: Query<(&mut Text, &mut TextColor), (With<CodeStatusText>, Without<CodeDigitSlot>)>,
    ui_q: Query<Entity, With<CodeEntryUi>>,
    mut door_q: Query<(Entity, &mut Sprite), With<CodeDoor>>,
    asset_server: Res<AssetServer>,
) {
    let Some(mut state) = entry_state else { return };

    if let Some(ref mut timer) = state.wrong_timer {
        timer.tick(time.delta());
        if timer.just_finished() {
            state.wrong_timer = None;
            if let Ok((mut txt, mut col)) = status_q.single_mut() {
                *txt = Text::new("W/S change  A/D move  Enter=submit  E=close");
                *col = TextColor(Color::srgb(0.6, 0.6, 0.6));
            }
        }
        return;
    }

    if input.just_pressed(bindings.interact) {
        close_keypad(&mut commands, &ui_q);
        return;
    }

    let cursor_before = state.cursor;

    if input.just_pressed(bindings.move_left) && state.cursor > 0 {
        state.cursor -= 1;
    }
    if input.just_pressed(bindings.move_right) && state.cursor < 2 {
        state.cursor += 1;
    }
    if input.just_pressed(bindings.move_up) {
        let idx = state.cursor;
        state.entered[idx] = (state.entered[idx] + 1) % 10;
    }
    if input.just_pressed(bindings.move_down) {
        let idx = state.cursor;
        state.entered[idx] = (state.entered[idx] + 9) % 10;
    }

    let cursor_changed = cursor_before != state.cursor;
    for (slot, mut txt, mut col) in &mut digit_q {
        let i = slot.0;
        let d = state.entered[i];
        if i == state.cursor {
            *txt = Text::new(format!("> {} <", d));
            *col = TextColor(Color::WHITE);
        } else {
            *txt = Text::new(format!("  {}  ", d));
            *col = TextColor(Color::srgb(0.5, 0.5, 0.5));
        }
        let _ = cursor_changed;
    }

    if input.just_pressed(KeyCode::Enter) {
        let correct = codes.codes.iter().zip(state.entered.iter()).all(|(stored, entered)| {
            stored.map_or(false, |d| d == *entered)
        });

        if correct {
            let open_door: Handle<Image> = asset_server.load("map/open_door.png");
            // Unlock every CodeDoor tile (the vault door spans multiple tiles).
            for (door_entity, _) in door_q.iter() {
                commands.entity(door_entity).remove::<Collidable>();
                commands.entity(door_entity).remove::<Collider>();
                if let Ok(mut e) = commands.get_entity(door_entity) {
                    e.insert(CodeDoor { unlocked: true });
                }
            }
            for (_, mut sprite) in door_q.iter_mut() {
                sprite.image = open_door.clone();
            }

            let sig_a: u8 = random_range(1u8..=5u8);
            signals.signals[0] = Some(sig_a);
            let font: Handle<Font> = asset_server.load(FONT_PATH);
            commands.spawn((
                Text2d::new(format!("Signal Strength A: {}", sig_a)),
                TextFont { font, font_size: 20.0, ..default() },
                TextColor(Color::srgb(0.2, 1.0, 0.5)),
                Transform::from_translation(Vec3::new(0.0, 60.0, 100.0)),
                crate::rewards::RewardPopup { timer: Timer::from_seconds(3.0, TimerMode::Once) },
                GameEntity,
            ));

            close_keypad(&mut commands, &ui_q);
        } else {
            if let Ok((mut txt, mut col)) = status_q.single_mut() {
                *txt = Text::new("✗  INCORRECT CODE  ✗");
                *col = TextColor(Color::srgb(1.0, 0.2, 0.2));
            }
            state.wrong_timer = Some(Timer::from_seconds(1.5, TimerMode::Once));
        }
    }
}

fn close_keypad(commands: &mut Commands, ui_q: &Query<Entity, With<CodeEntryUi>>) {
    for e in ui_q.iter() {
        commands.entity(e).despawn();
    }
    commands.remove_resource::<CodeEntryState>();
}

// ── Planet signals init ───────────────────────────────────────────────────────

fn init_planet_signals(mut commands: Commands) {
    commands.insert_resource(PlanetSignals::default());
}

// ── Terminal helpers ──────────────────────────────────────────────────────────

fn terminal_display(kind: TerminalKind, val: u8) -> &'static str {
    match kind {
        TerminalKind::Color  => ["RED", "GRN", "BLU", "YLW"][val as usize],
        TerminalKind::Symbol => SYMBOL_CHARS[val as usize],
        TerminalKind::Freq   => ["▰▱▱▱▱", "▰▰▱▱▱", "▰▰▰▱▱", "▰▰▰▰▱", "▰▰▰▰▰"][val as usize],
    }
}

fn terminal_max(kind: TerminalKind) -> u8 {
    match kind {
        TerminalKind::Color  => 3,
        TerminalKind::Symbol => 5,
        TerminalKind::Freq   => 4,
    }
}

fn terminal_title(kind: TerminalKind) -> &'static str {
    match kind {
        TerminalKind::Color  => "COLOR TERMINAL",
        TerminalKind::Symbol => "SYMBOL TERMINAL",
        TerminalKind::Freq   => "FREQUENCY MASTER",
    }
}

fn terminal_accent(kind: TerminalKind) -> Color {
    match kind {
        TerminalKind::Color  => Color::srgb(1.0, 0.5, 0.2),
        TerminalKind::Symbol => Color::srgb(0.8, 0.3, 1.0),
        TerminalKind::Freq   => Color::srgb(0.2, 1.0, 0.4),
    }
}

// ── Terminal proximity ────────────────────────────────────────────────────────

fn terminal_proximity(
    mut commands: Commands,
    player_q: Query<&Transform, With<Player>>,
    color_q: Query<(Entity, &Transform, &ColorTerminal)>,
    symbol_q: Query<(Entity, &Transform, &SymbolTerminal)>,
    freq_q: Query<(Entity, &Transform, &FreqMaster)>,
    prompt_q: Query<Entity, With<TerminalPrompt>>,
    session: Option<Res<TerminalSession>>,
    code_session: Option<Res<CodeEntryState>>,
    signals: Res<PlanetSignals>,
    input: Res<ButtonInput<KeyCode>>,
    bindings: Res<crate::settings::KeyBindings>,
    asset_server: Res<AssetServer>,
) {
    if session.is_some() || code_session.is_some() { return; }

    let Ok(player_tf) = player_q.single() else { return };
    let pp = player_tf.translation;
    let interact_half = Vec2::splat(TILE_SIZE * 2.5);
    let term_half = Vec2::splat(TILE_SIZE * 0.5);

    // Collect all terminal candidates: (entity, pos, kind, locked)
    let mut near: Option<(Entity, Vec3, TerminalKind)> = None;

    for (entity, tf, t) in &color_q {
        if t.unlocked { continue; }
        if aabb_overlap(pp.x, pp.y, interact_half, tf.translation.x, tf.translation.y, term_half) {
            near = Some((entity, tf.translation, TerminalKind::Color));
            break;
        }
    }
    if near.is_none() {
        for (entity, tf, t) in &symbol_q {
            if t.unlocked { continue; }
            if aabb_overlap(pp.x, pp.y, interact_half, tf.translation.x, tf.translation.y, term_half) {
                near = Some((entity, tf.translation, TerminalKind::Symbol));
                break;
            }
        }
    }
    if near.is_none() {
        for (entity, tf, t) in &freq_q {
            if t.unlocked { continue; }
            if aabb_overlap(pp.x, pp.y, interact_half, tf.translation.x, tf.translation.y, term_half) {
                near = Some((entity, tf.translation, TerminalKind::Freq));
                break;
            }
        }
    }

    for e in &prompt_q { commands.entity(e).despawn(); }

    let Some((terminal_entity, terminal_pos, kind)) = near else { return };
    let font: Handle<Font> = asset_server.load(FONT_PATH);

    // Freq master is locked until all 3 signals are revealed.
    let freq_locked = kind == TerminalKind::Freq && signals.signals.iter().any(|s| s.is_none());

    let prompt_text = if freq_locked {
        "[LOCKED] Need all 3 signals".to_string()
    } else {
        "[E] Interact".to_string()
    };
    let prompt_color = if freq_locked {
        Color::srgb(0.8, 0.2, 0.2)
    } else {
        terminal_accent(kind)
    };

    commands.spawn((
        Text2d::new(prompt_text),
        TextFont { font, font_size: 18.0, ..default() },
        TextColor(prompt_color),
        Transform::from_translation(terminal_pos + Vec3::new(0.0, TILE_SIZE * 1.5, 10.0)),
        TerminalPrompt,
        GameEntity,
    ));

    if !freq_locked && input.just_pressed(bindings.interact) {
        commands.insert_resource(TerminalSession {
            terminal_entity,
            kind,
            entered: [0; 3],
            cursor: 0,
            wrong_timer: None,
        });
        spawn_terminal_ui(&mut commands, &asset_server, kind);
    }
}

fn spawn_terminal_ui(commands: &mut Commands, asset_server: &AssetServer, kind: TerminalKind) {
    let font: Handle<Font> = asset_server.load(FONT_PATH);
    let slot_font: Handle<Font> = match kind {
        TerminalKind::Symbol | TerminalKind::Freq => asset_server.load(SYMBOL_FONT_PATH),
        _ => font.clone(),
    };
    let accent = terminal_accent(kind);

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(360.0),
                height: Val::Auto,
                left: Val::Percent(50.0),
                top: Val::Percent(40.0),
                margin: UiRect { left: Val::Px(-180.0), ..default() },
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(12.0),
                padding: UiRect::all(Val::Px(20.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.03, 0.08, 0.96)),
            BorderColor(accent),
            BorderRadius::all(Val::Px(8.0)),
            ZIndex(30),
            TerminalUi,
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new(terminal_title(kind)),
                TextFont { font: font.clone(), font_size: 22.0, ..default() },
                TextColor(accent),
            ));

            panel
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(12.0),
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|row| {
                    for i in 0..3usize {
                        row.spawn((
                            Text::new(format!("> {} <", terminal_display(kind, 0))),
                            TextFont { font: slot_font.clone(), font_size: 24.0, ..default() },
                            TextColor(if i == 0 { Color::WHITE } else { Color::srgb(0.5, 0.5, 0.5) }),
                            TerminalSlot(i),
                        ));
                    }
                });

            panel.spawn((
                Text::new("W/S change  A/D move  Enter=submit  E=close"),
                TextFont { font: font.clone(), font_size: 14.0, ..default() },
                TextColor(Color::srgb(0.6, 0.6, 0.6)),
                TerminalStatusText,
            ));
        });
}

// ── Terminal keypad update ────────────────────────────────────────────────────

fn update_terminal_ui(
    mut commands: Commands,
    session: Option<ResMut<TerminalSession>>,
    colors: Res<StationColors>,
    symbols: Res<StationSymbols>,
    mut signals: ResMut<PlanetSignals>,
    input: Res<ButtonInput<KeyCode>>,
    bindings: Res<crate::settings::KeyBindings>,
    time: Res<Time>,
    mut slot_q: Query<(&TerminalSlot, &mut Text, &mut TextColor)>,
    mut status_q: Query<(&mut Text, &mut TextColor), (With<TerminalStatusText>, Without<TerminalSlot>)>,
    ui_q: Query<Entity, With<TerminalUi>>,
    mut color_q: Query<&mut Sprite, (With<ColorTerminal>, Without<SymbolTerminal>, Without<FreqMaster>)>,
    mut symbol_q: Query<&mut Sprite, (With<SymbolTerminal>, Without<ColorTerminal>, Without<FreqMaster>)>,
    mut freq_q: Query<(Entity, &mut Sprite), (With<FreqMaster>, Without<ColorTerminal>, Without<SymbolTerminal>)>,
    asset_server: Res<AssetServer>,
) {
    let Some(mut state) = session else { return };

    if let Some(ref mut timer) = state.wrong_timer {
        timer.tick(time.delta());
        if timer.just_finished() {
            state.wrong_timer = None;
            if let Ok((mut txt, mut col)) = status_q.single_mut() {
                *txt = Text::new("W/S change  A/D move  Enter=submit  E=close");
                *col = TextColor(Color::srgb(0.6, 0.6, 0.6));
            }
        }
        return;
    }

    if input.just_pressed(bindings.interact) {
        close_terminal(&mut commands, &ui_q);
        return;
    }

    let kind = state.kind;
    let max_val = terminal_max(kind);

    if input.just_pressed(bindings.move_left) && state.cursor > 0 {
        state.cursor -= 1;
    }
    if input.just_pressed(bindings.move_right) && state.cursor < 2 {
        state.cursor += 1;
    }
    if input.just_pressed(bindings.move_up) {
        let idx = state.cursor;
        state.entered[idx] = (state.entered[idx] + 1) % (max_val + 1);
    }
    if input.just_pressed(bindings.move_down) {
        let idx = state.cursor;
        state.entered[idx] = (state.entered[idx] + max_val) % (max_val + 1);
    }

    for (slot, mut txt, mut col) in &mut slot_q {
        let i = slot.0;
        let val = state.entered[i];
        if i == state.cursor {
            *txt = Text::new(format!("> {} <", terminal_display(kind, val)));
            *col = TextColor(Color::WHITE);
        } else {
            *txt = Text::new(format!("  {}  ", terminal_display(kind, val)));
            *col = TextColor(Color::srgb(0.5, 0.5, 0.5));
        }
    }

    if input.just_pressed(KeyCode::Enter) {
        let correct = match kind {
            TerminalKind::Color  => colors.colors.iter().zip(state.entered.iter())
                .all(|(s, e)| s.map_or(false, |v| v == *e)),
            TerminalKind::Symbol => symbols.symbols.iter().zip(state.entered.iter())
                .all(|(s, e)| s.map_or(false, |v| v == *e)),
            TerminalKind::Freq   => signals.signals.iter().zip(state.entered.iter())
                .all(|(s, e)| s.map_or(false, |v| v == *e + 1)), // entered 0-4, signals 1-5
        };

        if correct {
            let terminal_entity = state.terminal_entity;
            let font: Handle<Font> = asset_server.load(FONT_PATH);

            match kind {
                TerminalKind::Color => {
                    let sig = random_range(1u8..=5u8);
                    signals.signals[1] = Some(sig);
                    if let Ok(mut sprite) = color_q.get_mut(terminal_entity) {
                        sprite.color = Color::srgb(0.3, 0.3, 0.3);
                    }
                    commands.entity(terminal_entity).remove::<Collidable>();
                    commands.entity(terminal_entity).remove::<Collider>();
                    if let Ok(mut e) = commands.get_entity(terminal_entity) {
                        e.insert(ColorTerminal { unlocked: true });
                    }
                    commands.spawn((
                        Text2d::new(format!("Signal Strength B: {}", sig)),
                        TextFont { font, font_size: 20.0, ..default() },
                        TextColor(Color::srgb(1.0, 0.5, 0.2)),
                        Transform::from_translation(Vec3::new(0.0, 60.0, 100.0)),
                        crate::rewards::RewardPopup { timer: Timer::from_seconds(3.0, TimerMode::Once) },
                        GameEntity,
                    ));
                }
                TerminalKind::Symbol => {
                    let sig = random_range(1u8..=5u8);
                    signals.signals[2] = Some(sig);
                    if let Ok(mut sprite) = symbol_q.get_mut(terminal_entity) {
                        sprite.color = Color::srgb(0.3, 0.3, 0.3);
                    }
                    commands.entity(terminal_entity).remove::<Collidable>();
                    commands.entity(terminal_entity).remove::<Collider>();
                    if let Ok(mut e) = commands.get_entity(terminal_entity) {
                        e.insert(SymbolTerminal { unlocked: true });
                    }
                    commands.spawn((
                        Text2d::new(format!("Signal Strength C: {}", sig)),
                        TextFont { font, font_size: 20.0, ..default() },
                        TextColor(Color::srgb(0.8, 0.3, 1.0)),
                        Transform::from_translation(Vec3::new(0.0, 60.0, 100.0)),
                        crate::rewards::RewardPopup { timer: Timer::from_seconds(3.0, TimerMode::Once) },
                        GameEntity,
                    ));
                }
                TerminalKind::Freq => {
                    // Unlock every FreqMaster tile so the full widened doorway opens.
                    for (entity, mut sprite) in freq_q.iter_mut() {
                        sprite.color = Color::srgb(0.3, 0.3, 0.3);
                        commands.entity(entity).remove::<Collidable>();
                        commands.entity(entity).remove::<Collider>();
                        commands.entity(entity).insert(FreqMaster { unlocked: true });
                    }
                    commands.spawn((
                        Text2d::new("Boss Arena Unlocked!"),
                        TextFont { font, font_size: 24.0, ..default() },
                        TextColor(Color::srgb(0.2, 1.0, 0.4)),
                        Transform::from_translation(Vec3::new(0.0, 60.0, 100.0)),
                        crate::rewards::RewardPopup { timer: Timer::from_seconds(3.0, TimerMode::Once) },
                        GameEntity,
                    ));
                }
            }

            close_terminal(&mut commands, &ui_q);
        } else {
            if let Ok((mut txt, mut col)) = status_q.single_mut() {
                *txt = Text::new("✗  INCORRECT  ✗");
                *col = TextColor(Color::srgb(1.0, 0.2, 0.2));
            }
            state.wrong_timer = Some(Timer::from_seconds(1.5, TimerMode::Once));
        }
    }
}

fn close_terminal(commands: &mut Commands, ui_q: &Query<Entity, With<TerminalUi>>) {
    for e in ui_q.iter() {
        commands.entity(e).despawn();
    }
    commands.remove_resource::<TerminalSession>();
}
