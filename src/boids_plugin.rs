use bevy::prelude::*;
use kd_tree::{KdPoint, KdTree};

#[derive(Clone, Debug)]
pub struct KdTreeItem {
    point: [f32; 2],
    entity: Entity,
}

impl KdPoint for KdTreeItem {
    type Scalar = f32;
    type Dim = typenum::U2; // 2 dimensional tree.
    fn at(&self, k: usize) -> f32 {
        self.point[k]
    }
}

#[derive(Resource)]
pub struct SpatialTree {
    pub tree: KdTree<KdTreeItem>,
}

#[derive(Resource)]
pub struct ForceMultipliers {
    separation: f32,
    alignment: f32,
    cohesion: f32,
}

#[derive(Component)]
pub struct Velocity(pub Vec3);

#[derive(Component)]
pub struct Acceleration(pub Vec3);

#[derive(Component)]
pub struct MaxSpeed(pub f32);

#[derive(Component)]
pub struct MaxForce(pub f32);

#[derive(Component)]
pub struct TrackedByKdTree;

#[derive(Bundle)]
pub struct BoidBundle {
    pub velocity: Velocity,
    pub acceleration: Acceleration,
    pub max_speed: MaxSpeed,
    pub max_force: MaxForce,
    pub tracked_by_kd_tree: TrackedByKdTree,
}

impl Default for BoidBundle {
    fn default() -> Self {
        Self {
            velocity: Velocity(Vec3::new(0., 0., 0.)),
            acceleration: Acceleration(Vec3::new(0., 0., 0.)),
            max_speed: MaxSpeed(4. * 60.),
            max_force: MaxForce(0.5 * 60.),
            tracked_by_kd_tree: TrackedByKdTree,
        }
    }
}

pub struct BoidsPlugin;

impl Plugin for BoidsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ForceMultipliers {
            separation: 2.0,
            alignment: 1.0,
            cohesion: 1.0,
        })
        .insert_resource(SpatialTree {
            tree: kd_tree::KdTree::build_by_ordered_float(Vec::new()),
        })
        .add_systems(PreUpdate, (wrap_around_screen, update_spatial_tree))
        .add_systems(
            Update,
            (
                separate,
                align,
                cohesion,
                apply_acceleration,
                update_position,
            ),
        );
    }
}

fn update_spatial_tree(
    query: Query<(Entity, &Transform), With<TrackedByKdTree>>,
    mut kd_tree: ResMut<SpatialTree>,
) {
    let mut raw_vec = Vec::with_capacity(query.iter().len());

    for (entity, transform) in query.iter() {
        raw_vec.push(KdTreeItem {
            point: [transform.translation.x, transform.translation.y],
            entity,
        });
    }

    kd_tree.tree = kd_tree::KdTree::par_build_by_ordered_float(raw_vec);
}

fn wrap_around_screen(
    mut query: Query<(&mut Transform, &Velocity)>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
) {
    let (camera, _) = camera_query.single();

    query.par_iter_mut().for_each(|(mut transform, _)| {
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
    });
}

fn separate(
    mut query: Query<(
        &Transform,
        &mut Acceleration,
        &Velocity,
        &MaxSpeed,
        &MaxForce,
        With<TrackedByKdTree>,
    )>,
    force_multipliers: Res<ForceMultipliers>,
    kd_tree: Res<SpatialTree>,
) {
    let desired_separation = 10.;

    query.par_iter_mut().for_each(
        |(transform, mut acceleration, velocity, max_speed, max_force, ())| {
            let mut sum = Vec3::new(0., 0., 0.);
            let mut count = 0;
            let location = Vec2::new(transform.translation.x, transform.translation.y);

            let results = kd_tree
                .tree
                .within_radius(&[location.x, location.y], desired_separation);

            for result in &results {
                let other_position = Vec3::new(result.point[0], result.point[1], 0.);
                let distance = transform.translation.distance(other_position);

                if distance == 0.0 {
                    continue;
                }

                let mut diff = transform.translation - other_position;
                diff = diff.normalize_or_zero() / distance;
                sum += diff;
                count += 1;
            }

            if count > 0 {
                sum /= count as f32;
                sum = sum.normalize_or_zero();
                sum *= max_speed.0;
                let mut steer = sum - velocity.0;

                steer = clamp_magnitude(steer, max_force.0);
                acceleration.0 += steer * force_multipliers.separation;
            }
        },
    );
}

fn align(
    mut query: Query<(
        &Transform,
        &mut Acceleration,
        &Velocity,
        &MaxSpeed,
        &MaxForce,
        With<TrackedByKdTree>,
    )>,
    other_query: Query<(&Transform, &Velocity), With<TrackedByKdTree>>,
    force_multipliers: Res<ForceMultipliers>,
    kd_tree: Res<SpatialTree>,
) {
    let neighbor_distance = 20.;

    query.par_iter_mut().for_each(
        |(transform, mut acceleration, velocity, max_speed, max_force, ())| {
            let mut sum = Vec3::new(0., 0., 0.);
            let mut count = 0;
            let location = Vec2::new(transform.translation.x, transform.translation.y);

            let results = kd_tree
                .tree
                .within_radius(&[location.x, location.y], neighbor_distance);

            for result in &results {
                if let Ok((_, velocity)) = other_query.get(result.entity) {
                    sum += velocity.0;
                    count += 1;
                }
            }

            if count == 0 {
                return;
            }

            sum /= count as f32;
            sum = sum.normalize_or_zero();
            sum *= max_speed.0;

            let mut steer = sum - velocity.0;

            steer = clamp_magnitude(steer, max_force.0);
            acceleration.0 += steer * force_multipliers.alignment;
        },
    );
}

fn cohesion(
    mut query: Query<
        (
            &Transform,
            &mut Acceleration,
            &Velocity,
            &MaxSpeed,
            &MaxForce,
        ),
        With<TrackedByKdTree>,
    >,
    force_multipliers: Res<ForceMultipliers>,
    kd_tree: Res<SpatialTree>,
) {
    let neighbor_distance = 20.;

    query.par_iter_mut().for_each(
        |(transform, mut acceleration, velocity, max_speed, max_force)| {
            let mut sum = Vec3::new(0., 0., 0.);
            let mut count = 0;
            let location = Vec2::new(transform.translation.x, transform.translation.y);

            let results = kd_tree
                .tree
                .within_radius(&[location.x, location.y], neighbor_distance);

            for result in &results {
                let other_position = Vec3::new(result.point[0], result.point[1], 0.);
                sum += other_position;
                count += 1;
            }

            if count == 0 {
                return;
            }

            sum /= count as f32;

            let mut desired_position = sum - transform.translation;
            desired_position = desired_position.normalize_or_zero();
            desired_position *= max_speed.0;

            let mut steer = desired_position - velocity.0;
            steer = clamp_magnitude(steer, max_force.0);
            acceleration.0 += steer * force_multipliers.cohesion;
        },
    );
}

fn apply_acceleration(
    mut query: Query<(&mut Velocity, &mut Acceleration, &MaxSpeed)>,
    time: Res<Time>,
) {
    for (mut velocity, mut acceleration, max_speed) in query.iter_mut() {
        velocity.0 += acceleration.0 * time.delta_seconds();
        velocity.0 = clamp_magnitude(velocity.0, max_speed.0);

        acceleration.0 *= 0.;
    }
}

fn update_position(mut query: Query<(&mut Transform, &Velocity)>, time: Res<Time>) {
    for (mut transform, velocity) in query.iter_mut() {
        transform.translation += Vec3::new(velocity.0.x, velocity.0.y, 0.) * time.delta_seconds();
        transform.rotation = Quat::from_rotation_z(velocity.0.y.atan2(velocity.0.x) + 180.);
    }
}

pub fn clamp_magnitude(value: Vec3, max: f32) -> Vec3 {
    if value.length() > max {
        value.normalize_or_zero() * max
    } else {
        value
    }
}
