// Bevy code commonly triggers these lints and they may be important signals
// about code quality. They are sometimes hard to avoid though, and the CI
// workflow treats them as errors, so this allows them throughout the project.
// Feel free to delete this line.
#![allow(clippy::too_many_arguments, clippy::type_complexity)]

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());

    let ship_texture_handle = asset_server.load("ship_C.png");
    commands.spawn(SpriteBundle {
        texture: ship_texture_handle.clone(),
        transform: Transform {
            translation: Vec3::new(0., 0., 0.),
            scale: Vec3::new(0.3, 0.3, 0.),
            ..Default::default()
        },
        ..default()
    });
}
