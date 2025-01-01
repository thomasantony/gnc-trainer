use crate::constants::{LANDER_HEIGHT, LANDER_WIDTH};
use crate::levels::CurrentLevel;
use crate::simulation::LanderState;
use bevy::asset::RenderAssetUsages;
use bevy::color::palettes::css::*;
use bevy::prelude::*;
use rand::Rng;

// Constants for view configuration
const WORLD_TO_SCREEN_SCALE: f32 = 10.0;
const RIGHT_PANEL_WIDTH: f32 = 600.0;
const PARTICLE_LIFETIME: f32 = 0.5;
const GROUND_OFFSET: f32 = -200.0; // Pixels from center of screen to ground
const MIN_VIEW_HEIGHT: f32 = 30.0; // Minimum world height (in meters) visible in the view
const PARTICLE_SIZE: f32 = 2.0; // Smaller particles
const PARTICLE_BASE_SPEED: f32 = 150.0; // Moderate speed for better visibility
const PARTICLE_SPREAD: f32 = 0.30; // Spread angle (in radians)
const PARTICLE_COUNT_PER_SPAWN: i32 = 3; // Number of particles to spawn each time
const PARTICLE_BOUNCE_DAMPING: f32 = 0.1; // How much velocity is retained after bounce
const PARTICLE_GROUND_Y: f32 = 0.1; // Ground level for particles [m]
const EXPLOSION_PARTICLE_COUNT: usize = 50;
const EXPLOSION_PARTICLE_SPEED: f32 = 400.0;

#[derive(Resource)]
pub struct CameraState {
    pub following: bool,
    pub target_offset: Vec2,
    pub explosion_spawned: bool, // Add this
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            following: true,
            target_offset: Vec2::ZERO,
            explosion_spawned: false,
        }
    }
}

// Components
#[derive(Component)]
pub struct Lander;

#[derive(Component)]
pub struct Ground;

#[derive(Component)]
pub struct LandingZone;

#[derive(Component)]
pub struct ExhaustParticle {
    lifetime: Timer,
    velocity: Vec2,
    world_pos: Vec2,
}

#[derive(Component)]
pub struct GridSystem;

#[derive(Resource, Default)]
pub struct ResetVisibilityFlag(pub bool);

const GRID_SPACING: f32 = 10.0; // 10 meter spacing

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

    // Spawn ground - grey block extending to bottom
    // Calculate landing zone dimensions and position
    let landing_zone_width =
        (level.config.success.x_max - level.config.success.x_min) * WORLD_TO_SCREEN_SCALE;
    let landing_zone_x =
        (level.config.success.x_min + level.config.success.x_max) / 2.0 * WORLD_TO_SCREEN_SCALE;

    // Spawn ground - grey block matching landing zone width and position
    commands.spawn((
        Sprite {
            color: Color::srgb(0.3, 0.3, 0.3),
            custom_size: Some(Vec2::new(landing_zone_width, 200.0)), // Same width as landing zone
            ..default()
        },
        Transform::from_xyz(
            center_offset + landing_zone_x, // Same x-position as landing zone
            GROUND_OFFSET - 100.0,          // Centered below ground level
            0.25,                           // Between background and landing zone
        ),
        Ground,
    ));

    // Spawn landing zone
    commands.spawn((
        Sprite {
            color: Color::srgba(0.0, 0.5, 0.0, 0.3),
            custom_size: Some(Vec2::new(landing_zone_width, 10.0)),
            ..default()
        },
        Transform::from_xyz(center_offset + landing_zone_x, GROUND_OFFSET + 5.0, 0.5),
        LandingZone,
    ));

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

    commands.insert_resource(ParticleSpawnTimer(Timer::from_seconds(
        0.05,
        TimerMode::Repeating,
    )));
}

// Add new system for updating grid lines
pub fn update_grid_lines(
    mut commands: Commands,
    grid_query: Query<Entity, With<GridSystem>>,
    camera_state: Res<CameraState>,
    lander_state: Res<LanderState>,
) {
    // Get the grid parent entity, or create one if it doesn't exist
    let grid_entity = if let Some(entity) = grid_query.iter().next() {
        // If grid exists, despawn all its children
        commands.entity(entity).despawn_descendants();
        entity
    } else {
        // Create new grid parent if none exists
        commands
            .spawn((
                GridSystem,
                Transform::default(),
                GlobalTransform::default(),
                Visibility::default(),
            ))
            .id()
    };

    // Work in world coordinates first
    let view_center = lander_state.position;
    let num_lines = 10; // Number of grid spacings to extend in each direction from center

    // Calculate world-space bounds centered on spacecraft
    let line_length = GRID_SPACING * num_lines as f32;

    // Calculate grid line positions in world space
    let start_x = ((view_center.x - line_length) / GRID_SPACING).floor() * GRID_SPACING;
    let end_x = ((view_center.x + line_length) / GRID_SPACING).ceil() * GRID_SPACING;
    let start_y = ((view_center.y - line_length) / GRID_SPACING).floor() * GRID_SPACING;
    let end_y = ((view_center.y + line_length) / GRID_SPACING).ceil() * GRID_SPACING;

    // Calculate world height for vertical lines (based on lander position)
    let vertical_world_height = line_length * 2.0; // Same scale as width
    let vertical_screen_height = vertical_world_height * WORLD_TO_SCREEN_SCALE;

    // Calculate world width for horizontal lines (based on lander position)
    let horizontal_world_width = line_length * 2.0;
    let horizontal_screen_width = horizontal_world_width * WORLD_TO_SCREEN_SCALE;

    // Spawn vertical lines
    let mut x = start_x;
    while x <= end_x {
        // Convert world X to screen X
        let screen_pos = world_to_screen(
            Vec2::new(x, lander_state.position.y),
            camera_state.target_offset,
        );

        commands
            .spawn((
                Sprite {
                    color: Color::srgba(0.5, 0.5, 0.5, 0.2),
                    custom_size: Some(Vec2::new(1.0, vertical_screen_height)),
                    ..default()
                },
                Transform::from_xyz(screen_pos.x, screen_pos.y, 0.1),
                GlobalTransform::default(),
                Visibility::default(),
                GridSystem,
            ))
            .set_parent(grid_entity);
        x += GRID_SPACING;
    }

    // Spawn horizontal lines
    let mut y = start_y;
    while y <= end_y {
        let screen_pos = world_to_screen(
            Vec2::new(lander_state.position.x, y),
            camera_state.target_offset,
        );
        commands
            .spawn((
                Sprite {
                    color: Color::srgba(0.5, 0.5, 0.5, 0.2),
                    custom_size: Some(Vec2::new(horizontal_screen_width, 1.0)),
                    ..default()
                },
                Transform::from_xyz(screen_pos.x, screen_pos.y, 0.1),
                GlobalTransform::default(),
                Visibility::default(),
                GridSystem,
            ))
            .set_parent(grid_entity);
        y += GRID_SPACING;
    }
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
    mut query_set: ParamSet<(
        Query<&mut Transform, With<Lander>>,
        Query<(
            &mut Transform,
            &Sprite,
            Option<&LandingZone>,
            Option<&Ground>,
        )>,
    )>,
    mut camera_state: ResMut<CameraState>,
    lander_state: Res<LanderState>,
    level: Res<CurrentLevel>,
) {
    // Calculate view offset based on lander position
    let offset = calculate_view_offset(lander_state.position);
    camera_state.target_offset = offset;

    // Update lander position
    if let Ok(mut transform) = query_set.p0().get_single_mut() {
        let screen_pos = world_to_screen(lander_state.position, offset);
        transform.translation.x = screen_pos.x;
        transform.translation.y = screen_pos.y;
        transform.rotation = Quat::from_rotation_z(lander_state.rotation);
    }

    // Update ground and landing zone positions
    let mut ground_query = query_set.p1();
    for (mut transform, sprite, landing_zone, ground) in ground_query.iter_mut() {
        if ground.is_some() {
            // Ground uses same position as landing zone
            let landing_center = Vec2::new(
                (level.config.success.x_min + level.config.success.x_max) / 2.0,
                0.0,
            );
            let screen_pos = world_to_screen(landing_center, offset);
            transform.translation.x = screen_pos.x;
            // Center the ground block using its height
            if let Some(size) = sprite.custom_size {
                transform.translation.y = screen_pos.y - (size.y / 2.0);
            }
        } else if landing_zone.is_some() {
            // Landing zone centered on success zone
            let landing_zone_pos = Vec2::new(
                (level.config.success.x_min + level.config.success.x_max) / 2.0,
                0.0,
            );
            let screen_pos = world_to_screen(landing_zone_pos, offset);
            transform.translation.x = screen_pos.x;
            transform.translation.y = screen_pos.y + 5.0; // Slight offset to stay above ground
        }
    }
}

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

    // Smaller spread at emission point
    let offset = Vec2::new(rng.gen_range(-0.2..0.2), rng.gen_range(0.0..0.5));

    // Calculate world position for the particle
    let world_pos = lander_pos + base_position + offset;

    // Calculate screen position accounting for camera offset
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
    ));
}

pub fn particle_system(
    mut commands: Commands,
    time: Res<Time>,
    mut timer: ResMut<ParticleSpawnTimer>,
    mut camera_state: ResMut<CameraState>,
    mut query_set: ParamSet<(
        Query<(Entity, &Transform, &mut Visibility), With<Lander>>,
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
                // Update screen and world positions
                let delta = particle.velocity * dt;
                transform.translation.x += delta.x;
                transform.translation.y += delta.y;
                particle.world_pos += delta / WORLD_TO_SCREEN_SCALE;

                // Check for ground collision
                if particle.world_pos.y <= PARTICLE_GROUND_Y {
                    // Bounce
                    particle.world_pos.y = PARTICLE_GROUND_Y;
                    particle.velocity.y = -particle.velocity.y * PARTICLE_BOUNCE_DAMPING;
                    particle.velocity.x *= 0.9; // Some horizontal damping

                    // If moving very slowly after bounce, despawn
                    if particle.velocity.length() < 20.0 {
                        to_despawn.push(entity);
                    }
                }
            }
        }
    }

    // Despawn finished particles
    for entity in to_despawn {
        commands.entity(entity).despawn();
    }

    // Handle crash explosion
    if lander_state.crashed && !camera_state.explosion_spawned {
        let mut lander_query = query_set.p0();

        if let Ok((_entity, lander_transform, mut visibility)) = lander_query.get_single_mut() {
            // Hide the lander
            *visibility = Visibility::Hidden;

            let mut rng = rand::thread_rng();

            // Spawn explosion particles in a circle
            for i in 0..EXPLOSION_PARTICLE_COUNT {
                let angle = (i as f32 / EXPLOSION_PARTICLE_COUNT as f32) * std::f32::consts::TAU;
                let direction = Vec2::new(angle.cos(), angle.sin());

                let velocity = EXPLOSION_PARTICLE_SPEED * direction * rng.gen_range(0.8..1.2);
                // Spawn larger, faster particles for explosion
                commands.spawn((
                    Sprite {
                        color: Color::srgb(1.0, 0.5, 0.0),
                        custom_size: Some(Vec2::new(4.0, 4.0)),
                        ..default()
                    },
                    Transform::from_translation(lander_transform.translation),
                    ExhaustParticle {
                        lifetime: Timer::from_seconds(1.0, TimerMode::Once),
                        velocity: velocity,
                        world_pos: lander_state.position,
                    },
                ));
            }
            camera_state.explosion_spawned = true;
        }
    }
    // Spawn new particles
    if lander_state.thrust_level > 0.0 && !lander_state.landed && !lander_state.crashed {
        timer.0.tick(time.delta());
        if timer.0.just_finished() {
            let num_particles =
                (lander_state.thrust_level * PARTICLE_COUNT_PER_SPAWN as f32) as i32;

            // When rotation is 0 (pointing up):
            //   - particles should come out the bottom
            //   - gimbal rotates this direction
            let exhaust_angle =
                lander_state.rotation + lander_state.gimbal_angle + std::f32::consts::FRAC_PI_2;
            let exhaust_direction = -Vec2::new(exhaust_angle.cos(), exhaust_angle.sin());

            // Calculate base position (at the bottom of the lander)
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

pub fn reset_lander_visibility(
    mut lander_query: Query<&mut Visibility, With<Lander>>,
    mut reset_flag: ResMut<ResetVisibilityFlag>,
) {
    if reset_flag.0 {
        if let Ok(mut visibility) = lander_query.get_single_mut() {
            *visibility = Visibility::Visible;
        }
        reset_flag.0 = false; // This is correct - we want to reset it after handling
    }
}
