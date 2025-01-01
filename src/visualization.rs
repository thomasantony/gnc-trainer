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

const PARTICLE_SPAWN_RATE: f32 = 0.01; // Spawn every 10ms
const PARTICLE_SIZE: f32 = 2.0; // Smaller particles
const PARTICLE_BASE_SPEED: f32 = 150.0; // Moderate speed for better visibility
const PARTICLE_SPREAD: f32 = 0.35; // Narrower spread angle (in radians)

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
pub struct Ground;

#[derive(Component)]
pub struct LandingZone;

#[derive(Component)]
pub struct ExhaustParticle {
    lifetime: Timer,
    velocity: Vec2,
}

#[derive(Component)]
pub struct GridSystem;

const GRID_SPACING: f32 = 10.0; // 10 meter spacing

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

    commands.spawn((
        GridSystem,
        Transform::default(),
        GlobalTransform::default(),
        Visibility::default(),
    ));

    commands.insert_resource(ParticleSpawnTimer(Timer::from_seconds(
        PARTICLE_SPAWN_RATE,
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
    mut lander_query: Query<&mut Transform, With<Lander>>,
    mut landing_zone_query: Query<&mut Transform, (With<LandingZone>, Without<Lander>)>,
    mut ground_query: Query<&mut Visibility, With<Ground>>,
    mut camera_state: ResMut<CameraState>,
    lander_state: Res<LanderState>,
    level: Res<CurrentLevel>,
) {
    // Calculate view offset based on lander position
    let offset = calculate_view_offset(lander_state.position);
    camera_state.target_offset = offset;

    // Update lander position
    if let Ok(mut transform) = lander_query.get_single_mut() {
        let screen_pos = world_to_screen(lander_state.position, offset);
        transform.translation.x = screen_pos.x;
        transform.translation.y = screen_pos.y;
        transform.rotation = Quat::from_rotation_z(lander_state.rotation);
    }

    // Update landing zone position
    if let Ok(mut transform) = landing_zone_query.get_single_mut() {
        let landing_zone_pos = Vec2::new(
            (level.config.success.x_min + level.config.success.x_max) / 2.0,
            0.0,
        );
        let screen_pos = world_to_screen(landing_zone_pos, camera_state.target_offset);
        transform.translation.x = screen_pos.x;
        transform.translation.y = screen_pos.y + 5.0;
    }

    // Update ground visibility
    if let Ok(mut visibility) = ground_query.get_single_mut() {
        *visibility = if lander_state.position.y * WORLD_TO_SCREEN_SCALE
            < MIN_VIEW_HEIGHT * WORLD_TO_SCREEN_SCALE
        {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
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

    // Calculate screen position accounting for camera offset once
    let screen_pos = world_to_screen(lander_pos + base_position + offset, camera_offset);

    commands.spawn((
        Sprite {
            color: ORANGE_RED.into(),
            custom_size: Some(Vec2::new(PARTICLE_SIZE, PARTICLE_SIZE)),
            ..default()
        },
        Transform::from_xyz(screen_pos.x, screen_pos.y, 0.5),
        ExhaustParticle {
            lifetime: Timer::from_seconds(PARTICLE_LIFETIME, TimerMode::Once),
            velocity: angle * speed,
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
    // Update existing particles
    let mut to_despawn = Vec::new();
    {
        let mut particle_query = query_set.p1();
        for (entity, mut transform, mut particle) in particle_query.iter_mut() {
            particle.lifetime.tick(time.delta());
            if particle.lifetime.finished() {
                to_despawn.push(entity);
            } else {
                // Update position based on velocity
                transform.translation.x += particle.velocity.x * time.delta_secs();
                transform.translation.y += particle.velocity.y * time.delta_secs();
            }
        }
    }

    // Despawn finished particles
    for entity in to_despawn {
        commands.entity(entity).despawn();
    }

    // Spawn new particles
    if lander_state.thrust_level > 0.0 && !lander_state.landed && !lander_state.crashed {
        timer.0.tick(time.delta());
        if timer.0.just_finished() {
            let num_particles = (lander_state.thrust_level * 5.0) as i32; // More particles

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
