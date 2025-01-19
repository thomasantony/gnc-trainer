use crate::{ui::GameState, GridCellType};
use bevy::{
    input::mouse::{MouseMotion, MouseScrollUnit, MouseWheel},
    math::DVec3,
    pbr::{CascadeShadowConfigBuilder, NotShadowCaster},
    prelude::*,
};
use bevy_panorbit_camera::PanOrbitCameraPlugin;
use big_space::{
    commands::BigSpaceCommands,
    plugin::BigSpacePlugin,
    prelude::{GridCell, GridCommands, GridTransform, Grids},
};
use camera::{camera_inputs, spawn_camera};
use std::f32::consts::TAU;

mod camera;

// Constants matching surveyor_gfx implementation
const MOON_RADIUS: f32 = 1737.1e3; // meters
const EARTH_RADIUS: f32 = 6378.14e3; // meters
const INITIAL_ALTITUDE: f32 = 100e3; // 100km initial orbit
const SUN_RADIUS_M: f64 = 695_508_000_f64;
const EARTH_ORBIT_RADIUS_M: f64 = 149.60e9;
const EARTH_MOON_DIST_M: f64 = 384_400_000_f64;

const LANDER_X: f64 = MOON_RADIUS as f64 + INITIAL_ALTITUDE as f64;
const LANDER_Y: f64 = 0.0;
const LANDER_Z: f64 = 0.0;

#[derive(Component)]
pub struct Spacecraft3d;

#[derive(Component)]
pub struct FollowCamera {
    pub focus: Vec3,
    pub alpha: f32,  // horizontal rotation
    pub beta: f32,   // vertical rotation
    pub radius: f32, // distance from focus
    pub is_upside_down: bool,
}

impl Default for FollowCamera {
    fn default() -> Self {
        Self {
            focus: Vec3::ZERO,
            alpha: 0.0,
            beta: 0.0,
            radius: 10.0,
            is_upside_down: false,
        }
    }
}

#[derive(Component)]
pub struct CelBody(pub CelestialBodyType);

#[derive(Debug, Eq, PartialEq, Component)]
pub enum CelestialBodyType {
    Moon,
    Earth,
    Sun,
}

// This event contains the internal state of the lander computed  by "update_lander_state_from_simulation"
// This will be used by downstream systems to update the graphics and camera
#[derive(Event)]
pub struct SpacecraftStateUpdate {
    pub pos: Vec3,
    pub vel: Vec3,
    pub quat: Quat,
}

pub struct Visualization3dPlugin;

impl Plugin for Visualization3dPlugin {
    fn build(&self, app: &mut App) {
        // app.add_systems(OnEnter(GameState::ThreeDViz), setup_3d_scene)
        app.add_systems(Startup, (setup_3d_scene))
            .add_plugins(BigSpacePlugin::<GridCellType>::new(true))
            .insert_resource(ClearColor(Color::srgb(0.0, 0.0, 0.0)))
            .add_event::<SpacecraftStateUpdate>()
            .add_systems(Update, (camera_inputs,))
            .add_systems(
                Update,
                (
                    update_celestial_bodies,
                    render_lander_state,
                    camera::sync_camera,
                )
                    .run_if(in_state(GameState::ThreeDViz)),
            )
            .add_plugins(PanOrbitCameraPlugin);
    }
}

pub fn spawn_lander(commands: &mut GridCommands<GridCellType>, asset_server: Res<AssetServer>) {
    let lander_pos = DVec3::new(LANDER_X, LANDER_Y, LANDER_Z);

    let (lander_cell, lander_pos) = commands.grid().translation_to_grid(lander_pos);
    // let (grid_cell, lander_translation) = settings.translation_to_grid(lander_pos);
    commands.spawn_spatial((
        SceneRoot(
            asset_server.load(GltfAssetLabel::Scene(0).from_asset("Surveyor/Surveyor-Lander.gltf")),
        ),
        Transform::from_translation(lander_pos.clone()),
        lander_cell,
        Spacecraft3d,
    ));
    println!("Lander Spawned")
}

fn setup_3d_scene(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    commands.spawn((
        DirectionalLight {
            color: Color::WHITE,
            illuminance: 120_000.,
            shadows_enabled: true,
            ..default()
        },
        CascadeShadowConfigBuilder {
            num_cascades: 4,
            minimum_distance: 0.1,
            maximum_distance: 10_000.0,
            first_cascade_far_bound: 100.0,
            overlap_proportion: 0.2,
        }
        .build(),
    ));

    let sun_mesh_handle = meshes.add(Sphere::new(SUN_RADIUS_M as f32).mesh().ico(6).unwrap());

    commands.spawn_big_space_default::<GridCellType>(|root| {
        // Add sun first

        let sun_pos = DVec3::Z * (EARTH_MOON_DIST_M + EARTH_ORBIT_RADIUS_M);
        let (sun_cell, sun_pos) = root.grid().translation_to_grid(sun_pos);

        root.insert((CelestialBodyType::Sun, Name::new("Sun")));
        root.spawn_spatial((
            Mesh3d(sun_mesh_handle),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::WHITE,
                emissive: LinearRgba::rgb(1000., 1000., 1000.),
                ..default()
            })),
            Transform::from_translation(sun_pos),
            sun_cell,
            NotShadowCaster,
        ));

        // Earth
        let earth_pos = DVec3::X * EARTH_ORBIT_RADIUS_M;
        let (earth_cell, earth_pos) = root.grid().translation_to_grid(earth_pos);
        let earth_mesh_handle = meshes.add(Sphere::new(EARTH_RADIUS).mesh().uv(32, 18));

        root.insert((CelestialBodyType::Earth, Name::new("Earth")));
        root.spawn_spatial((
            Mesh3d(earth_mesh_handle),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::WHITE,
                emissive: LinearRgba::rgb(1000., 1000., 1000.),
                ..default()
            })),
            Transform::from_translation(earth_pos),
            earth_cell,
        ));

        // Moon
        let moon_pos = DVec3::ZERO;
        let (moon_cell, moon_pos) = root.grid().translation_to_grid(moon_pos);
        let moon_mesh_handle = meshes.add(Sphere::new(MOON_RADIUS as f32).mesh().uv(64, 180));

        let moon_material = materials.add(StandardMaterial {
            base_color_texture: Some(asset_server.load("textures/moon/base_color.jpg")),
            emissive: Color::srgb_u8(30, 30, 30).to_linear(),
            ..default()
        });
        root.insert((CelestialBodyType::Moon, Name::new("Moon")));
        root.spawn_spatial((
            Mesh3d(moon_mesh_handle),
            MeshMaterial3d(moon_material),
            Transform::from_translation(moon_pos).with_rotation(Quat::from_euler(
                EulerRot::XYZ,
                -TAU / 4.0_f32,
                0.0,
                TAU / 2.0_f32,
            )),
            moon_cell,
        ));

        // Earth
        spawn_lander(root, asset_server);
        spawn_camera(root);
    });
}

fn update_celestial_bodies(time: Res<Time>, mut event_writer: EventWriter<SpacecraftStateUpdate>) {
    // Basic orbit for testing
    let orbit_period = 120.0; // 2 minutes per orbit

    let slowdown = 0.1;

    let angle = (time.elapsed_secs() / orbit_period) * TAU as f32 * slowdown;
    let radius = MOON_RADIUS as f32 + INITIAL_ALTITUDE as f32;

    let new_translation = Vec3::new(radius * angle.cos(), 0.0, radius * angle.sin());

    // Send event with the new state
    let event = SpacecraftStateUpdate {
        pos: new_translation,
        vel: Vec3::ZERO,
        quat: Quat::IDENTITY,
    };
    event_writer.send(event);
}

pub fn render_lander_state(
    mut sc_state: EventReader<SpacecraftStateUpdate>,
    mut sc_query: Query<(Entity, GridTransform<GridCellType>), With<Spacecraft3d>>,
    grids: Grids<GridCellType>,
) {
    if let Some(sc_state) = sc_state.read().last() {
        let (sc, mut grid_transform) = sc_query.single_mut();

        let Some(grid) = grids.parent_grid(sc) else {
            return;
        };

        let (new_cell, new_pos) = grid.translation_to_grid(sc_state.pos);
        grid_transform.transform.translation = new_pos;
        *grid_transform.cell = new_cell;
        grid_transform.transform.rotation = sc_state.quat;
    }
}
