use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::{
    App, Changed, Component, Entity, IntoSystemConfigs, Query, Res, ResMut, Resource, Schedule,
    Transform, Update, World,
};
use bevy::time::TimePlugin;
use criterion::{criterion_group, criterion_main, Criterion};
use de_core::projection::ToAltitude;
use de_energy::{update_nearby_recv, EnergyReceiver, NearbyUnits};
use de_index::{EntityIndex, LocalCollider};
use de_objects::ObjectCollider;
use de_test_utils::{load_points, NumPoints};
use parry3d::math::{Isometry, Vector};
use parry3d::shape::{Cuboid, TriMesh};

const UNIT_SPACING: f32 = 7.;
const MOVEMENT_RADIUS: f32 = 40.;
const SPEED: f32 = 10.; // based on MAX_H_SPEED in movement

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct UpdateUnits;

#[derive(Component)]
struct UnitNumber(u32);

#[derive(Component)]
struct Centre {
    x: f32,
    y: f32,
}

#[derive(Resource)]
struct Clock(f32); // this clock is used in a substitute of time to make it more deterministic

impl Clock {
    fn inc(&mut self) {
        self.0 += 0.1 // 10HZ
    }
}

fn update_index(
    mut index: ResMut<EntityIndex>,
    moved: Query<(Entity, &Transform), Changed<Transform>>,
) {
    for (entity, transform) in moved.iter() {
        let position = Isometry::new(
            transform.translation.into(),
            transform.rotation.to_scaled_axis().into(),
        );
        index.update(entity, position);
    }
}

fn init_world_with_entities_moving(world: &mut World, num_entities: &NumPoints) {
    let mut index = EntityIndex::new();

    let max_point_value =
        UNIT_SPACING * (<NumPoints as Into<usize>>::into(*num_entities) as f32).sqrt();
    assert!(max_point_value > 0.);

    let points = load_points(num_entities, max_point_value);

    for (i, point) in points.into_iter().enumerate() {
        let point_msl = point.to_msl();

        let collider = LocalCollider::new(
            ObjectCollider::from(TriMesh::from(Cuboid::new(Vector::new(3., 3., 4.)))),
            Isometry::new(point_msl.into(), Vector::identity()),
        );

        let entity = world
            .spawn((
                Transform::from_translation(point_msl),
                Centre {
                    x: point_msl.x,
                    y: point_msl.y,
                },
                EnergyReceiver,
                NearbyUnits::default(),
                UnitNumber(i as u32),
            ))
            .id();

        index.insert(entity, collider);
    }

    world.insert_resource(Clock(0.));
    world.insert_resource(index);
}

/// Move entities in circles of radius DISTANCE_FROM_MAP_EDGE / 2.
fn move_entities_in_circle(
    clock: Res<Clock>,
    mut query: Query<(&mut Transform, &UnitNumber, &Centre)>,
) {
    for (mut transform, unit_number, centre) in query.iter_mut() {
        // Change direction (counter)clockwise based on entity_mum % 2 == 0
        let direction = if unit_number.0 % 2 == 0 { 1. } else { -1. };

        let t = clock.0;
        let omega = SPEED / MOVEMENT_RADIUS;
        let omega_shift = 0.1 * unit_number.0 as f32;

        let x = MOVEMENT_RADIUS * (omega_shift + t * omega * direction).sin();
        let y = MOVEMENT_RADIUS * (omega_shift + t * omega * direction).cos();

        transform.translation.x = x + centre.x;
        transform.translation.y = y + centre.y;
    }
}

fn nearby_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("Nearby unit movement scenarios");

    use NumPoints::*;
    for i in [OneHundred, OneThousand, TenThousand, OneHundredThousand] {
        let mut app = App::default();
        init_world_with_entities_moving(&mut app.world, &i);
        app.add_systems(Update, update_nearby_recv);
        app.add_plugins(TimePlugin);

        let update_units_schedule = Schedule::default();
        app.add_schedule(UpdateUnits, update_units_schedule);

        app.add_systems(
            UpdateUnits,
            (
                move_entities_in_circle,
                update_index.after(move_entities_in_circle),
            ),
        );

        let number_of_units: usize = i.into();

        group.throughput(criterion::Throughput::Elements(number_of_units as u64));
        group.bench_function(
            format!("{} units all moving in circles", number_of_units),
            |b| {
                b.iter_custom(|iters| {
                    let time = std::time::Instant::now();
                    let mut duration_updating_other_stuff = std::time::Duration::default();

                    for _ in 0..iters {
                        let update_other_stuff = std::time::Instant::now();
                        app.world.resource_mut::<Clock>().inc();
                        app.world.run_schedule(UpdateUnits);
                        duration_updating_other_stuff += update_other_stuff.elapsed();

                        app.update();
                    }

                    time.elapsed() - duration_updating_other_stuff
                });
            },
        );
        let mut amount_of_connections_formed = 0;
        for connections in app.world.query::<&mut NearbyUnits>().iter(&app.world) {
            amount_of_connections_formed += connections.units().len()
        }
        println!(
            "We ended up with {} connections between {} units",
            amount_of_connections_formed, number_of_units
        );
    }
}

criterion_group!(benches, nearby_benchmark);

criterion_main!(benches);