use bevy::prelude::*;
use rand::random_range;
use crate::player::{Player, aabb_overlap};
use crate::rewards::RewardPopup;
use crate::{GameEntity, GameState, PlanetLevelMarker, StationLevel, TILE_SIZE, Z_ENTITIES};
use crate::room::RoomVec;

pub const SYMBOL_CHARS: [&str; 6] = ["▲", "●", "■", "⬡", "✦", "⊕"];

// ── Resources ─────────────────────────────────────────────────────────────────

/// Symbol values collected from each station in the current 3-station cycle.
/// Index = position in cycle (0, 1, 2). None = not yet found.
/// Values: 0=▲ 1=● 2=■ 3=⬡ 4=✦ 5=⊕
#[derive(Resource, Default, Clone)]
pub struct StationSymbols {
    pub symbols: [Option<u8>; 3],
}

// ── Components ────────────────────────────────────────────────────────────────

#[derive(Component)]
pub struct SymbolChip {
    pub station_index: usize,
    pub symbol: u8,
}

#[derive(Resource)]
pub struct SymbolChipRes {
    pub img: Handle<Image>,
}

// ── Plugin ────────────────────────────────────────────────────────────────────

pub struct StationSymbolPlugin;

impl Plugin for StationSymbolPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<StationSymbols>()
            .add_systems(Startup, load_assets)
            .add_systems(OnEnter(GameState::Loading), init_station_symbols)
            .add_systems(OnEnter(GameState::Playing), spawn_symbol_chip)
            .add_systems(
                Update,
                collect_symbol_chip.run_if(in_state(GameState::Playing)),
            );
    }
}

// ── Systems ───────────────────────────────────────────────────────────────────

fn load_assets(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(SymbolChipRes {
        img: asset_server.load("items/key.png"),
    });
}

pub fn init_station_symbols(
    mut commands: Commands,
    saved: Option<Res<crate::SavedPlayerBuffs>>,
) {
    let symbols = saved
        .map(|b| b.station_symbols)
        .unwrap_or([None; 3]);
    commands.insert_resource(StationSymbols { symbols });
}

fn spawn_symbol_chip(
    mut commands: Commands,
    res: Res<SymbolChipRes>,
    symbols: Res<StationSymbols>,
    station_level: Res<StationLevel>,
    rooms: Res<RoomVec>,
    planet: Option<Res<PlanetLevelMarker>>,
) {
    if planet.is_some() { return; }

    let station_index = (station_level.0 % 3) as usize;

    if symbols.symbols[station_index].is_some() { return; }

    let symbol = random_range(0u8..6u8);

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
    let pos = Vec3::new(center.x + TILE_SIZE * 2.0, center.y - TILE_SIZE * 2.0, Z_ENTITIES);

    let mut sprite = Sprite::from_image(res.img.clone());
    sprite.color = Color::srgb(0.7, 0.2, 1.0);

    commands.spawn((
        sprite,
        Transform::from_translation(pos),
        SymbolChip { station_index, symbol },
        GameEntity,
    ));
}

fn collect_symbol_chip(
    mut commands: Commands,
    player_q: Query<&Transform, With<Player>>,
    chip_q: Query<(Entity, &Transform, &SymbolChip)>,
    mut symbols: ResMut<StationSymbols>,
    asset_server: Res<AssetServer>,
) {
    let Ok(player_tf) = player_q.single() else { return };
    let pp = player_tf.translation;
    let half = Vec2::splat(TILE_SIZE * 0.8);

    for (entity, chip_tf, chip) in &chip_q {
        let cp = chip_tf.translation;
        if aabb_overlap(pp.x, pp.y, half, cp.x, cp.y, half) {
            symbols.symbols[chip.station_index] = Some(chip.symbol);
            commands.entity(entity).despawn();

            let font: Handle<Font> = asset_server.load(crate::SYMBOL_FONT_PATH);
            commands.spawn((
                Text2d::new(format!(
                    "Symbol Chip: Station {} = {}",
                    chip.station_index + 1,
                    SYMBOL_CHARS[chip.symbol as usize]
                )),
                TextFont { font, font_size: 18.0, ..default() },
                TextColor(Color::srgb(0.8, 0.4, 1.0)),
                Transform::from_translation(cp + Vec3::new(0.0, TILE_SIZE, 10.0)),
                RewardPopup { timer: Timer::from_seconds(2.0, TimerMode::Once) },
                GameEntity,
            ));
        }
    }
}
