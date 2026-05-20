use bevy::prelude::*;
use rand::random_range;
use std::f32::consts::{TAU, FRAC_PI_2};

use crate::{GameEntity, GameState, Z_ENTITIES};
use crate::player::Player;
use crate::room::RoomVec;

#[derive(Component)]
struct AirParticle {
    velocity: Vec2,
    lifetime: Timer,
}

#[derive(Component)]
pub(crate) struct DashParticle {
    pub(crate) velocity: Vec2,
    pub(crate) lifetime: Timer,
}

#[derive(Resource)]
struct ParticleEmitTimer(Timer);

pub struct AirParticlePlugin;

impl Plugin for AirParticlePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ParticleEmitTimer(Timer::from_seconds(
            1.0 / 12.0,
            TimerMode::Repeating,
        )))
        .add_systems(
            Update,
            (emit_air_particles, update_air_particles, update_dash_particles)
                .run_if(in_state(GameState::Playing))
                .run_if(not(resource_exists::<crate::PlanetLevelMarker>)),
        );
    }
}

fn emit_air_particles(
    mut commands: Commands,
    time: Res<Time>,
    mut emit_timer: ResMut<ParticleEmitTimer>,
    rooms: Res<RoomVec>,
    player_query: Query<&Transform, With<Player>>,
) {
    emit_timer.0.tick(time.delta());
    if !emit_timer.0.just_finished() {
        return;
    }

    let Ok(player_tf) = player_query.single() else {
        return;
    };
    let player_pos = player_tf.translation.truncate();

    let breach_positions: Vec<Vec2> = rooms
        .0
        .iter()
        .find(|room| room.bounds_check(player_pos))
        .map(|room| room.breaches.clone())
        .unwrap_or_default();

    if breach_positions.is_empty() {
        return;
    }

    let nearest_breach = breach_positions
        .iter()
        .min_by(|a, b| {
            a.distance(player_pos)
                .partial_cmp(&b.distance(player_pos))
                .unwrap()
        })
        .copied()
        .unwrap();

    for _ in 0..4 {
        let angle: f32 = random_range(0.0..TAU);
        let radius: f32 = random_range(80.0..=180.0);
        let spawn_pos = player_pos + Vec2::new(angle.cos(), angle.sin()) * radius;

        let base_dir = (nearest_breach - spawn_pos).normalize_or_zero();

        // rotate base_dir by a small random spread (±20°)
        let spread: f32 = random_range(-0.35..=0.35);
        let (sin_s, cos_s) = spread.sin_cos();
        let jittered_dir = Vec2::new(
            base_dir.x * cos_s - base_dir.y * sin_s,
            base_dir.x * sin_s + base_dir.y * cos_s,
        );

        let speed: f32 = random_range(100.0..=180.0);
        let velocity = jittered_dir * speed;

        let rotation = Quat::from_rotation_z(velocity.to_angle() - FRAC_PI_2);

        commands.spawn((
            Sprite::from_color(Color::srgba(0.7, 0.9, 1.0, 0.8), Vec2::new(3.0, 7.0)),
            Transform {
                translation: spawn_pos.extend(Z_ENTITIES + 5.0),
                rotation,
                ..default()
            },
            AirParticle {
                velocity,
                lifetime: Timer::from_seconds(random_range(1.0..=2.0), TimerMode::Once),
            },
            GameEntity,
        ));
    }
}

fn update_air_particles(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Transform, &mut Sprite, &mut AirParticle)>,
) {
    for (entity, mut tf, mut sprite, mut particle) in &mut query {
        particle.lifetime.tick(time.delta());

        if particle.lifetime.finished() {
            commands.entity(entity).despawn();
            continue;
        }

        tf.translation += (particle.velocity * time.delta_secs()).extend(0.0);

        let alpha = 1.0 - particle.lifetime.fraction();
        sprite.color = sprite.color.with_alpha(alpha);
    }
}

fn update_dash_particles(
    mut commands: Commands,
    time: Res<Time>,
    rooms: Res<RoomVec>,
    mut query: Query<(Entity, &mut Transform, &mut Sprite, &mut DashParticle)>,
) {
    for (entity, mut tf, mut sprite, mut particle) in &mut query {
        particle.lifetime.tick(time.delta());

        if particle.lifetime.finished() {
            commands.entity(entity).despawn();
            continue;
        }

        // Gently steer toward nearest breach so particles flow with the vacuum
        let pos = tf.translation.truncate();
        if let Some(room) = rooms.0.iter().find(|r| r.bounds_check(pos)) {
            if let Some(&breach_pos) = room.breaches.iter().min_by(|a, b| {
                a.distance(pos).partial_cmp(&b.distance(pos)).unwrap()
            }) {
                let to_breach = (breach_pos - pos).normalize_or_zero();
                particle.velocity += to_breach * 3000.0 * time.delta_secs();
            }
        }

        tf.translation += (particle.velocity * time.delta_secs()).extend(0.0);

        let alpha = 1.0 - particle.lifetime.fraction();
        sprite.color = sprite.color.with_alpha(alpha);
    }
}
