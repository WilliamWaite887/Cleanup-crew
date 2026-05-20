use bevy::prelude::*;
use super::{
    DialButton, DialType, DialInteractState, DialTargets, DialUi, DialCurrentText, DialPrompt,
    PlanetBossDoor, PlanetBossDoorPrompt, TerminalSession, CodeEntryState, MiniBossGate,
};
use super::planet1::{
    P1_EROOM_TR_TLC,  P1_EROOM_TR_BRC,  P1_EROOM_TR_TILE_TLC,  P1_EROOM_TR_TILE_BRC,
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
use crate::{GameEntity, FONT_PATH, TILE_SIZE};
use crate::collidable::{Collidable, Collider};
use crate::player::{Player, aabb_overlap};
use crate::room::{Room, RoomVec};
use crate::settings::KeyBindings;

// ── Planet 2 room builder ─────────────────────────────────────────────────────

pub(super) fn build_planet2_rooms() -> RoomVec {
    let mut rv = RoomVec(Vec::new());

    // Same 7 enemy rooms as P1 — different tile chars in the map add the dials.
    rv.0.push(planet_enemy_room(P1_EROOM_TR_TLC,  P1_EROOM_TR_BRC,  P1_EROOM_TR_TILE_TLC,  P1_EROOM_TR_TILE_BRC,  49, 44));
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

// ── Dial proximity ────────────────────────────────────────────────────────────

pub(super) fn dial_proximity(
    mut commands: Commands,
    player_q: Query<&Transform, With<Player>>,
    dial_q: Query<(Entity, &Transform, &DialButton)>,
    prompt_q: Query<Entity, With<DialPrompt>>,
    dial_state: Option<Res<DialInteractState>>,
    session: Option<Res<TerminalSession>>,
    code_session: Option<Res<CodeEntryState>>,
    dial_targets: Res<DialTargets>,
    input: Res<ButtonInput<KeyCode>>,
    bindings: Res<KeyBindings>,
    asset_server: Res<AssetServer>,
) {
    if dial_state.is_some() || session.is_some() || code_session.is_some() { return; }

    let Ok(player_tf) = player_q.single() else { return };
    let pp = player_tf.translation;
    let interact_half = Vec2::splat(TILE_SIZE * 2.5);
    let dial_half = Vec2::splat(TILE_SIZE * 0.5);

    let mut near: Option<(Entity, Vec3, usize, DialType, bool, u8)> = None;
    for (entity, tf, dial) in &dial_q {
        if aabb_overlap(pp.x, pp.y, interact_half, tf.translation.x, tf.translation.y, dial_half) {
            let locked = dial_targets.targets[dial.dial_idx].is_none();
            near = Some((entity, tf.translation, dial.dial_idx, dial.dial_type, locked, dial.current));
            break;
        }
    }

    for e in &prompt_q { commands.entity(e).despawn(); }

    let Some((dial_entity, dial_pos, dial_idx, dial_type, locked, current)) = near else { return };

    let label = ["DIAL A", "DIAL B", "DIAL C"][dial_idx];
    let (prompt_text, prompt_color) = if locked {
        (format!("[{}] LOCKED — solve terminal first", label), Color::srgb(0.8, 0.2, 0.2))
    } else {
        (format!("[E] Set {}", label), Color::srgb(0.2, 1.0, 1.0))
    };

    let font: Handle<Font> = asset_server.load(FONT_PATH);
    commands.spawn((
        Text2d::new(prompt_text),
        TextFont { font, font_size: 18.0, ..default() },
        TextColor(prompt_color),
        Transform::from_translation(dial_pos + Vec3::new(0.0, TILE_SIZE * 1.5, 10.0)),
        DialPrompt,
        GameEntity,
    ));

    if !locked && input.just_pressed(bindings.interact) {
        commands.insert_resource(DialInteractState {
            dial_entity,
            dial_idx,
            dial_type,
            current,
        });
        spawn_dial_ui(&mut commands, &asset_server, dial_idx, dial_type, current, dial_targets.targets[dial_idx]);
    }
}

fn spawn_dial_ui(
    commands: &mut Commands,
    asset_server: &AssetServer,
    dial_idx: usize,
    dial_type: DialType,
    current: u8,
    target: Option<u8>,
) {
    let font: Handle<Font> = asset_server.load(FONT_PATH);
    let title = ["DIAL A", "DIAL B", "DIAL C"][dial_idx];
    let accent = match dial_type {
        DialType::Code   => Color::srgb(0.9, 0.9, 0.2),
        DialType::Color  => Color::srgb(1.0, 0.6, 0.1),
        DialType::Symbol => Color::srgb(0.8, 0.3, 1.0),
    };
    let target_str = match target {
        Some(t) => format!("TARGET: {}", t),
        None    => "TARGET: ???".to_string(),
    };

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(280.0),
                height: Val::Auto,
                left: Val::Percent(50.0),
                top: Val::Percent(40.0),
                margin: UiRect { left: Val::Px(-140.0), ..default() },
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(10.0),
                padding: UiRect::all(Val::Px(20.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.03, 0.08, 0.96)),
            BorderColor(accent),
            BorderRadius::all(Val::Px(8.0)),
            ZIndex(30),
            DialUi,
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new(title),
                TextFont { font: font.clone(), font_size: 22.0, ..default() },
                TextColor(accent),
            ));
            panel.spawn((
                Text::new(target_str),
                TextFont { font: font.clone(), font_size: 16.0, ..default() },
                TextColor(Color::srgb(0.7, 0.7, 0.7)),
            ));
            panel.spawn((
                Text::new(format!("▲  {}  ▼", current)),
                TextFont { font: font.clone(), font_size: 32.0, ..default() },
                TextColor(Color::WHITE),
                DialCurrentText,
            ));
            panel.spawn((
                Text::new("W/S  change    Enter  confirm    E  close"),
                TextFont { font: font.clone(), font_size: 13.0, ..default() },
                TextColor(Color::srgb(0.5, 0.5, 0.5)),
            ));
        });
}

// ── Dial UI update ────────────────────────────────────────────────────────────

pub(super) fn update_dial_ui(
    mut commands: Commands,
    dial_state: Option<ResMut<DialInteractState>>,
    mut dial_q: Query<(&mut DialButton, &mut Sprite)>,
    dial_targets: Res<DialTargets>,
    input: Res<ButtonInput<KeyCode>>,
    bindings: Res<KeyBindings>,
    mut current_text_q: Query<&mut Text, With<DialCurrentText>>,
    ui_q: Query<Entity, With<DialUi>>,
) {
    let Some(mut state) = dial_state else { return };

    let dial_max: u8 = match state.dial_type {
        DialType::Code   => 9,
        DialType::Color  => 3,
        DialType::Symbol => 5,
    };

    if input.just_pressed(bindings.interact) {
        close_dial_ui(&mut commands, &ui_q);
        return;
    }

    if input.just_pressed(bindings.move_up) {
        state.current = (state.current + 1) % (dial_max + 1);
    }
    if input.just_pressed(bindings.move_down) {
        state.current = (state.current + dial_max) % (dial_max + 1);
    }

    if let Ok(mut txt) = current_text_q.single_mut() {
        *txt = Text::new(format!("▲  {}  ▼", state.current));
    }

    if input.just_pressed(KeyCode::Enter) {
        let confirmed_val = state.current;
        let target = dial_targets.targets[state.dial_idx];
        if let Ok((mut dial, mut sprite)) = dial_q.get_mut(state.dial_entity) {
            dial.current = confirmed_val;
            if target == Some(confirmed_val) {
                sprite.color = Color::srgb(0.2, 1.0, 0.3);
            } else {
                sprite.color = Color::srgb(1.0, 1.0, 1.0);
            }
        }
        close_dial_ui(&mut commands, &ui_q);
    }
}

fn close_dial_ui(commands: &mut Commands, ui_q: &Query<Entity, With<DialUi>>) {
    for e in ui_q.iter() {
        commands.entity(e).despawn();
    }
    commands.remove_resource::<DialInteractState>();
}

// ── Check all dials ───────────────────────────────────────────────────────────

pub(super) fn check_all_dials(
    mut commands: Commands,
    dial_q: Query<&DialButton>,
    dial_targets: Res<DialTargets>,
    mut boss_door_q: Query<&mut PlanetBossDoor>,
    mini_gate_q: Query<Entity, With<MiniBossGate>>,
) {
    let targets_ready = dial_targets.targets.iter().all(|t| t.is_some());
    if !targets_ready || dial_q.is_empty() { return; }

    let all_correct = dial_q.iter().all(|dial| {
        dial_targets.targets[dial.dial_idx] == Some(dial.current)
    });

    if all_correct {
        for mut door in &mut boss_door_q {
            door.ready = true;
        }
        for gate_e in &mini_gate_q {
            commands.entity(gate_e).remove::<Collidable>();
            commands.entity(gate_e).remove::<Collider>();
        }
    }
}

// ── Planet 2 boss door proximity ──────────────────────────────────────────────

pub(super) fn boss_door_proximity(
    mut commands: Commands,
    player_q: Query<&Transform, With<Player>>,
    mut door_q: Query<(Entity, &Transform, &mut Sprite, &PlanetBossDoor)>,
    prompt_q: Query<Entity, With<PlanetBossDoorPrompt>>,
    session: Option<Res<TerminalSession>>,
    code_session: Option<Res<CodeEntryState>>,
    dial_state: Option<Res<DialInteractState>>,
    input: Res<ButtonInput<KeyCode>>,
    bindings: Res<KeyBindings>,
    asset_server: Res<AssetServer>,
) {
    if session.is_some() || code_session.is_some() || dial_state.is_some() { return; }

    let Ok(player_tf) = player_q.single() else { return };
    let pp = player_tf.translation;
    let interact_half = Vec2::splat(TILE_SIZE * 2.5);
    let door_half = Vec2::splat(TILE_SIZE * 0.5);

    let mut near_pos: Option<Vec3> = None;
    let mut is_ready = false;
    for (_, door_tf, _, door) in door_q.iter() {
        if aabb_overlap(pp.x, pp.y, interact_half, door_tf.translation.x, door_tf.translation.y, door_half) {
            near_pos = Some(door_tf.translation);
            is_ready = door.ready;
            break;
        }
    }

    for e in &prompt_q { commands.entity(e).despawn(); }

    let Some(door_pos) = near_pos else { return };

    let (text, color) = if is_ready {
        ("[E] Open".to_string(), Color::srgb(0.2, 1.0, 0.4))
    } else {
        ("Calibration incomplete".to_string(), Color::srgb(0.8, 0.2, 0.2))
    };

    let font: Handle<Font> = asset_server.load(FONT_PATH);
    commands.spawn((
        Text2d::new(text),
        TextFont { font, font_size: 18.0, ..default() },
        TextColor(color),
        Transform::from_translation(door_pos + Vec3::new(0.0, TILE_SIZE * 1.5, 10.0)),
        PlanetBossDoorPrompt,
        GameEntity,
    ));

    if is_ready && input.just_pressed(bindings.interact) {
        let popup_pos = pp + Vec3::new(0.0, TILE_SIZE * 2.0, 100.0);
        for (entity, _, mut sprite, _) in door_q.iter_mut() {
            sprite.color = Color::srgb(0.3, 0.3, 0.3);
            commands.entity(entity).remove::<Collidable>();
            commands.entity(entity).remove::<Collider>();
        }
        let font: Handle<Font> = asset_server.load(FONT_PATH);
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
