// Based on: https://natureofcode.com/book/chapter-6-autonomous-agents/

// Bevy code commonly triggers these lints and they may be important signals
// about code quality. They are sometimes hard to avoid though, and the CI
// workflow treats them as errors, so this allows them throughout the project.
// Feel free to delete this line.
#![allow(clippy::too_many_arguments, clippy::type_complexity)]

use bevy::{prelude::*, sprite::MaterialMesh2dBundle, window::WindowMode};
use rand::Rng;

#[derive(Resource)]
struct FlowField {
    field: Vec<Vec<Vec3>>,
    resolution: f32,
}

#[derive(Resource)]
struct ForceMultipliers {
    seek: f32,
    separation: f32,
    alignment: f32,
    cohesion: f32,
}

impl FlowField {
    fn get_desired_velocity_for_world_position(
        &self,
        camera: &Camera,
        camera_transform: &GlobalTransform,
        world_position: Vec3,
    ) -> Vec3 {
        match camera.world_to_viewport(camera_transform, world_position) {
            Some(screen_position) => {
                let x = screen_position.x as usize / self.resolution as usize;
                let clamped_x = x.clamp(0, self.field.len() - 1);
                let y = screen_position.y as usize / self.resolution as usize;
                let clamped_y = y.clamp(0, self.field[0].len() - 1);

                self.field[clamped_x][clamped_y]
            }
            None => {
                warn! {
                    "world position: {:?} is not in view", world_position
                }
                Vec3::new(0., 0., 0.)
            }
        }
    }
}

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

#[derive(Component)]
struct FlowFieldFollower;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy Boids".into(),
                mode: WindowMode::Windowed,

                // Tells wasm to resize the window according to the available canvas
                fit_canvas_to_parent: true,
                // Tells wasm not to override default event handling, like F5, Ctrl+R etc.
                prevent_default_event_handling: false,
                ..default()
            }),
            ..default()
        }))
        .insert_resource(FlowField {
            field: Vec::new(),
            resolution: 5.,
        })
        .insert_resource(ForceMultipliers {
            seek: 0.5,
            separation: 1.5,
            alignment: 1.0,
            cohesion: 1.0,
        })
        .add_systems(Startup, setup)
        .add_systems(PreUpdate, update_target_with_mouse_pos)
        .add_systems(PreUpdate, wrap_around_screen)
        .add_systems(Update, compute_flow_field)
        .add_systems(Update, seek_flow_field.after(compute_flow_field))
        .add_systems(Update, separate)
        .add_systems(Update, align)
        .add_systems(Update, cohesion)
        .add_systems(Update, apply_acceleration)
        .add_systems(Update, update_position)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn(Camera2dBundle::default());

    let material_handle = materials.add(ColorMaterial::from(Color::GREEN));

    let mut rng = rand::thread_rng();

    commands.spawn(Target(Vec3::new(0., 0., 0.)));

    for _i in 0..2000 {
        commands.spawn((
            MaterialMesh2dBundle {
                mesh: meshes.add(shape::Circle::new(2.).into()).into(),
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
            Velocity(Vec3::new(0., 0., 0.)),
            Acceleration(Vec3::new(0., 0., 0.)),
            MaxSpeed(4.),
            MaxForce(0.1),
            FlowFieldFollower,
        ));
    }
}

fn update_target_with_mouse_pos(
    mut target_query: Query<&mut Target>,
    mut mouse_motion_events: EventReader<CursorMoved>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
) {
    let (camera, camera_transform) = camera_query.single();

    for event in mouse_motion_events.read() {
        let raw_world_position = camera
            .viewport_to_world_2d(camera_transform, event.position)
            .unwrap();

        for mut target in target_query.iter_mut() {
            target.0 = Vec3::new(raw_world_position.x, raw_world_position.y, 0.);
        }
    }
}

fn wrap_around_screen(
    mut query: Query<(&mut Transform, &mut Velocity)>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
) {
    let (camera, _) = camera_query.single();

    for (mut transform, _) in query.iter_mut() {
        let viewport_size = camera.logical_target_size().unwrap();

        if transform.translation.x > viewport_size.x / 2. {
            transform.translation.x = -viewport_size.x / 2.;
        } else if transform.translation.x < -viewport_size.x / 2. {
            transform.translation.x = viewport_size.x / 2.;
        }

        if transform.translation.y > viewport_size.y / 2. {
            transform.translation.y = -viewport_size.y / 2.;
        } else if transform.translation.y < -viewport_size.y / 2. {
            transform.translation.y = viewport_size.y / 2.;
        }
    }
}

fn compute_flow_field(
    mut flow_field: ResMut<FlowField>,
    target_query: Query<&Target>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    mut gizmos: Gizmos,
) {
    let (camera, camera_transform) = camera_query.single();
    let target: Vec3 = target_query.single().0;

    let viewport_size = camera.logical_target_size().unwrap();

    let flow_field_scale = flow_field.resolution;
    let field_width = viewport_size.x / flow_field_scale;
    let field_height = viewport_size.y / flow_field_scale;

    // Resize flow field if necessary
    if (flow_field.field.len() != field_width as usize)
        || (flow_field.field[0].len() != field_height as usize)
    {
        flow_field.field =
            vec![vec![Vec3::new(0., 0., 0.); field_height as usize]; field_width as usize];
    }

    // Compute flow field

    gizmos.circle_2d(Vec2::new(target.x, target.y), 5., Color::BLUE);

    for x in 0..field_width as usize {
        for y in 0..field_height as usize {
            let screen_position =
                Vec2::new(x as f32 * flow_field_scale, y as f32 * flow_field_scale);
            let raw_world_position = camera
                .viewport_to_world_2d(camera_transform, screen_position)
                .unwrap();
            let world_position = Vec3::new(raw_world_position.x, raw_world_position.y, 0.);
            let desired_velocity = target - world_position;
            flow_field.field[x][y] = desired_velocity;

            /*info!(
                "flow field position: x:{} y:{}: {:?} velocity {:?}",
                x, y, raw_world_position, desired_velocity
            );*/
        }
    }
}

// direct port of processing's map function
fn map(value: f32, start1: f32, stop1: f32, start2: f32, stop2: f32) -> f32 {
    start2 + (stop2 - start2) * ((value - start1) / (stop1 - start1))
}

fn clamp_magnitude(value: Vec3, max: f32) -> Vec3 {
    if value.length() > max {
        value.normalize_or_zero() * max
    } else {
        value
    }
}

fn seek_flow_field(
    mut query: Query<(
        &Transform,
        &mut Acceleration,
        &Velocity,
        &MaxSpeed,
        &MaxForce,
        With<FlowFieldFollower>,
    )>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    flow_field: Res<FlowField>,
    force_multipliers: Res<ForceMultipliers>,
    //mut gizmos: Gizmos,
) {
    let (camera, camera_transform) = camera_query.single();

    for (transform, mut acceleration, velocity, max_speed, max_force, ()) in query.iter_mut() {
        let location = transform.translation;

        let mut desired_velocity =
            flow_field.get_desired_velocity_for_world_position(camera, camera_transform, location);

        let distance: f32 = desired_velocity.length();

        //gizmos.line(location, location + desired_velocity, Color::GREEN);

        let target_radius = 100.;
        if distance < target_radius {
            let m = map(distance, 0., target_radius, 0., max_speed.0);
            desired_velocity = desired_velocity.normalize_or_zero() * m;
        } else {
            desired_velocity = desired_velocity.normalize_or_zero() * max_speed.0;
        }

        let mut steering = desired_velocity - velocity.0;
        steering = clamp_magnitude(steering, max_force.0);

        acceleration.0 += steering * force_multipliers.seek;
    }
}

fn separate(
    mut query: Query<(
        &Transform,
        &mut Acceleration,
        &Velocity,
        &MaxSpeed,
        &MaxForce,
    )>,
    other_query: Query<(&Transform, &Velocity)>,
    force_multipliers: Res<ForceMultipliers>,
) {
    let desired_separation = 20.;

    for (transform, mut acceleration, velocity, max_speed, max_force) in query.iter_mut() {
        let mut sum = Vec3::new(0., 0., 0.);
        let mut count = 0;

        for (other_transform, _) in other_query.iter() {
            let d = transform.translation.distance(other_transform.translation);

            if (d > 0.) && (d < desired_separation) {
                let mut diff = transform.translation - other_transform.translation;
                diff = diff.normalize_or_zero() / d;
                sum += diff;
                count += 1;
            }
        }

        if count > 0 {
            sum /= count as f32;
            sum = sum.normalize_or_zero();
            sum *= max_speed.0;
            let mut steer = sum - velocity.0;
            steer = clamp_magnitude(steer, max_force.0);
            acceleration.0 += steer * force_multipliers.separation;
        }
    }
}

fn align(
    mut query: Query<(
        &Transform,
        &mut Acceleration,
        &Velocity,
        &MaxSpeed,
        &MaxForce,
    )>,
    other_query: Query<(&Transform, &Velocity)>,
    force_multipliers: Res<ForceMultipliers>,
) {
    let neighbor_distance = 50.;

    for (transform, mut acceleration, velocity, max_speed, max_force) in query.iter_mut() {
        let mut sum = Vec3::new(0., 0., 0.);
        let mut count = 0;

        for (other_transform, other_velocity) in other_query.iter() {
            let d = transform.translation.distance(other_transform.translation);

            if d > 0. && d < neighbor_distance {
                sum += other_velocity.0;
                count += 1;
            }
        }

        if count == 0 {
            continue;
        }

        sum /= count as f32;
        sum = sum.normalize_or_zero();
        sum *= max_speed.0;

        let mut steer = sum - velocity.0;
        steer = clamp_magnitude(steer, max_force.0);
        acceleration.0 += steer * force_multipliers.alignment;
    }
}

fn cohesion(
    mut query: Query<(
        &Transform,
        &mut Acceleration,
        &Velocity,
        &MaxSpeed,
        &MaxForce,
    )>,
    other_query: Query<(&Transform, &Velocity)>,
    force_multipliers: Res<ForceMultipliers>,
) {
    let neighbor_distance = 50.;

    for (transform, mut acceleration, velocity, max_speed, max_force) in query.iter_mut() {
        let mut sum = Vec3::new(0., 0., 0.);
        let mut count = 0;

        for (other_transform, _) in other_query.iter() {
            let d = transform.translation.distance(other_transform.translation);

            if d > 0. && d < neighbor_distance {
                sum += other_transform.translation;
                count += 1;
            }
        }

        if count == 0 {
            continue;
        }

        sum /= count as f32;

        let mut desired_position = sum - transform.translation;
        desired_position = desired_position.normalize_or_zero();
        desired_position *= max_speed.0;

        let mut steer = desired_position - velocity.0;
        steer = clamp_magnitude(steer, max_force.0);
        acceleration.0 += steer * force_multipliers.cohesion;
    }
}

fn apply_acceleration(mut query: Query<(&mut Velocity, &mut Acceleration, &MaxSpeed)>) {
    for (mut velocity, mut acceleration, max_speed) in query.iter_mut() {
        velocity.0 += acceleration.0;
        velocity.0 = clamp_magnitude(velocity.0, max_speed.0);

        acceleration.0 *= 0.;
    }
}

fn update_position(mut query: Query<(&mut Transform, &Velocity)> /*mut gizmos: Gizmos*/) {
    for (mut transform, velocity) in query.iter_mut() {
        /*gizmos.line(
            transform.translation,
            transform.translation + velocity.0 * 100.,
            Color::RED,
        );*/

        transform.translation += Vec3::new(velocity.0.x, velocity.0.y, 0.);
        transform.rotation = Quat::from_rotation_z(velocity.0.y.atan2(velocity.0.x) + 180.);
    }
}
