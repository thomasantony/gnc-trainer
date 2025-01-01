use crate::constants::{LANDER_HEIGHT, LANDER_WIDTH};
use crate::levels::CurrentLevel;
use crate::simulation::LanderState;
use bevy::asset::RenderAssetUsages;
use bevy::color::palettes::css::*;
use bevy::prelude::*;
use rand::Rng;

// Constants for view configuration
const WORLD_TO_SCREEN_SCALE: f32 = 10.0;
const VISUALIZATION_WIDTH: f32 = 680.0;
const RIGHT_PANEL_WIDTH: f32 = 600.0;
const PARTICLE_LIFETIME: f32 = 0.5;
const GROUND_OFFSET: f32 = -200.0; // Pixels from center of screen to ground
const MIN_VIEW_HEIGHT: f32 = 30.0; // Minimum world height (in meters) visible in the view

#[derive(Resource)]
pub struct CameraState {
    pub following: bool,
    pub target_offset: Vec2, // Changed to Vec2 to track both x and y offsets
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            following: true,
            target_offset: Vec2::ZERO,
        }
    }
}

// Components
#[derive(Component)]
pub struct Lander;

#[derive(Component)]
struct Ground;

#[derive(Component)]
pub struct LandingZone;

#[derive(Component)]
pub struct ExhaustParticle {
    lifetime: Timer,
    velocity: Vec2,
}

// Resource to manage particle spawning
#[derive(Resource)]
pub struct ParticleSpawnTimer(Timer);

fn create_triangle_mesh() -> Mesh {
    let mut mesh = Mesh::new(
        bevy::render::render_resource::PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );

    // Convert from meters to screen coordinates
    let half_height = (LANDER_HEIGHT / 2.0) * WORLD_TO_SCREEN_SCALE;
    let half_width = (LANDER_WIDTH / 2.0) * WORLD_TO_SCREEN_SCALE;

    let vertices = [
        [0.0, half_height, 0.0],          // top
        [-half_width, -half_height, 0.0], // bottom left
        [half_width, -half_height, 0.0],  // bottom right
    ];
    let indices = [0u32, 1, 2];
    let normals = [[0.0, 0.0, 1.0]; 3];
    let uvs = [[0.5, 0.0], [0.0, 1.0], [1.0, 1.0]];

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices.to_vec());
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals.to_vec());
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs.to_vec());
    mesh.insert_indices(bevy::render::mesh::Indices::U32(indices.to_vec()));
    mesh
}

pub fn spawn_visualization(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    level: Res<CurrentLevel>,
) {
    commands.insert_resource(CameraState::default());
    let center_offset = -(RIGHT_PANEL_WIDTH / 2.0);

    // Spawn lander
    commands.spawn((
        Mesh2d(meshes.add(create_triangle_mesh())),
        MeshMaterial2d(materials.add(ColorMaterial::from_color(PURPLE))),
        Transform {
            translation: Vec3::new(center_offset, 0.0, 1.0),
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        },
        Lander,
    ));

    // Spawn ground
    commands.spawn((
        Sprite {
            color: FOREST_GREEN.into(),
            custom_size: Some(Vec2::new(VISUALIZATION_WIDTH, 2.0)),
            ..default()
        },
        Transform::from_xyz(center_offset, GROUND_OFFSET, 0.0),
        Ground,
    ));

    // Spawn landing zone
    let landing_zone_width =
        (level.config.success.x_max - level.config.success.x_min) * WORLD_TO_SCREEN_SCALE;
    let landing_zone_x =
        (level.config.success.x_min + level.config.success.x_max) / 2.0 * WORLD_TO_SCREEN_SCALE;

    commands.spawn((
        Sprite {
            color: Color::srgba(0.0, 1.0, 0.0, 0.3), // Semi-transparent green
            custom_size: Some(Vec2::new(landing_zone_width, 10.0)),
            ..default()
        },
        Transform::from_xyz(center_offset + landing_zone_x, GROUND_OFFSET + 5.0, 0.5),
        LandingZone,
    ));

    commands.insert_resource(ParticleSpawnTimer(Timer::from_seconds(
        0.05,
        TimerMode::Repeating,
    )));
}

fn calculate_view_offset(lander_pos: Vec2) -> Vec2 {
    // Always calculate full offset needed to center the lander
    let screen_pos_without_offset = Vec2::new(
        lander_pos.x * WORLD_TO_SCREEN_SCALE,
        lander_pos.y * WORLD_TO_SCREEN_SCALE + GROUND_OFFSET,
    );

    // For X: always follow to keep centered horizontally
    let x_offset = screen_pos_without_offset.x;

    // For Y: smoothly transition based on height
    let ground_view_height = MIN_VIEW_HEIGHT * WORLD_TO_SCREEN_SCALE;
    let full_follow_height = ground_view_height * 2.0;
    let screen_y = lander_pos.y * WORLD_TO_SCREEN_SCALE;

    let y_offset = if screen_y > full_follow_height {
        // Above transition: full vertical follow
        screen_pos_without_offset.y
    } else if screen_y < ground_view_height {
        // Below transition: no vertical follow
        0.0
    } else {
        // In transition: smoothly interpolate
        let t = (screen_y - ground_view_height) / ground_view_height;
        screen_pos_without_offset.y * t
    };

    Vec2::new(x_offset, y_offset)
}

fn world_to_screen(pos: Vec2, camera_offset: Vec2) -> Vec2 {
    let center_offset = -(RIGHT_PANEL_WIDTH / 2.0);

    Vec2::new(
        pos.x * WORLD_TO_SCREEN_SCALE + center_offset - camera_offset.x,
        pos.y * WORLD_TO_SCREEN_SCALE + GROUND_OFFSET - camera_offset.y,
    )
}

pub fn update_visualization(
    mut lander_query: Query<&mut Transform, With<Lander>>,
    mut landing_zone_query: Query<&mut Transform, (With<LandingZone>, Without<Lander>)>,
    mut camera_state: ResMut<CameraState>,
    lander_state: Res<LanderState>,
    level: Res<CurrentLevel>,
) {
    // Calculate view offset based on lander position
    let offset = calculate_view_offset(lander_state.position);
    camera_state.target_offset = offset;

    if let Ok(mut transform) = lander_query.get_single_mut() {
        let screen_pos = world_to_screen(lander_state.position, offset);
        transform.translation.x = screen_pos.x;
        transform.translation.y = screen_pos.y;
        transform.rotation = Quat::from_rotation_z(lander_state.rotation);
    }

    if let Ok(mut transform) = landing_zone_query.get_single_mut() {
        let landing_zone_pos = Vec2::new(
            (level.config.success.x_min + level.config.success.x_max) / 2.0,
            0.0, // Ground level
        );
        let screen_pos = world_to_screen(landing_zone_pos, camera_state.target_offset);
        transform.translation.x = screen_pos.x;
        transform.translation.y = screen_pos.y + 5.0; // Slight offset from ground
    }
}

fn spawn_particle(
    commands: &mut Commands,
    lander_pos: Vec3,
    base_position: Vec2,
    particle_direction: Vec2,
) {
    let mut rng = rand::thread_rng();
    let spread = 0.2; // Spread angle in radians
    let angle = particle_direction + rng.gen_range(-spread..spread);
    let speed = 100.0 * rng.gen_range(0.8..1.2); // Pixels per second, with some variation

    let offset = Vec2::new(
        base_position.x + rng.gen_range(-3.0..3.0),
        base_position.y + rng.gen_range(-3.0..3.0),
    );

    commands.spawn((
        Sprite {
            color: ORANGE_RED.into(),
            custom_size: Some(Vec2::new(2.0, 2.0)),
            ..default()
        },
        Transform::from_xyz(lander_pos.x + offset.x, lander_pos.y + offset.y, 0.5),
        ExhaustParticle {
            lifetime: Timer::from_seconds(PARTICLE_LIFETIME, TimerMode::Once),
            velocity: (particle_direction + angle) * speed,
        },
    ));
}

pub fn particle_system(
    mut commands: Commands,
    time: Res<Time>,
    mut timer: ResMut<ParticleSpawnTimer>,
    camera_state: Res<CameraState>,
    mut query_set: ParamSet<(
        Query<&Transform, With<Lander>>,
        Query<(Entity, &mut Transform, &mut ExhaustParticle)>,
    )>,
    lander_state: Res<LanderState>,
) {
    // Update existing particles with the camera offset
    let mut to_despawn = Vec::new();
    {
        let mut particle_query = query_set.p1();
        for (entity, mut transform, mut particle) in particle_query.iter_mut() {
            particle.lifetime.tick(time.delta());
            if particle.lifetime.finished() {
                to_despawn.push(entity);
            } else {
                transform.translation.x += particle.velocity.x * time.delta_secs();
                transform.translation.y += particle.velocity.y * time.delta_secs();
                // Adjust particle position for camera movement
                transform.translation.x -= camera_state.target_offset.x;
                transform.translation.y -= camera_state.target_offset.y;
            }
        }
    }

    // Despawn finished particles
    for entity in to_despawn {
        commands.entity(entity).despawn();
    }

    if lander_state.thrust_level > 0.0 && !lander_state.landed && !lander_state.crashed {
        timer.0.tick(time.delta());
        if timer.0.just_finished() {
            let lander_query = query_set.p0();
            if let Ok(lander_transform) = lander_query.get_single() {
                let num_particles = (lander_state.thrust_level * 3.0) as i32;

                // When rotation is 0 (pointing up):
                //   - particles should come out the bottom
                //   - gimbal rotates this direction
                let exhaust_direction_angle =
                    lander_state.rotation + lander_state.gimbal_angle - std::f32::consts::FRAC_PI_2;

                // Spawn position should be at the base of the triangle
                let exhaust_direction =
                    Vec2::new(exhaust_direction_angle.cos(), exhaust_direction_angle.sin());
                let exhaust_offset =
                    exhaust_direction * LANDER_HEIGHT * 0.5 * WORLD_TO_SCREEN_SCALE;

                for _ in 0..num_particles {
                    spawn_particle(
                        &mut commands,
                        lander_transform.translation,
                        exhaust_offset,
                        exhaust_direction,
                    );
                }
            }
        }
    }
}
