use bevy::prelude::*;
use rand::random_range;
use crate::{GameEntity, GameState, PlanetLevelMarker, TILE_SIZE, Z_ENTITIES};
use crate::player::{Player, WeaponBuffStacks, aabb_overlap};
use crate::enemies::Enemy;
use crate::room::LevelState;

// ─── Components ──────────────────────────────────────────────────────────────

/// Marker added to one random enemy per planet run; their death spawns the KeyPickup.
#[derive(Component)]
pub struct KeyHolder;

/// The key sprite on the floor waiting to be collected.
#[derive(Component)]
pub struct KeyPickup;

/// The chest entity in the world (boss room).
#[derive(Component)]
pub struct Chest;

// ─── Resources ───────────────────────────────────────────────────────────────

#[derive(Resource)]
pub struct KeyChestRes {
    pub key_img:   Handle<Image>,
    pub chest_img: Handle<Image>,
}

#[derive(Resource)]
pub struct LevelKeyState {
    /// Index (0..6, non-airlock) of the room whose enemies include the KeyHolder.
    pub key_holder_room: usize,
    /// True once `KeyHolder` has been inserted on an enemy entity.
    pub key_assigned: bool,
    /// True once the player has collected the key off the floor.
    pub has_key: bool,
    /// True when this level is the planet run (key drops here, chest is in boss room).
    pub is_planet_run: bool,
    /// True once the boss-room chest has been spawned on the planet.
    pub boss_chest_spawned: bool,
}

impl LevelKeyState {
    fn new() -> Self {
        Self {
            key_holder_room: random_range(0..6usize),
            key_assigned: false,
            has_key: false,
            is_planet_run: false,
            boss_chest_spawned: false,
        }
    }
}

// ─── Plugin ──────────────────────────────────────────────────────────────────

pub struct KeyChestPlugin;

impl Plugin for KeyChestPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, load_assets)
            .add_systems(OnEnter(GameState::Loading), init_level_key_state)
            .add_systems(
                Update,
                (
                    assign_key_holder,
                    pickup_key,
                    interact_with_chest,
                )
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

// ─── Systems ─────────────────────────────────────────────────────────────────

fn load_assets(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(KeyChestRes {
        key_img:   asset_server.load("items/key.png"),
        chest_img: asset_server.load("chests/chest_closed.png"),
    });
}

fn init_level_key_state(
    mut commands: Commands,
    existing: Option<Res<LevelKeyState>>,
    planet_marker: Option<Res<PlanetLevelMarker>>,
) {
    if planet_marker.is_some() {
        // Entering the planet — preserve the key the player collected during the run.
        let has_key = existing.map_or(false, |s| s.has_key);
        commands.insert_resource(LevelKeyState {
            is_planet_run: true,
            has_key,
            ..LevelKeyState::new()
        });
    } else {
        commands.insert_resource(LevelKeyState::new());
    }
}

/// Tags one random enemy in the designated room with `KeyHolder`.
/// Only runs on the planet — the key is found here, not in the space station.
fn assign_key_holder(
    mut commands: Commands,
    mut key_state: ResMut<LevelKeyState>,
    lvl_state: Res<LevelState>,
    enemy_q: Query<Entity, With<Enemy>>,
) {
    if !key_state.is_planet_run { return; }
    if key_state.key_assigned { return; }
    let LevelState::InRoom(idx, _, _) = *lvl_state else { return };
    if idx != key_state.key_holder_room { return; }

    let enemies: Vec<Entity> = enemy_q.iter().collect();
    if enemies.is_empty() { return; }

    let pick = enemies[random_range(0..enemies.len())];
    commands.entity(pick).insert(KeyHolder);
    key_state.key_assigned = true;
}

/// Auto-collect the key when the player walks over it.
fn pickup_key(
    mut commands: Commands,
    mut key_state: ResMut<LevelKeyState>,
    player_q: Query<&Transform, With<Player>>,
    key_q: Query<(Entity, &Transform), With<KeyPickup>>,
) {
    let Ok(player_tf) = player_q.single() else { return };
    let pp = player_tf.translation;
    let half = Vec2::splat(TILE_SIZE * 0.6);

    for (entity, key_tf) in &key_q {
        let kp = key_tf.translation;
        if aabb_overlap(pp.x, pp.y, half, kp.x, kp.y, half) {
            commands.entity(entity).despawn();
            key_state.has_key = true;
        }
    }
}

/// Press the interact key near a chest while holding the key to open it.
fn interact_with_chest(
    mut commands: Commands,
    input: Res<ButtonInput<KeyCode>>,
    mut key_state: ResMut<LevelKeyState>,
    player_q: Query<&Transform, With<Player>>,
    chest_q: Query<(Entity, &Transform), With<Chest>>,
    mut inventory_q: Query<&mut crate::weapons::WeaponInventory, With<Player>>,
    buff_stacks_q: Query<&WeaponBuffStacks, With<Player>>,
    bindings: Res<crate::settings::KeyBindings>,
) {
    if !input.just_pressed(bindings.interact) { return; }
    if !key_state.has_key { return; }

    let Ok(player_tf) = player_q.single() else { return };
    let pp = player_tf.translation;
    let interact_half = Vec2::splat(TILE_SIZE * 1.5);
    let chest_half = Vec2::splat(TILE_SIZE * 0.5);

    for (entity, chest_tf) in &chest_q {
        let cp = chest_tf.translation;
        if aabb_overlap(pp.x, pp.y, interact_half, cp.x, cp.y, chest_half) {
            commands.entity(entity).despawn();
            key_state.has_key = false;
            if let Ok(mut inv) = inventory_q.single_mut() {
                let new_type = crate::weapons::WeaponType::BeamRifle;
                let already_owned = inv.weapons.iter().any(|w| w.weapon_type == new_type);
                if !already_owned {
                    let mut new_weapon = crate::weapons::Weapon::new(new_type);
                    if let Ok(stacks) = buff_stacks_q.single() {
                        for _ in 0..stacks.atk_speed {
                            crate::rewards::atk_speed::apply(&mut new_weapon);
                        }
                        for _ in 0..stacks.damage {
                            crate::rewards::damage_up::apply(&mut new_weapon);
                        }
                        for _ in 0..stacks.piercing {
                            crate::rewards::piercing::apply(&mut new_weapon);
                        }
                    }
                    inv.weapons.push(new_weapon);
                }
            }
            break;
        }
    }
}

// ─── Public helpers (called from other modules) ───────────────────────────────

/// Spawns a key pickup at the given world position.
/// Called by `enemies::check_enemy_health` when a `KeyHolder` enemy is killed.
pub fn drop_key(commands: &mut Commands, res: &KeyChestRes, pos: Vec3) {
    commands.spawn((
        Sprite::from_image(res.key_img.clone()),
        Transform::from_xyz(pos.x, pos.y, Z_ENTITIES),
        KeyPickup,
        GameEntity,
    ));
}
