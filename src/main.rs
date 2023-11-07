// Based on: https://natureofcode.com/book/chapter-6-autonomous-agents/

// Bevy code commonly triggers these lints and they may be important signals
// about code quality. They are sometimes hard to avoid though, and the CI
// workflow treats them as errors, so this allows them throughout the project.
// Feel free to delete this line.
#![allow(clippy::too_many_arguments, clippy::type_complexity)]

use bevy::prelude::*;

#[derive(Component)]
struct Velocity(Vec3);

#[derive(Component)]
struct Acceleration(Vec3);

#[derive(Component)]
struct Target(Vec3);

#[derive(Component)]
struct MaxSpeed(f32);

#[derive(Component)]
struct MaxForce(f32);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, seek)
        .add_systems(Update, apply_acceleration.after(seek))
        .add_systems(Update, update_position.after(apply_acceleration))
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());

    let ship_texture_handle = asset_server.load("ship_C.png");
    commands.spawn((
        SpriteBundle {
            texture: ship_texture_handle.clone(),
            transform: Transform {
                translation: Vec3::new(80., 50., 0.),
                scale: Vec3::new(0.3, 0.3, 0.3),
                ..Default::default()
            },
            ..default()
        },
        Velocity(Vec3::new(0., 0., 0.)),
        Acceleration(Vec3::new(0., 0., 0.)),
        Target(Vec3::new(10., 50., 0.)),
        MaxSpeed(10.),
        MaxForce(1.),
    ));

    commands.spawn((
        SpriteBundle {
            texture: ship_texture_handle.clone(),
            transform: Transform {
                translation: Vec3::new(0., 0., 0.),
                scale: Vec3::new(0.3, 0.3, 0.3),
                ..Default::default()
            },
            ..default()
        },
        Velocity(Vec3::new(0., 0., 0.)),
        Acceleration(Vec3::new(0., 0., 0.)),
        Target(Vec3::new(10., 15., 0.)),
        MaxSpeed(10.),
        MaxForce(1.),
    ));
}

fn seek(mut query: Query<(&Transform, &Target, &mut Acceleration, &MaxSpeed, &MaxForce)>) {
    for (transform, target, mut acceleration, max_speed, max_force) in query.iter_mut() {
        let mut desired_velocity = target.0 - transform.translation;
        desired_velocity = desired_velocity.normalize() * max_speed.0;

        let mut steering = desired_velocity - transform.translation;
        steering = steering.normalize() * max_force.0;

        acceleration.0 += steering;
    }
}

fn apply_acceleration(mut query: Query<(&mut Velocity, &Acceleration, &MaxSpeed)>) {
    for (mut velocity, acceleration, max_speed) in query.iter_mut() {
        velocity.0 += acceleration.0;
        velocity.0 = velocity.0.normalize() * max_speed.0;
    }
}

fn update_position(mut query: Query<(&mut Transform, &Velocity)>) {
    for (mut transform, velocity) in query.iter_mut() {
        transform.translation += velocity.0;
        transform.rotation = Quat::from_rotation_z(velocity.0.y.atan2(velocity.0.x) + 180.);
        //info!("Position: {}", transform.translation);
    }
}
