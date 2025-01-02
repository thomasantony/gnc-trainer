use bevy::prelude::*;
use rand::Rng;

use crate::simulation::LanderState;
use crate::visualization::{world_to_screen, CameraState, LevelSpecific};

// Constants for particle system
const PARTICLE_LIFETIME: f32 = 0.5;
const PARTICLE_SIZE: f32 = 2.0;
const PARTICLE_BASE_SPEED: f32 = 150.0;
const PARTICLE_SPREAD: f32 = 0.30; // Spread angle (in radians)
const PARTICLE_COUNT_PER_SPAWN: i32 = 10;
const PARTICLE_BOUNCE_DAMPING: f32 = 0.1;
const PARTICLE_GROUND_Y: f32 = 0.1;
const LANDER_HEIGHT: f32 = 3.0; // Duplicated from constants.rs for particle positioning

const EXPLOSION_PARTICLE_COUNT_MIN: usize = 100;
const EXPLOSION_PARTICLE_COUNT_MAX: usize = 200;
const EXPLOSION_PARTICLE_SPEED: f32 = 200.0;
const EXPLOSION_PARTICLE_SPREAD: f32 = 0.25;

#[derive(Component)]
pub struct ExhaustParticle {
    lifetime: Timer,
    velocity: Vec2,
    world_pos: Vec2,
}

#[derive(Resource)]
pub struct ParticleSpawnTimer(pub Timer);

fn spawn_particle(
    commands: &mut Commands,
    lander_pos: Vec2,
    base_position: Vec2,
    particle_direction: Vec2,
    camera_offset: Vec2,
) {
    let mut rng = rand::thread_rng();
    let spread = PARTICLE_SPREAD;
    let angle_offset = rng.gen_range(-spread..spread);
    let angle = Vec2::new(
        particle_direction.x * angle_offset.cos() - particle_direction.y * angle_offset.sin(),
        particle_direction.x * angle_offset.sin() + particle_direction.y * angle_offset.cos(),
    );
    let speed = PARTICLE_BASE_SPEED * rng.gen_range(0.8..1.2);

    let offset = Vec2::new(rng.gen_range(-0.2..0.2), rng.gen_range(0.0..0.5));
    let world_pos = lander_pos + base_position + offset;
    let screen_pos = world_to_screen(world_pos, camera_offset);

    commands.spawn((
        Sprite {
            color: Color::srgba(0.8, 0.3, 0.2, 0.8),
            custom_size: Some(Vec2::new(PARTICLE_SIZE, PARTICLE_SIZE)),
            ..default()
        },
        Transform::from_xyz(screen_pos.x, screen_pos.y, 0.5),
        ExhaustParticle {
            lifetime: Timer::from_seconds(PARTICLE_LIFETIME, TimerMode::Once),
            velocity: angle * speed,
            world_pos,
        },
        LevelSpecific,
    ));
}

// Make something Rico would appreciate
pub fn kaboom(
    commands: &mut Commands,
    lander_pos: Vec2,
    lander_vel: Vec2,
    lander_transform: &Transform,
) {
    let mut rng = rand::thread_rng();

    let particle_count = rng.gen_range(EXPLOSION_PARTICLE_COUNT_MIN..EXPLOSION_PARTICLE_COUNT_MAX);
    for i in 0..particle_count {
        let angle = (i as f32 / particle_count as f32) * std::f32::consts::TAU;
        let angle_offset = rng.gen_range(-EXPLOSION_PARTICLE_SPREAD..EXPLOSION_PARTICLE_SPREAD);

        let direction = Vec2::new(angle.cos(), angle.sin());
        let direction = Vec2::new(
            direction.x * angle_offset.cos() - direction.y * angle_offset.sin(),
            direction.x * angle_offset.sin() + direction.y * angle_offset.cos(),
        );
        let velocity = EXPLOSION_PARTICLE_SPEED * direction * rng.gen_range(0.8..1.2);

        commands.spawn((
            Sprite {
                color: Color::srgb(1.0, 0.5, 0.0),
                custom_size: Some(Vec2::new(4.0, 4.0)),
                ..default()
            },
            Transform::from_translation(lander_transform.translation),
            ExhaustParticle {
                lifetime: Timer::from_seconds(1.0, TimerMode::Once),
                velocity: velocity + lander_vel,
                world_pos: lander_pos,
            },
            LevelSpecific,
        ));
    }
}

pub fn particle_system(
    mut commands: Commands,
    time: Res<Time>,
    mut timer: ResMut<ParticleSpawnTimer>,
    mut camera_state: ResMut<CameraState>,
    mut query_set: ParamSet<(
        Query<(Entity, &Transform, &mut Visibility), With<crate::visualization::Lander>>,
        Query<(Entity, &mut Transform, &mut ExhaustParticle)>,
    )>,
    lander_state: Res<LanderState>,
) {
    let dt = time.delta_secs();

    // Update existing particles
    let mut to_despawn = Vec::new();
    {
        let mut particle_query = query_set.p1();
        for (entity, mut transform, mut particle) in particle_query.iter_mut() {
            particle.lifetime.tick(time.delta());

            if particle.lifetime.finished() {
                to_despawn.push(entity);
            } else {
                let delta = particle.velocity * dt;
                transform.translation.x += delta.x;
                transform.translation.y += delta.y;
                particle.world_pos += delta / crate::visualization::WORLD_TO_SCREEN_SCALE;

                if particle.world_pos.y <= PARTICLE_GROUND_Y {
                    particle.world_pos.y = PARTICLE_GROUND_Y;
                    particle.velocity.y = -particle.velocity.y * PARTICLE_BOUNCE_DAMPING;
                    particle.velocity.x *= 0.9;

                    if particle.velocity.length() < 20.0 {
                        to_despawn.push(entity);
                    }
                }
            }
        }
    }

    for entity in to_despawn {
        commands.entity(entity).despawn();
    }

    // Handle crash explosion
    if lander_state.crashed && !camera_state.explosion_spawned {
        let mut lander_query = query_set.p0();
        if let Ok((_entity, lander_transform, mut visibility)) = lander_query.get_single_mut() {
            *visibility = Visibility::Hidden;
            kaboom(
                &mut commands,
                lander_state.position,
                lander_state.velocity,
                lander_transform,
            );
            camera_state.explosion_spawned = true;
        }
    }

    // Spawn new particles
    if lander_state.thrust_level > 0.0 && !lander_state.landed && !lander_state.crashed {
        timer.0.tick(time.delta());
        if timer.0.just_finished() {
            let num_particles =
                (lander_state.thrust_level * PARTICLE_COUNT_PER_SPAWN as f32) as i32;
            let exhaust_angle =
                lander_state.rotation + lander_state.gimbal_angle + std::f32::consts::FRAC_PI_2;
            let exhaust_direction = -Vec2::new(exhaust_angle.cos(), exhaust_angle.sin());

            let base_offset = Vec2::new(
                lander_state.rotation.sin() * LANDER_HEIGHT / 2.0,
                -lander_state.rotation.cos() * LANDER_HEIGHT / 2.0,
            );

            for _ in 0..num_particles {
                spawn_particle(
                    &mut commands,
                    lander_state.position,
                    base_offset,
                    exhaust_direction,
                    camera_state.target_offset,
                );
            }
        }
    }
}
