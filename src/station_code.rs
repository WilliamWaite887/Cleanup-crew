use bevy::prelude::*;
use rand::random_range;
use crate::collidable::Collidable;
use crate::player::{Player, aabb_overlap};
use crate::rewards::RewardPopup;
use crate::{GameEntity, GameState, PlanetLevelMarker, StationLevel, FONT_PATH, TILE_SIZE, Z_ENTITIES};
use crate::room::RoomVec;

// ── Resources ─────────────────────────────────────────────────────────────────

/// Digits collected from each station in the current 3-station cycle.
/// Index = position in cycle (0, 1, 2). None = not yet found in that station.
#[derive(Resource, Default, Clone)]
pub struct StationCodes {
    pub codes: [Option<u8>; 3],
}

// ── Components ────────────────────────────────────────────────────────────────

/// Floor pickup entity spawned in a station; collecting it reveals that station's digit.
#[derive(Component)]
pub struct CodeFragment {
    pub station_index: usize,
    pub digit: u8,
}

/// Marker for the three HUD digit slots.
#[derive(Component)]
pub struct CodeHudSlot(pub usize);

/// Root node of the code HUD row.
#[derive(Component)]
pub struct CodeHudRoot;

#[derive(Resource)]
pub struct CodeFragmentRes {
    pub img: Handle<Image>,
}

// ── Plugin ────────────────────────────────────────────────────────────────────

pub struct StationCodePlugin;

impl Plugin for StationCodePlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<StationCodes>()
            .add_systems(Startup, load_assets)
            .add_systems(
                OnEnter(GameState::Loading),
                init_station_codes,
            )
            .add_systems(
                OnEnter(GameState::Playing),
                (spawn_code_fragment, setup_code_hud),
            )
            .add_systems(
                Update,
                (collect_code_fragment, update_code_hud)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

// ── Systems ───────────────────────────────────────────────────────────────────

fn load_assets(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(CodeFragmentRes {
        // Reuses key sprite with a cyan tint for now; replace with a dedicated asset later.
        img: asset_server.load("items/key.png"),
    });
}

/// Restore codes from SavedPlayerBuffs when the level begins loading.
pub fn init_station_codes(
    mut commands: Commands,
    saved: Option<Res<crate::SavedPlayerBuffs>>,
) {
    let codes = saved
        .map(|b| b.station_codes)
        .unwrap_or([None; 3]);
    commands.insert_resource(StationCodes { codes });
}

/// Generate a random digit for this station and spawn the pickup in a mid-level room.
/// Skipped on the planet level (no code fragments there).
fn spawn_code_fragment(
    mut commands: Commands,
    res: Res<CodeFragmentRes>,
    mut codes: ResMut<StationCodes>,
    station_level: Res<StationLevel>,
    rooms: Res<RoomVec>,
    planet: Option<Res<PlanetLevelMarker>>,
) {
    if planet.is_some() { return; }

    let station_index = (station_level.0 % 3) as usize;

    // Only spawn once per station — skip if already collected in a prior visit.
    if codes.codes[station_index].is_some() { return; }

    let digit = random_range(0u8..=9u8);
    codes.codes[station_index] = Some(digit);

    // Pick a non-airlock room in the middle of the list for the spawn position.
    let non_airlock: Vec<&crate::room::Room> =
        rooms.0.iter().filter(|r| !r.is_airlock).collect();

    let target_room = if non_airlock.len() >= 3 {
        non_airlock[non_airlock.len() / 2]
    } else if let Some(r) = non_airlock.first() {
        r
    } else {
        return;
    };

    let center = (target_room.top_left_corner + target_room.bot_right_corner) * 0.5;
    let pos = Vec3::new(center.x + TILE_SIZE, center.y + TILE_SIZE, Z_ENTITIES);

    let mut sprite = Sprite::from_image(res.img.clone());
    // Cyan tint to distinguish it from the regular (white) key pickup.
    sprite.color = Color::srgb(0.2, 1.0, 1.0);

    commands.spawn((
        sprite,
        Transform::from_translation(pos),
        CodeFragment { station_index, digit },
        GameEntity,
    ));
}

/// Collect the code fragment when the player walks over it.
fn collect_code_fragment(
    mut commands: Commands,
    player_q: Query<&Transform, With<Player>>,
    fragment_q: Query<(Entity, &Transform, &CodeFragment)>,
    asset_server: Res<AssetServer>,
) {
    let Ok(player_tf) = player_q.single() else { return };
    let pp = player_tf.translation;
    let half = Vec2::splat(TILE_SIZE * 0.8);

    for (entity, frag_tf, frag) in &fragment_q {
        let fp = frag_tf.translation;
        if aabb_overlap(pp.x, pp.y, half, fp.x, fp.y, half) {
            commands.entity(entity).despawn();

            // Floating confirmation text.
            let font: Handle<Font> = asset_server.load(FONT_PATH);
            commands.spawn((
                Text2d::new(format!("Code Fragment: Station {} = {}", frag.station_index + 1, frag.digit)),
                TextFont { font, font_size: 18.0, ..default() },
                TextColor(Color::srgb(0.2, 1.0, 1.0)),
                Transform::from_translation(fp + Vec3::new(0.0, TILE_SIZE, 10.0)),
                RewardPopup { timer: Timer::from_seconds(2.0, TimerMode::Once) },
                GameEntity,
            ));
        }
    }
}

/// Spawn a small HUD row showing the 3 code digit slots.
fn setup_code_hud(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font: Handle<Font> = asset_server.load(FONT_PATH);

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(76.0),
                right: Val::Px(20.0),
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(4.0),
                align_items: AlignItems::Center,
                padding: UiRect::all(Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
            BorderRadius::all(Val::Px(4.0)),
            ZIndex(10),
            CodeHudRoot,
            GameEntity,
        ))
        .with_children(|row| {
            row.spawn((
                Text::new("CODE "),
                TextFont { font: font.clone(), font_size: 16.0, ..default() },
                TextColor(Color::srgb(0.4, 0.8, 1.0)),
            ));
            for i in 0..3usize {
                row.spawn((
                    Text::new("[ ? ]"),
                    TextFont { font: font.clone(), font_size: 16.0, ..default() },
                    TextColor(Color::srgb(0.5, 0.5, 0.5)),
                    CodeHudSlot(i),
                ));
            }
        });
}

/// Keep HUD slots in sync with collected digits.
fn update_code_hud(
    codes: Res<StationCodes>,
    mut slot_q: Query<(&CodeHudSlot, &mut Text, &mut TextColor)>,
) {
    if !codes.is_changed() { return; }
    for (slot, mut text, mut color) in &mut slot_q {
        match codes.codes[slot.0] {
            Some(d) => {
                *text = Text::new(format!("[ {} ]", d));
                *color = TextColor(Color::srgb(0.2, 1.0, 1.0));
            }
            None => {
                *text = Text::new("[ ? ]");
                *color = TextColor(Color::srgb(0.5, 0.5, 0.5));
            }
        }
    }
}
