use crate::simulation::LanderState;
use bevy::asset::RenderAssetUsages;
use bevy::color::palettes::css::PURPLE;
use bevy::prelude::*;
use rand::Rng;

// Constants for view configuration
const WORLD_TO_SCREEN_SCALE: f32 = 4.0;
const VISUALIZATION_WIDTH: f32 = 680.0;
const RIGHT_PANEL_WIDTH: f32 = 600.0;
const PARTICLE_LIFETIME: f32 = 0.5;

// Components
#[derive(Component)]
pub struct Lander;

#[derive(Component)]
struct Ground;

#[derive(Component)]
pub struct ExhaustParticle {
    lifetime: Timer,
}

// Resource to manage particle spawning
#[derive(Resource)]
pub struct ParticleSpawnTimer(Timer);

// Create a triangle mesh for the lander
fn create_triangle_mesh() -> Mesh {
    let mut mesh = Mesh::new(
        bevy::render::render_resource::PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );

    let vertices = [
        [0.0, 10.0, 0.0],  // top
        [-7.0, -7.0, 0.0], // bottom left
        [7.0, -7.0, 0.0],  // bottom right
    ];
    let indices = [0u32, 1, 2]; // Counter-clockwise order
    let normals = [[0.0, 0.0, 1.0]; 3];
    let uvs = [[0.5, 0.0], [0.0, 1.0], [1.0, 1.0]];

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices.to_vec());
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals.to_vec());
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs.to_vec());
    mesh.insert_indices(bevy::render::mesh::Indices::U32(indices.to_vec()));
    mesh
}

// Spawn the visualization entities
pub fn spawn_visualization(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let center_offset = -(RIGHT_PANEL_WIDTH / 2.0);

    // Spawn lander as a solid triangle
    commands.spawn((
        Mesh2d(meshes.add(create_triangle_mesh())),
        MeshMaterial2d(materials.add(Color::from(PURPLE))),
        Transform::from_xyz(center_offset, 0.0, 1.0),
        Lander,
    ));

    // Spawn ground
    commands.spawn((
        Sprite {
            color: Color::srgb(0.0, 0.8, 0.0), // Green
            custom_size: Some(Vec2::new(VISUALIZATION_WIDTH, 2.0)),
            ..default()
        },
        Transform::from_xyz(center_offset, 0.0, 0.0),
        GlobalTransform::default(),
        Visibility::default(),
        InheritedVisibility::default(),
        ViewVisibility::default(),
        Ground,
    ));

    // Initialize particle spawn timer
    commands.insert_resource(ParticleSpawnTimer(Timer::from_seconds(
        0.05,
        TimerMode::Repeating,
    )));
}

pub fn update_visualization(
    mut lander_query: Query<&mut Transform, With<Lander>>,
    lander_state: Res<LanderState>,
) {
    if let Ok(mut transform) = lander_query.get_single_mut() {
        let screen_pos = world_to_screen(lander_state.position);
        transform.translation.x = screen_pos.x;
        transform.translation.y = screen_pos.y;
    }
}

pub fn particle_system(
    mut commands: Commands,
    time: Res<Time>,
    mut timer: ResMut<ParticleSpawnTimer>,
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
                // Move particle downward and add some random horizontal movement
                transform.translation.y -= 100.0 * time.delta().as_secs_f32();
                transform.translation.x += rand::thread_rng().gen_range(-0.5..0.5);
            }
        }
    }

    // Despawn finished particles
    for entity in to_despawn {
        commands.entity(entity).despawn();
    }

    // Spawn new particles based on thrust level
    if lander_state.thrust_level > 0.0 {
        timer.0.tick(time.delta());
        if timer.0.just_finished() {
            let lander_query = query_set.p0();
            if let Ok(lander_transform) = lander_query.get_single() {
                let num_particles = (lander_state.thrust_level * 3.0) as i32;
                for _ in 0..num_particles {
                    spawn_particle(&mut commands, lander_transform.translation);
                }
            }
        }
    }
}

fn spawn_particle(commands: &mut Commands, lander_pos: Vec3) {
    let mut rng = rand::thread_rng();
    let offset = Vec2::new(rng.gen_range(-3.0..3.0), rng.gen_range(-2.0..0.0));

    commands.spawn((
        Sprite {
            color: Color::srgb(1.0, 0.6, 0.0), // Orange
            custom_size: Some(Vec2::new(2.0, 2.0)),
            ..default()
        },
        Transform::from_xyz(lander_pos.x + offset.x, lander_pos.y + offset.y, 0.5),
        GlobalTransform::default(),
        ExhaustParticle {
            lifetime: Timer::from_seconds(PARTICLE_LIFETIME, TimerMode::Once),
        },
        Visibility::default(),
        InheritedVisibility::default(),
        ViewVisibility::default(),
    ));
}

// Convert simulation coordinates to screen coordinates
fn world_to_screen(pos: Vec2) -> Vec2 {
    let center_offset = -(RIGHT_PANEL_WIDTH / 2.0);
    Vec2::new(
        pos.x * WORLD_TO_SCREEN_SCALE + center_offset,
        pos.y * WORLD_TO_SCREEN_SCALE,
    )
}
