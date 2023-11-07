// Based on: https://natureofcode.com/book/chapter-6-autonomous-agents/

// Bevy code commonly triggers these lints and they may be important signals
// about code quality. They are sometimes hard to avoid though, and the CI
// workflow treats them as errors, so this allows them throughout the project.
// Feel free to delete this line.
#![allow(clippy::too_many_arguments, clippy::type_complexity)]

use bevy::prelude::*;

#[derive(Component)]
struct Velocity(Vec2);

#[derive(Component)]
struct Acceleration(Vec2);

#[derive(Component)]
struct Target(Vec2);

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
                scale: Vec3::new(0.3, 0.3, 0.),
                ..Default::default()
            },
            ..default()
        },
        Velocity(Vec2::new(0., 0.)),
        Acceleration(Vec2::new(0., 0.)),
        Target(Vec2::new(100., 150.)),
        MaxSpeed(0.2),
        MaxForce(0.01),
    ));
}

// direct port of processing's map function
fn map(value: f32, start1: f32, stop1: f32, start2: f32, stop2: f32) -> f32 {
    start2 + (stop2 - start2) * ((value - start1) / (stop1 - start1))
}

fn clamp_magnitude(value: Vec2, max: f32) -> Vec2 {
    if value.length() > max {
        value.normalize() * max
    } else {
        value
    }
}

fn seek(mut query: Query<(&Transform, &Target, &mut Acceleration, &MaxSpeed, &MaxForce)>) {
    for (transform, target, mut acceleration, max_speed, max_force) in query.iter_mut() {
        let location = Vec2::new(transform.translation.x, transform.translation.y);
        let mut desired_velocity = target.0 - location;
        info!("Target: {}", target.0);
        info!("Position: {}", location);
        info!("Desired Velocity: {}", desired_velocity);
        let distance = desired_velocity.length();
        info!("Distance: {}", distance);

        let target_radius = 100.;
        if distance < target_radius {
            let m = map(distance, 0., target_radius, 0., max_speed.0);
            info!("m: {}", m);
            desired_velocity = desired_velocity.normalize() * m;
        } else {
            info!("max speed {}", max_speed.0);
            desired_velocity = desired_velocity.normalize() * max_speed.0;
        }

        let mut steering = desired_velocity - location;
        steering = clamp_magnitude(steering, max_force.0);

        acceleration.0 += steering;
    }
}

fn apply_acceleration(mut query: Query<(&mut Velocity, &mut Acceleration, &MaxSpeed)>) {
    for (mut velocity, mut acceleration, max_speed) in query.iter_mut() {
        velocity.0 += acceleration.0;
        velocity.0 = clamp_magnitude(velocity.0, max_speed.0);

        acceleration.0 *= 0.;
    }
}

fn update_position(mut query: Query<(&mut Transform, &Velocity)>) {
    for (mut transform, velocity) in query.iter_mut() {
        info!("Position: {}", transform.translation);
        info!("Velocity: {}", velocity.0);

        transform.translation += Vec3::new(velocity.0.x, velocity.0.y, 0.);
        transform.rotation = Quat::from_rotation_z(velocity.0.y.atan2(velocity.0.x) + 180.);
    }
}
