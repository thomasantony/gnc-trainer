use crate::constants::{LANDER_HEIGHT, LANDER_WIDTH};
use crate::levels::{CurrentLevel, Reference};
use crate::simulation::LanderState;
use bevy::asset::RenderAssetUsages;
use bevy::color::palettes::css::*;
use bevy::prelude::*;

// Constants for view configuration
pub(crate) const WORLD_TO_SCREEN_SCALE: f32 = 10.0;
const RIGHT_PANEL_WIDTH: f32 = 600.0;
const GROUND_OFFSET: f32 = -200.0; // Pixels from center of screen to ground
const MIN_VIEW_HEIGHT: f32 = 30.0; // Minimum world height (in meters) visible in the view

#[derive(Resource)]
pub struct CameraState {
    pub following: bool,
    pub target_offset: Vec2,
    pub explosion_spawned: bool,
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
pub struct TargetZone;

#[derive(Component)]
pub struct GridSystem;

#[derive(Resource, Default)]
pub struct ResetVisibilityFlag(pub bool);

const GRID_SPACING: f32 = 10.0; // 10 meter spacing

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

#[derive(Resource, Default)]
pub struct ResetVisualization(pub bool);

pub fn reset_visualization_system(
    mut commands: Commands,
    mut reset_flag: ResMut<ResetVisualization>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    level: Res<CurrentLevel>,
    query: Query<Entity, With<LevelSpecific>>,
) {
    if reset_flag.0 {
        // Cleanup
        for entity in query.iter() {
            commands.entity(entity).despawn();
        }

        // Respawn
        spawn_visualization(commands, meshes, materials, level);

        reset_flag.0 = false;
    }
}

#[derive(Component)]
pub struct LevelSpecific;

pub fn spawn_visualization(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    level: Res<CurrentLevel>,
) {
    commands.insert_resource(CameraState::default());
    let center_offset = -(RIGHT_PANEL_WIDTH / 2.0);
    let config = &level.config;

    // Spawn ground
    let ground_width = 10000.0;
    commands.spawn((
        Sprite {
            color: Color::srgb(0.3, 0.3, 0.3),
            custom_size: Some(Vec2::new(ground_width, 200.0)),
            ..default()
        },
        Transform::from_xyz(
            center_offset + ground_width / 4.0,
            GROUND_OFFSET - 100.0,
            0.25,
        ),
        Ground,
        LevelSpecific,
    ));

    // Spawn success zone
    let initial_pos = Vec2::new(config.initial.x0, config.initial.y0);
    let screen_pos = world_to_screen(initial_pos, Vec2::ZERO);

    // Get dimensions from level config
    let zone_width = (config.success.position_box.x_max - config.success.position_box.x_min)
        * WORLD_TO_SCREEN_SCALE;
    let zone_height = (config.success.position_box.y_max - config.success.position_box.y_min)
        * WORLD_TO_SCREEN_SCALE;

    if let Reference::Initial = config.success.position_box.reference {
        // Hover-type target zone
        commands.spawn((
            Sprite {
                color: Color::srgba(0.0, 0.5, 0.0, 0.2),
                custom_size: Some(Vec2::new(zone_width.max(1.0), zone_height.max(1.0))),
                ..default()
            },
            Transform::from_xyz(screen_pos.x, screen_pos.y, 0.5),
            TargetZone,
            LevelSpecific,
        ));
    } else {
        // Landing strip
        commands.spawn((
            Sprite {
                color: Color::srgba(0.0, 0.5, 0.0, 0.2),
                custom_size: Some(Vec2::new(zone_width.max(1.0), 10.0)),
                ..default()
            },
            Transform::from_xyz(screen_pos.x, GROUND_OFFSET + 5.0, 0.5),
            TargetZone,
        ));
    }

    // // Spawn failure bounds if they exist
    // if let Some(bounds) = &config.failure.bounds {
    //     let bounds_width = (bounds.x_max - bounds.x_min) * WORLD_TO_SCREEN_SCALE;
    //     let bounds_height = (bounds.y_max - bounds.y_min) * WORLD_TO_SCREEN_SCALE;

    //     commands.spawn((
    //         Sprite {
    //             color: Color::srgba(0.8, 0.0, 0.0, 0.1),
    //             custom_size: Some(Vec2::new(bounds_width.max(1.0), bounds_height.max(1.0))),
    //             ..default()
    //         },
    //         Transform::from_xyz(screen_pos.x, screen_pos.y, 0.4),
    //         TargetZone,
    //     ));
    // }

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
        LevelSpecific,
    ));
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

pub(crate) fn world_to_screen(pos: Vec2, camera_offset: Vec2) -> Vec2 {
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
            Option<&TargetZone>,
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

    // Update ground and zone positions
    let mut ground_query = query_set.p1();
    for (mut transform, sprite, target_zone, ground) in ground_query.iter_mut() {
        if ground.is_some() {
            // Ground uses same position as landing zone
            let landing_center = Vec2::new(
                (level.config.success.position_box.x_min + level.config.success.position_box.x_max)
                    / 2.0,
                0.0,
            );
            let screen_pos = world_to_screen(landing_center, offset);
            transform.translation.x = screen_pos.x;
            // Center the ground block using its height
            if let Some(size) = sprite.custom_size {
                transform.translation.y = screen_pos.y - (size.y / 2.0);
            }
        } else if target_zone.is_some() {
            // Only update position if this is an absolute reference zone
            match level.config.success.position_box.reference {
                Reference::Absolute => {
                    // Landing zone centered on success zone
                    let landing_zone_pos = Vec2::new(
                        (level.config.success.position_box.x_min
                            + level.config.success.position_box.x_max)
                            / 2.0,
                        0.0,
                    );
                    let screen_pos = world_to_screen(landing_zone_pos, offset);
                    transform.translation.x = screen_pos.x;
                    transform.translation.y = screen_pos.y + 5.0; // Slight offset to stay above ground
                }
                Reference::Initial => {
                    // For hover target, track initial position
                    let initial_pos = Vec2::new(level.config.initial.x0, level.config.initial.y0);
                    let screen_pos = world_to_screen(initial_pos, offset);
                    transform.translation.x = screen_pos.x;
                    transform.translation.y = screen_pos.y;
                }
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
