// Based on: https://natureofcode.com/book/chapter-6-autonomous-agents/

// Bevy code commonly triggers these lints and they may be important signals
// about code quality. They are sometimes hard to avoid though, and the CI
// workflow treats them as errors, so this allows them throughout the project.
// Feel free to delete this line.
#![allow(clippy::too_many_arguments, clippy::type_complexity)]

use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    sprite::{MaterialMesh2dBundle, Mesh2dHandle},
    window::{PresentMode, WindowMode},
};

use boids_plugin::{BoidBundle, BoidsPlugin};
use flow_field_plugin::FlowFieldFollower;
use rand::Rng;

pub mod boids_plugin;
pub mod flow_field_plugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy Boids".into(),
                mode: WindowMode::Windowed,
                present_mode: PresentMode::AutoNoVsync,

                // Tells wasm to resize the window according to the available canvas
                fit_canvas_to_parent: true,
                // Tells wasm not to override default event handling, like F5, Ctrl+R etc.
                prevent_default_event_handling: false,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(LogDiagnosticsPlugin::default())
        .add_plugins(FrameTimeDiagnosticsPlugin)
        .add_plugins(BoidsPlugin)
        //.add_plugins(FlowFieldPlugin)
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn(Camera2dBundle::default());

    let material_handle = materials.add(ColorMaterial::from(Color::GREEN));
    let mesh_handle: Mesh2dHandle = meshes.add(shape::Circle::new(2.).into()).into();
    let mut rng = rand::thread_rng();

    for _i in 0..4000 {
        commands.spawn((
            MaterialMesh2dBundle {
                mesh: mesh_handle.clone(),
                material: material_handle.clone(),
                transform: Transform {
                    translation: Vec3::new(
                        rng.gen_range(-1000.0..1000.0),
                        rng.gen_range(-800.0..800.0),
                        0.,
                    ),
                    ..Default::default()
                },
                ..default()
            },
            BoidBundle::default(),
            FlowFieldFollower,
        ));
    }
}
