use bevy::prelude::*;
use super::{
    FinalBoss, BossArenaState, BossExitDoor,
    CodeDoor, CodeEntryState, CodeDoorPrompt, CodeEntryUi, CodeDigitSlot, CodeStatusText,
    ColorTerminal, SymbolTerminal, FreqMaster, PlanetSignals, TerminalSession, TerminalKind,
    TerminalPrompt, TerminalUi, TerminalSlot, TerminalStatusText,
    DialTargets, MiniBossArenaState,
};
use crate::{
    GameEntity, PlanetCount,
    StationLevel, TestPlanetMode,
    FONT_PATH, SYMBOL_FONT_PATH, TILE_SIZE, Z_ENTITIES,
};
use crate::collidable::{Collidable, Collider};
use crate::enemies::{
    ActiveEnemy, AnimationTimer, Enemy, EnemyFrames, EnemyMoveSpeed, EnemyRes,
    HitAnimation, MeleeEnemy, Velocity, ENEMY_SPEED,
};
use crate::map::{Door, GeneratedLevel, TileRes};
use crate::player::{Player, aabb_overlap};
use crate::room::{Room, RoomVec};
use crate::station_code::StationCodes;
use crate::station_color::StationColors;
use crate::station_symbol::{StationSymbols, SYMBOL_CHARS};
use crate::settings::KeyBindings;
use rand::random_range;

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
pub(super) const P1_ARENA_TLC: Vec2 = Vec2::new(-4720.0, 3056.0);
pub(super) const P1_ARENA_BRC: Vec2 = Vec2::new(-2896.0, 1552.0);

// E Room — Top Right
pub(super) const P1_EROOM_TR_TLC:      Vec2 = Vec2::new(1584.0, 2992.0);
pub(super) const P1_EROOM_TR_BRC:      Vec2 = Vec2::new(3184.0, 1552.0);
pub(super) const P1_EROOM_TR_TILE_TLC: Vec2 = Vec2::new(201.0,  6.0);
pub(super) const P1_EROOM_TR_TILE_BRC: Vec2 = Vec2::new(249.0, 49.0);

// E Room — Top Center
pub(super) const P1_EROOM_TC_TLC:      Vec2 = Vec2::new(-2320.0, 2512.0);
pub(super) const P1_EROOM_TC_BRC:      Vec2 = Vec2::new(-16.0,   1840.0);
pub(super) const P1_EROOM_TC_TILE_TLC: Vec2 = Vec2::new(79.0,  21.0);
pub(super) const P1_EROOM_TC_TILE_BRC: Vec2 = Vec2::new(149.0, 40.0);

// E Room — Mid Right 1
pub(super) const P1_EROOM_MR1_TLC:      Vec2 = Vec2::new(1904.0,  80.0);
pub(super) const P1_EROOM_MR1_BRC:      Vec2 = Vec2::new(2992.0, -656.0);
pub(super) const P1_EROOM_MR1_TILE_TLC: Vec2 = Vec2::new(211.0,  99.0);
pub(super) const P1_EROOM_MR1_TILE_BRC: Vec2 = Vec2::new(243.0, 118.0);

// E Room — Mid Left
pub(super) const P1_EROOM_ML_TLC:      Vec2 = Vec2::new(-2768.0,  -848.0);
pub(super) const P1_EROOM_ML_BRC:      Vec2 = Vec2::new(-1680.0, -1520.0);
pub(super) const P1_EROOM_ML_TILE_TLC: Vec2 = Vec2::new(65.0,  128.0);
pub(super) const P1_EROOM_ML_TILE_BRC: Vec2 = Vec2::new(97.0,  147.0);

// E Room — Mid Right 2
pub(super) const P1_EROOM_MR2_TLC:      Vec2 = Vec2::new(1968.0, -1200.0);
pub(super) const P1_EROOM_MR2_BRC:      Vec2 = Vec2::new(2992.0, -2000.0);
pub(super) const P1_EROOM_MR2_TILE_TLC: Vec2 = Vec2::new(211.0, 139.0);
pub(super) const P1_EROOM_MR2_TILE_BRC: Vec2 = Vec2::new(243.0, 160.0);

// E Room — Bottom Left
pub(super) const P1_EROOM_BL_TLC:      Vec2 = Vec2::new(-2992.0, -2160.0);
pub(super) const P1_EROOM_BL_BRC:      Vec2 = Vec2::new(-1968.0, -2768.0);
pub(super) const P1_EROOM_BL_TILE_TLC: Vec2 = Vec2::new(56.0,  167.0);
pub(super) const P1_EROOM_BL_TILE_BRC: Vec2 = Vec2::new(88.0,  186.0);

// E Room — Bottom Center
pub(super) const P1_EROOM_BC_TLC:      Vec2 = Vec2::new(-1200.0, -2096.0);
pub(super) const P1_EROOM_BC_BRC:      Vec2 = Vec2::new(-112.0,  -2768.0);
pub(super) const P1_EROOM_BC_TILE_TLC: Vec2 = Vec2::new(114.0, 167.0);
pub(super) const P1_EROOM_BC_TILE_BRC: Vec2 = Vec2::new(146.0, 186.0);

// Spawn Room
pub(super) const P1_SPAWN_TLC:      Vec2 = Vec2::new(1968.0, -2448.0);
pub(super) const P1_SPAWN_BRC:      Vec2 = Vec2::new(2992.0, -2800.0);
pub(super) const P1_SPAWN_TILE_TLC: Vec2 = Vec2::new(211.0, 178.0);
pub(super) const P1_SPAWN_TILE_BRC: Vec2 = Vec2::new(243.0, 187.0);

// Boss spawns near the centre of the arena (col 30, row 20).
pub(super) const P1_BOSS_SPAWN: Vec3 = Vec3::new(-3824.0, 2544.0, Z_ENTITIES);

// Chest spawns 4 tiles below the boss spawn after the boss is defeated.
pub(super) const BOSS_CHEST_POS: Vec3 = Vec3::new(-3824.0, 2416.0, Z_ENTITIES);

// Exit corridor — 3×3 gap (rows 52-54, cols 29-31). Door centred at row 53, col 30.
pub(super) const BOSS_EXIT_DOOR_POS: Vec3 = Vec3::new(-3824.0, 1488.0, Z_ENTITIES);

// "Leave Planet" beacon — centre of the exit room (row 60, col 30).
pub(super) const PLANET_EXIT_BEACON_POS: Vec3 = Vec3::new(-3824.0, 1264.0, Z_ENTITIES);

// Exit room (rows 54-66, cols 23-37).
pub(super) const P1_EXIT_TLC:      Vec2 = Vec2::new(-4048.0, 1456.0);
pub(super) const P1_EXIT_BRC:      Vec2 = Vec2::new(-3600.0, 1072.0);
pub(super) const P1_EXIT_TILE_TLC: Vec2 = Vec2::new(23.0, 54.0);
pub(super) const P1_EXIT_TILE_BRC: Vec2 = Vec2::new(37.0, 66.0);

// Vault reward positions (cols 5-23, rows 133-142); code door at col 23.
pub(super) static P1_VAULT_REWARDS: [Vec3; 3] = [
    Vec3::new(-4528.0, -1200.0, Z_ENTITIES),  // col=8,  row=137
    Vec3::new(-4400.0, -1200.0, Z_ENTITIES),  // col=12, row=137
    Vec3::new(-4272.0, -1200.0, Z_ENTITIES),  // col=16, row=137
];

// ── Planet 3 mini-boss arena constants ───────────────────────────────────────
// Planet 3 reuses the Top-Right enemy room as the mini-boss arena.
pub(super) const P3_MINI_ARENA_TLC: Vec2 = P1_EROOM_TR_TLC;
pub(super) const P3_MINI_ARENA_BRC: Vec2 = P1_EROOM_TR_BRC;

// ── Room builder helpers ──────────────────────────────────────────────────────

pub(super) fn make_enemy_layout(width: usize, height: usize) -> Vec<String> {
    let row = "#".repeat(width);
    vec![row; height]
}

pub(super) fn make_empty_layout() -> Vec<String> {
    vec!["............".to_string(); 4]
}

pub(super) fn planet_enemy_room(tlc: Vec2, brc: Vec2, tile_tlc: Vec2, tile_brc: Vec2, w: usize, h: usize) -> Room {
    let mut r = Room::new(tlc, brc, tile_tlc, tile_brc, make_enemy_layout(w, h));
    r.base_enemies = 6;
    r.health_mult = 2.5;
    r
}

// ── Planet 1 room builder ─────────────────────────────────────────────────────

pub(super) fn build_planet1_rooms() -> RoomVec {
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

pub(super) fn setup_planet_level(
    mut commands: Commands,
    mut room_vec: ResMut<RoomVec>,
    planet_count: Res<PlanetCount>,
) {
    use std::io::{BufRead, BufReader};
    let planet_idx = planet_count.0 as usize;
    let map_path = super::planet_map_file(planet_idx);

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
    *room_vec = super::build_planet_rooms(planet_idx);
}

// ── Boss arena state ──────────────────────────────────────────────────────────

pub(super) fn init_boss_arena_state(mut commands: Commands) {
    commands.insert_resource(BossArenaState::Idle);
}

/// Spawns a collidable wall entity sealing the exit corridor south of the boss arena.
pub(super) fn spawn_boss_exit_door(mut commands: Commands, tiles: Res<TileRes>) {
    commands.spawn((
        Sprite::from_image(tiles.closed_door.clone()),
        Transform::from_translation(BOSS_EXIT_DOOR_POS),
        Collidable,
        Collider { half_extents: Vec2::new(TILE_SIZE * 1.5, TILE_SIZE * 1.5) },
        BossExitDoor,
        crate::GameEntity,
    ));
}

pub(super) fn boss_arena_trigger(
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

    let hp = 1500.0 + station_level.0 as f32 * 500.0;
    let boss_pos = super::planet_boss_spawn(planet_count.0 as usize);
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

    super::shared::do_spawn_boss_health_bar(&mut commands, &asset_server);

    commands.insert_resource(BossArenaState::Active);
}

// ── Planet resources init ─────────────────────────────────────────────────────

pub(super) fn init_planet_resources(mut commands: Commands) {
    commands.insert_resource(PlanetSignals::default());
    commands.insert_resource(DialTargets::default());
    commands.insert_resource(MiniBossArenaState::Idle);
}

// ── Test-mode clue injection ─────────────────────────────────────────────────

pub(super) fn inject_test_planet_clues(
    mut codes: ResMut<StationCodes>,
    mut colors: ResMut<StationColors>,
    mut symbols: ResMut<StationSymbols>,
    test_mode: Option<Res<TestPlanetMode>>,
) {
    if test_mode.is_none() && symbols.symbols.iter().any(|s| s.is_some()) { return; }
    codes.codes     = [Some(1), Some(2), Some(3)];
    colors.colors   = [Some(0), Some(1), Some(2)];
    symbols.symbols = [Some(0), Some(1), Some(2)];
}

// ── Code door systems ─────────────────────────────────────────────────────────

pub(super) fn code_door_proximity(
    mut commands: Commands,
    player_q: Query<&Transform, With<Player>>,
    door_q: Query<(Entity, &Transform, &CodeDoor)>,
    prompt_q: Query<Entity, With<CodeDoorPrompt>>,
    entry_state: Option<Res<CodeEntryState>>,
    input: Res<ButtonInput<KeyCode>>,
    bindings: Res<KeyBindings>,
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

pub(super) fn update_code_entry_ui(
    mut commands: Commands,
    entry_state: Option<ResMut<CodeEntryState>>,
    codes: Res<StationCodes>,
    mut signals: ResMut<PlanetSignals>,
    mut dial_targets: ResMut<DialTargets>,
    planet_count: Res<PlanetCount>,
    input: Res<ButtonInput<KeyCode>>,
    bindings: Res<KeyBindings>,
    time: Res<Time>,
    player_q: Query<&Transform, With<Player>>,
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
            for (door_entity, _) in door_q.iter() {
                commands.entity(door_entity).remove::<Collidable>();
                commands.entity(door_entity).remove::<crate::collidable::Collider>();
                if let Ok(mut e) = commands.get_entity(door_entity) {
                    e.insert(CodeDoor { unlocked: true });
                }
            }
            for (_, mut sprite) in door_q.iter_mut() {
                sprite.image = open_door.clone();
            }

            let font: Handle<Font> = asset_server.load(FONT_PATH);
            let popup_pos = player_q.single()
                .map(|tf| tf.translation + Vec3::new(0.0, TILE_SIZE * 2.0, 100.0))
                .unwrap_or(Vec3::new(0.0, 60.0, 100.0));
            if planet_count.0 == 0 {
                let sig_a: u8 = random_range(1u8..=5u8);
                signals.signals[0] = Some(sig_a);
                commands.spawn((
                    Text2d::new(format!("Signal Strength A: {}", sig_a)),
                    TextFont { font, font_size: 20.0, ..default() },
                    TextColor(Color::srgb(0.2, 1.0, 0.5)),
                    Transform::from_translation(popup_pos),
                    crate::rewards::RewardPopup { timer: Timer::from_seconds(3.0, TimerMode::Once) },
                    GameEntity,
                ));
            } else {
                let target: u8 = random_range(0u8..=9u8);
                dial_targets.targets[0] = Some(target);
                commands.spawn((
                    Text2d::new(format!("Dial A Target: {}", target)),
                    TextFont { font, font_size: 20.0, ..default() },
                    TextColor(Color::srgb(0.9, 0.9, 0.2)),
                    Transform::from_translation(popup_pos),
                    crate::rewards::RewardPopup { timer: Timer::from_seconds(3.0, TimerMode::Once) },
                    GameEntity,
                ));
            }

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

pub(super) fn terminal_proximity(
    mut commands: Commands,
    player_q: Query<&Transform, With<Player>>,
    color_q: Query<(Entity, &Transform, &ColorTerminal)>,
    symbol_q: Query<(Entity, &Transform, &SymbolTerminal)>,
    freq_q: Query<(Entity, &Transform, &FreqMaster)>,
    prompt_q: Query<Entity, With<TerminalPrompt>>,
    session: Option<Res<TerminalSession>>,
    code_session: Option<Res<CodeEntryState>>,
    signals: Res<PlanetSignals>,
    planet_count: Res<PlanetCount>,
    input: Res<ButtonInput<KeyCode>>,
    bindings: Res<KeyBindings>,
    asset_server: Res<AssetServer>,
) {
    if session.is_some() || code_session.is_some() { return; }

    let Ok(player_tf) = player_q.single() else { return };
    let pp = player_tf.translation;
    let interact_half = Vec2::splat(TILE_SIZE * 2.5);
    let term_half = Vec2::splat(TILE_SIZE * 0.5);

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
            planet_idx: planet_count.0,
            font: asset_server.load(FONT_PATH),
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

    let (ui_w, ui_margin) = match kind {
        TerminalKind::Freq => (520.0_f32, -260.0_f32),
        _                  => (360.0_f32, -180.0_f32),
    };

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(ui_w),
                height: Val::Auto,
                left: Val::Percent(50.0),
                top: Val::Percent(40.0),
                margin: UiRect { left: Val::Px(ui_margin), ..default() },
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

pub(super) fn update_terminal_ui(
    mut commands: Commands,
    session: Option<ResMut<TerminalSession>>,
    colors: Res<StationColors>,
    symbols: Res<StationSymbols>,
    mut signals: ResMut<PlanetSignals>,
    mut dial_targets: ResMut<DialTargets>,
    input: Res<ButtonInput<KeyCode>>,
    bindings: Res<KeyBindings>,
    time: Res<Time>,
    player_q: Query<&Transform, With<Player>>,
    mut slot_q: Query<(&TerminalSlot, &mut Text, &mut TextColor)>,
    mut status_q: Query<(&mut Text, &mut TextColor), (With<TerminalStatusText>, Without<TerminalSlot>)>,
    ui_q: Query<Entity, With<TerminalUi>>,
    mut color_q: Query<&mut Sprite, (With<ColorTerminal>, Without<SymbolTerminal>, Without<FreqMaster>)>,
    mut symbol_q: Query<&mut Sprite, (With<SymbolTerminal>, Without<ColorTerminal>, Without<FreqMaster>)>,
    mut freq_q: Query<(Entity, &mut Sprite), (With<FreqMaster>, Without<ColorTerminal>, Without<SymbolTerminal>)>,
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
                .all(|(s, e)| s.map_or(false, |v| v == *e + 1)),
        };

        if correct {
            let terminal_entity = state.terminal_entity;
            let font = state.font.clone();
            let planet_idx = state.planet_idx;
            let popup_pos = player_q.single()
                .map(|tf| tf.translation + Vec3::new(0.0, TILE_SIZE * 2.0, 100.0))
                .unwrap_or(Vec3::new(0.0, 60.0, 100.0));

            match kind {
                TerminalKind::Color => {
                    if let Ok(mut sprite) = color_q.get_mut(terminal_entity) {
                        sprite.color = Color::srgb(0.3, 0.3, 0.3);
                    }
                    commands.entity(terminal_entity).remove::<Collidable>();
                    commands.entity(terminal_entity).remove::<Collider>();
                    if let Ok(mut e) = commands.get_entity(terminal_entity) {
                        e.insert(ColorTerminal { unlocked: true });
                    }
                    if planet_idx == 0 {
                        let sig = random_range(1u8..=5u8);
                        signals.signals[1] = Some(sig);
                        commands.spawn((
                            Text2d::new(format!("Signal Strength B: {}", sig)),
                            TextFont { font, font_size: 20.0, ..default() },
                            TextColor(Color::srgb(1.0, 0.5, 0.2)),
                            Transform::from_translation(popup_pos),
                            crate::rewards::RewardPopup { timer: Timer::from_seconds(3.0, TimerMode::Once) },
                            GameEntity,
                        ));
                    } else {
                        let target = random_range(0u8..=3u8);
                        dial_targets.targets[1] = Some(target);
                        commands.spawn((
                            Text2d::new(format!("Dial B Target: {}", target)),
                            TextFont { font, font_size: 20.0, ..default() },
                            TextColor(Color::srgb(1.0, 0.6, 0.1)),
                            Transform::from_translation(popup_pos),
                            crate::rewards::RewardPopup { timer: Timer::from_seconds(3.0, TimerMode::Once) },
                            GameEntity,
                        ));
                    }
                }
                TerminalKind::Symbol => {
                    if let Ok(mut sprite) = symbol_q.get_mut(terminal_entity) {
                        sprite.color = Color::srgb(0.3, 0.3, 0.3);
                    }
                    commands.entity(terminal_entity).remove::<Collidable>();
                    commands.entity(terminal_entity).remove::<Collider>();
                    if let Ok(mut e) = commands.get_entity(terminal_entity) {
                        e.insert(SymbolTerminal { unlocked: true });
                    }
                    if planet_idx == 0 {
                        let sig = random_range(1u8..=5u8);
                        signals.signals[2] = Some(sig);
                        commands.spawn((
                            Text2d::new(format!("Signal Strength C: {}", sig)),
                            TextFont { font, font_size: 20.0, ..default() },
                            TextColor(Color::srgb(0.8, 0.3, 1.0)),
                            Transform::from_translation(popup_pos),
                            crate::rewards::RewardPopup { timer: Timer::from_seconds(3.0, TimerMode::Once) },
                            GameEntity,
                        ));
                    } else {
                        let target = random_range(0u8..=5u8);
                        dial_targets.targets[2] = Some(target);
                        commands.spawn((
                            Text2d::new(format!("Dial C Target: {}", target)),
                            TextFont { font, font_size: 20.0, ..default() },
                            TextColor(Color::srgb(0.8, 0.3, 1.0)),
                            Transform::from_translation(popup_pos),
                            crate::rewards::RewardPopup { timer: Timer::from_seconds(3.0, TimerMode::Once) },
                            GameEntity,
                        ));
                    }
                }
                TerminalKind::Freq => {
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
                        Transform::from_translation(popup_pos),
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
