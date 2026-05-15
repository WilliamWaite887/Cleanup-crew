use bevy::prelude::*;
use rand::random_range;
use crate::player::{Player, aabb_overlap};
use crate::rewards::RewardPopup;
use crate::{GameEntity, GameState, PlanetLevelMarker, StationLevel, FONT_PATH, TILE_SIZE, Z_ENTITIES};
use crate::room::RoomVec;

const COLOR_NAMES: [&str; 4] = ["RED", "GRN", "BLU", "YLW"];

// ── Resources ─────────────────────────────────────────────────────────────────

/// Color values collected from each station in the current 3-station cycle.
/// Index = position in cycle (0, 1, 2). None = not yet found.
/// Values: 0=RED 1=GRN 2=BLU 3=YLW
#[derive(Resource, Default, Clone)]
pub struct StationColors {
    pub colors: [Option<u8>; 3],
}

// ── Components ────────────────────────────────────────────────────────────────

#[derive(Component)]
pub struct ColorChip {
    pub station_index: usize,
    pub color: u8,
}

#[derive(Resource)]
pub struct ColorChipRes {
    pub img: Handle<Image>,
}

// ── Plugin ────────────────────────────────────────────────────────────────────

pub struct StationColorPlugin;

impl Plugin for StationColorPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<StationColors>()
            .add_systems(Startup, load_assets)
            .add_systems(OnEnter(GameState::Loading), init_station_colors)
            .add_systems(OnEnter(GameState::Playing), spawn_color_chip)
            .add_systems(
                Update,
                collect_color_chip.run_if(in_state(GameState::Playing)),
            );
    }
}

// ── Systems ───────────────────────────────────────────────────────────────────

fn load_assets(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(ColorChipRes {
        img: asset_server.load("items/key.png"),
    });
}

pub fn init_station_colors(
    mut commands: Commands,
    saved: Option<Res<crate::SavedPlayerBuffs>>,
) {
    let colors = saved
        .map(|b| b.station_colors)
        .unwrap_or([None; 3]);
    commands.insert_resource(StationColors { colors });
}

fn spawn_color_chip(
    mut commands: Commands,
    res: Res<ColorChipRes>,
    colors: Res<StationColors>,
    station_level: Res<StationLevel>,
    rooms: Res<RoomVec>,
    planet: Option<Res<PlanetLevelMarker>>,
) {
    if planet.is_some() { return; }

    let station_index = (station_level.0 % 3) as usize;

    if colors.colors[station_index].is_some() { return; }

    let color = random_range(0u8..4u8);

    let non_airlock: Vec<&crate::room::Room> =
        rooms.0.iter().filter(|r| !r.is_airlock).collect();

    let target_room = if non_airlock.len() >= 4 {
        non_airlock[non_airlock.len() / 2 + 1]
    } else if let Some(r) = non_airlock.last() {
        r
    } else {
        return;
    };

    let center = (target_room.top_left_corner + target_room.bot_right_corner) * 0.5;
    let pos = Vec3::new(center.x - TILE_SIZE, center.y - TILE_SIZE, Z_ENTITIES);

    let mut sprite = Sprite::from_image(res.img.clone());
    sprite.color = Color::srgb(1.0, 0.3, 0.3);

    commands.spawn((
        sprite,
        Transform::from_translation(pos),
        ColorChip { station_index, color },
        GameEntity,
    ));
}

fn collect_color_chip(
    mut commands: Commands,
    player_q: Query<&Transform, With<Player>>,
    chip_q: Query<(Entity, &Transform, &ColorChip)>,
    mut colors: ResMut<StationColors>,
    asset_server: Res<AssetServer>,
) {
    let Ok(player_tf) = player_q.single() else { return };
    let pp = player_tf.translation;
    let half = Vec2::splat(TILE_SIZE * 0.8);

    for (entity, chip_tf, chip) in &chip_q {
        let cp = chip_tf.translation;
        if aabb_overlap(pp.x, pp.y, half, cp.x, cp.y, half) {
            colors.colors[chip.station_index] = Some(chip.color);
            commands.entity(entity).despawn();

            let font: Handle<Font> = asset_server.load(FONT_PATH);
            commands.spawn((
                Text2d::new(format!(
                    "Color Chip: Station {} = {}",
                    chip.station_index + 1,
                    COLOR_NAMES[chip.color as usize]
                )),
                TextFont { font, font_size: 18.0, ..default() },
                TextColor(Color::srgb(1.0, 0.4, 0.4)),
                Transform::from_translation(cp + Vec3::new(0.0, TILE_SIZE, 10.0)),
                RewardPopup { timer: Timer::from_seconds(2.0, TimerMode::Once) },
                GameEntity,
            ));
        }
    }
}
