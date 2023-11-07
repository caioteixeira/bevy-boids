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
                scale: Vec3::new(0.3, 0.3, 0.),
                ..Default::default()
            },
            ..default()
        },
        Velocity(Vec3::new(0., 0., 0.)),
        Acceleration(Vec3::new(0., 0., 0.)),
        Target(Vec3::new(100., 150., 0.)),
        MaxSpeed(4.),
        MaxForce(0.1),
    ));
}

// direct port of processing's map function
fn map(value: f32, start1: f32, stop1: f32, start2: f32, stop2: f32) -> f32 {
    start2 + (stop2 - start2) * ((value - start1) / (stop1 - start1))
}

fn clamp_magnitude(value: Vec3, max: f32) -> Vec3 {
    if value.length() > max {
        value.normalize() * max
    } else {
        value
    }
}

fn seek(mut query: Query<(&Transform, &Target, &mut Acceleration, &Velocity, &MaxSpeed, &MaxForce)>, mut gizmos: Gizmos) {
    for (transform, target, mut acceleration, velocity, max_speed, max_force) in query.iter_mut() {
        let location = transform.translation;
        let mut desired_velocity = target.0 - location;

        gizmos.circle_2d(Vec2::new(target.0.x, target.0.y), 10., Color::BLUE);

        let distance = desired_velocity.length();

        gizmos.line(location, location + desired_velocity, Color::GREEN);

        let target_radius = 100.;
        if distance < target_radius {
            let m = map(distance, 0., target_radius, 0., max_speed.0);
            desired_velocity = desired_velocity.normalize() * m;
        } else {
            desired_velocity = desired_velocity.normalize() * max_speed.0;
        }

        let mut steering = desired_velocity - velocity.0;
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

fn update_position(mut query: Query<(&mut Transform, &Velocity)>, mut gizmos : Gizmos) {
    for (mut transform, velocity) in query.iter_mut() {
        gizmos.line(transform.translation, transform.translation + velocity.0 * 100., Color::RED);

        transform.translation += Vec3::new(velocity.0.x, velocity.0.y, 0.);
        transform.rotation = Quat::from_rotation_z(velocity.0.y.atan2(velocity.0.x) + 180.);
    }
}
