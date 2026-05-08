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
use crate::station_code::StationCodes;
use crate::rewards::RewardRes;
use crate::{
    EndScreenButtons, GameEntity, GameState, PlanetCount, PlanetLevelMarker,
    StationLevel, FONT_PATH, TILE_SIZE, Z_ENTITIES,
};
use crate::player::{Player, aabb_overlap};

// ── Components ───────────────────────────────────────────────────────────────

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

/// Active code-entry session.
#[derive(Resource)]
struct CodeEntryState {
    door_entity: Entity,
    entered: [u8; 3],
    cursor: usize,
    wrong_timer: Option<Timer>,
}

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
            // On planet Playing: green surface tint + spawn boss + health bar + vault rewards
            .add_systems(
                OnEnter(GameState::Playing),
                (tint_planet_background, spawn_final_boss, spawn_boss_health_bar, spawn_vault_rewards)
                    .run_if(resource_exists::<PlanetLevelMarker>),
            )
            // Detect boss death → trigger PlanetWin
            .add_systems(
                Update,
                check_planet_win
                    .run_if(in_state(GameState::Playing))
                    .run_if(resource_exists::<PlanetLevelMarker>),
            )
            // Update boss health bar fill width each frame
            .add_systems(
                Update,
                update_boss_health_bar
                    .run_if(in_state(GameState::Playing))
                    .run_if(resource_exists::<PlanetLevelMarker>),
            )
            // Code door systems — proximity prompt and keypad interaction
            .add_systems(
                Update,
                (code_door_proximity, update_code_entry_ui)
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

// 80 cols × 60 rows.
// Layout (top to bottom):
//   Boss arena    : rows  1-16, cols  8-71
//   Corridor      : rows 17-19, cols 38-41
//   Combat Room 2 : rows 20-28, cols 23-56
//   Corridor      : rows 29-31, cols 38-41
//   Hub/CR1 + vaults: rows 32-42, cols 3-76
//     Left vault  : cols  3-20 (code door at col 20, row 37)
//     Hub/CR1     : cols 20-59
//     Right vault : cols 59-76 (code door at col 59, row 37)
//   Corridor      : rows 43-46, cols 38-41
//   Entry room    : rows 47-56, cols 28-51  (player spawns row 52 col 39)
const PLANET_MAP_FILE: &str = "assets/planet/planet_level.txt";

// Pre-computed world-space room bounds for an 80×60 map (TILE_SIZE = 32).
//   x0 = -(80*32)/2 + 16 = -1264
//   y0 = -(60*32)/2 + 16 = -944
//   world_x(col) = -1264 + col * 32
//   world_y(row) = -944 + (59 - row) * 32

const ARENA_TLC:  Vec2 = Vec2::new(-1008.0, 912.0);  // col=8,  row=1
const ARENA_BRC:  Vec2 = Vec2::new(1008.0,  432.0);  // col=71, row=16
const CR2_TLC:    Vec2 = Vec2::new(-528.0,  304.0);  // col=23, row=20
const CR2_BRC:    Vec2 = Vec2::new(528.0,    48.0);  // col=56, row=28
const HUB_TLC:    Vec2 = Vec2::new(-624.0,  -80.0);  // col=20, row=32
const HUB_BRC:    Vec2 = Vec2::new(624.0,  -400.0);  // col=59, row=42
const ENTRY_TLC:  Vec2 = Vec2::new(-368.0, -560.0);  // col=28, row=47
const ENTRY_BRC:  Vec2 = Vec2::new(368.0,  -848.0);  // col=51, row=56

// Tile-grid coordinates for each room.
const ARENA_TILE_TLC:  Vec2 = Vec2::new(8.0,  1.0);
const ARENA_TILE_BRC:  Vec2 = Vec2::new(71.0, 16.0);
const CR2_TILE_TLC:    Vec2 = Vec2::new(23.0, 20.0);
const CR2_TILE_BRC:    Vec2 = Vec2::new(56.0, 28.0);
const HUB_TILE_TLC:    Vec2 = Vec2::new(20.0, 32.0);
const HUB_TILE_BRC:    Vec2 = Vec2::new(59.0, 42.0);
const ENTRY_TILE_TLC:  Vec2 = Vec2::new(28.0, 47.0);
const ENTRY_TILE_BRC:  Vec2 = Vec2::new(51.0, 56.0);

// Boss spawns near the centre of the arena (row 8, col 39).
const BOSS_SPAWN: Vec3 = Vec3::new(-16.0, 688.0, Z_ENTITIES);

// Pre-baked world positions for the 3 reward boxes inside each vault.
const LEFT_VAULT_REWARDS:  [Vec3; 3] = [
    Vec3::new(-976.0, -240.0, Z_ENTITIES),
    Vec3::new(-912.0, -240.0, Z_ENTITIES),
    Vec3::new(-848.0, -240.0, Z_ENTITIES),
];
const RIGHT_VAULT_REWARDS: [Vec3; 3] = [
    Vec3::new(816.0, -240.0, Z_ENTITIES),
    Vec3::new(880.0, -240.0, Z_ENTITIES),
    Vec3::new(944.0, -240.0, Z_ENTITIES),
];

// ── Setup planet level ────────────────────────────────────────────────────────

fn setup_planet_level(mut commands: Commands) {
    use std::io::{BufRead, BufReader};
    let file = match std::fs::File::open(PLANET_MAP_FILE) {
        Ok(f) => f,
        Err(e) => {
            warn!("Could not open planet level file '{}': {e}", PLANET_MAP_FILE);
            return;
        }
    };
    let rows: Vec<String> = BufReader::new(file)
        .lines()
        .filter_map(|l| l.ok())
        .collect();
    commands.insert_resource(GeneratedLevel(rows));

    let empty: Vec<String> = vec!["............".to_string(); 4];
    // Layouts with '#' tiles allow the room system to spawn enemies.
    let cr2_layout: Vec<String> = vec!["################################".to_string(); 7];
    let hub_layout: Vec<String> = vec!["######################################".to_string(); 9];

    let mut rv = RoomVec(Vec::new());

    // Boss arena — empty layout so no regular enemies spawn; FinalBoss is pre-spawned.
    rv.0.push(Room::new(ARENA_TLC, ARENA_BRC, ARENA_TILE_TLC, ARENA_TILE_BRC, empty.clone()));

    // Combat Room 2 — enemy room blocking access to the boss arena.
    rv.0.push(Room::new(CR2_TLC, CR2_BRC, CR2_TILE_TLC, CR2_TILE_BRC, cr2_layout));

    // Hub / Combat Room 1 — enemy room; also has the two code-door vaults on its sides.
    rv.0.push(Room::new(HUB_TLC, HUB_BRC, HUB_TILE_TLC, HUB_TILE_BRC, hub_layout));

    // Entry corridor — pre-cleared so the player can move freely from spawn.
    let mut entry = Room::new(ENTRY_TLC, ENTRY_BRC, ENTRY_TILE_TLC, ENTRY_TILE_BRC, empty);
    entry.cleared = true;
    entry.visited = true;
    rv.0.push(entry);

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
    bar_q: Query<Entity, With<BossHealthBarRoot>>,
) {
    clear_color.0 = Color::srgb(0.02, 0.02, 0.06);
    commands.remove_resource::<PlanetLevelMarker>();
    for e in &bar_q {
        commands.entity(e).despawn();
    }
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

// ── Boss health bar ───────────────────────────────────────────────────────────

fn spawn_boss_health_bar(mut commands: Commands, asset_server: Res<AssetServer>) {
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

            // Track background
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

// ── Vault rewards ─────────────────────────────────────────────────────────────

/// Pre-spawn 3 reward boxes in each vault room at level entry.
/// The code doors prevent access until the correct code is entered.
fn spawn_vault_rewards(mut commands: Commands, reward_res: Res<RewardRes>) {
    for &pos in LEFT_VAULT_REWARDS.iter().chain(RIGHT_VAULT_REWARDS.iter()) {
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

// ── Code door systems ─────────────────────────────────────────────────────────

/// Show "[E] Enter Code" prompt when the player is near a locked CodeDoor,
/// and open the keypad when they press the interact key.
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
    // Don't change proximity prompts while the keypad is open.
    if entry_state.is_some() { return; }

    let Ok(player_tf) = player_q.single() else { return };
    let pp = player_tf.translation;
    let interact_half = Vec2::splat(TILE_SIZE * 2.5);
    let door_half = Vec2::splat(TILE_SIZE * 0.5);

    // Find the nearest unlocked door in range.
    let mut near_door: Option<(Entity, Vec3)> = None;
    for (entity, door_tf, door) in &door_q {
        if door.unlocked { continue; }
        let dp = door_tf.translation;
        if aabb_overlap(pp.x, pp.y, interact_half, dp.x, dp.y, door_half) {
            near_door = Some((entity, dp));
            break;
        }
    }

    // Despawn stale prompts.
    for e in &prompt_q {
        commands.entity(e).despawn();
    }

    if let Some((door_entity, door_pos)) = near_door {
        // Spawn floating prompt above the door.
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
            // Open the keypad UI.
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
            // Title
            panel.spawn((
                Text::new("ENTER CODE"),
                TextFont { font: font.clone(), font_size: 22.0, ..default() },
                TextColor(Color::srgb(0.2, 1.0, 1.0)),
            ));

            // Digit slots row
            panel
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(12.0),
                        align_items: AlignItems::Center,
                        ..default()
                    },
                ))
                .with_children(|row| {
                    for i in 0..3usize {
                        row.spawn((
                            Text::new("▶ 0 ◀"),
                            TextFont { font: font.clone(), font_size: 28.0, ..default() },
                            TextColor(if i == 0 { Color::WHITE } else { Color::srgb(0.5, 0.5, 0.5) }),
                            CodeDigitSlot(i),
                        ));
                    }
                });

            // Status line
            panel.spawn((
                Text::new("↑↓ change  ←→ move  E=submit  Esc=cancel"),
                TextFont { font: font.clone(), font_size: 14.0, ..default() },
                TextColor(Color::srgb(0.6, 0.6, 0.6)),
                CodeStatusText,
            ));
        });
}

/// Handle keypad input while the code entry UI is active.
fn update_code_entry_ui(
    mut commands: Commands,
    entry_state: Option<ResMut<CodeEntryState>>,
    codes: Res<StationCodes>,
    input: Res<ButtonInput<KeyCode>>,
    bindings: Res<crate::settings::KeyBindings>,
    time: Res<Time>,
    mut digit_q: Query<(&CodeDigitSlot, &mut Text, &mut TextColor)>,
    mut status_q: Query<(&mut Text, &mut TextColor), (With<CodeStatusText>, Without<CodeDigitSlot>)>,
    ui_q: Query<Entity, With<CodeEntryUi>>,
    mut door_q: Query<&mut Sprite, With<CodeDoor>>,
    asset_server: Res<AssetServer>,
) {
    let Some(mut state) = entry_state else { return };

    // Tick the "wrong code" flash timer.
    if let Some(ref mut timer) = state.wrong_timer {
        timer.tick(time.delta());
        if timer.just_finished() {
            state.wrong_timer = None;
            // Restore hint text.
            if let Ok((mut txt, mut col)) = status_q.single_mut() {
                *txt = Text::new("↑↓ change  ←→ move  E=submit  Esc=cancel");
                *col = TextColor(Color::srgb(0.6, 0.6, 0.6));
            }
        }
        return; // Ignore input while flashing.
    }

    // ── Input handling ────────────────────────────────────────────────────────

    if input.just_pressed(KeyCode::Escape) {
        close_keypad(&mut commands, &ui_q);
        return;
    }

    let cursor_before = state.cursor;

    if input.just_pressed(KeyCode::ArrowLeft) && state.cursor > 0 {
        state.cursor -= 1;
    }
    if input.just_pressed(KeyCode::ArrowRight) && state.cursor < 2 {
        state.cursor += 1;
    }
    if input.just_pressed(KeyCode::ArrowUp) {
        let idx = state.cursor;
        state.entered[idx] = (state.entered[idx] + 1) % 10;
    }
    if input.just_pressed(KeyCode::ArrowDown) {
        let idx = state.cursor;
        state.entered[idx] = (state.entered[idx] + 9) % 10;
    }

    // Update digit display if cursor or digit changed.
    let cursor_changed = cursor_before != state.cursor;
    for (slot, mut txt, mut col) in &mut digit_q {
        let i = slot.0;
        let d = state.entered[i];
        if i == state.cursor {
            *txt = Text::new(format!("▶ {} ◀", d));
            *col = TextColor(Color::WHITE);
        } else {
            *txt = Text::new(format!("  {}  ", d));
            *col = TextColor(Color::srgb(0.5, 0.5, 0.5));
        }
        let _ = cursor_changed; // suppress unused warning
    }

    // ── Submit ────────────────────────────────────────────────────────────────

    if input.just_pressed(bindings.interact) {
        let correct = codes.codes.iter().zip(state.entered.iter()).all(|(stored, entered)| {
            stored.map_or(false, |d| d == *entered)
        });

        if correct {
            // Unlock the door.
            let door_entity = state.door_entity;
            commands.entity(door_entity).remove::<Collidable>();
            commands.entity(door_entity).remove::<Collider>();
            if let Ok(mut door) = commands.get_entity(door_entity) {
                door.insert(CodeDoor { unlocked: true });
            }

            // Swap to open door sprite.
            let open_door: Handle<Image> = asset_server.load("map/open_door.png");
            if let Ok(mut sprite) = door_q.get_mut(door_entity) {
                sprite.image = open_door;
            }

            close_keypad(&mut commands, &ui_q);
        } else {
            // Flash "INCORRECT CODE".
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
        commands.entity(e).despawn_recursive();
    }
    commands.remove_resource::<CodeEntryState>();
}
