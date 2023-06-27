use std::collections::HashMap;
use kiddo::float::kdtree::KdTree;
use bevy::prelude::*;
use kiddo::nearest_neighbour::NearestNeighbour;
use de_core::baseset::GameSet;
use de_core::gamestate::GameState;
use de_core::objects::{MovableSolid, StaticSolid};
use de_objects::SolidObjects;

struct KdTreePlugin;

impl Plugin for KdTreePlugin {
    fn build(&self, app: &mut App) {
        app.add_system(setup.in_schedule(OnEnter(GameState::Playing))).add_system(
            update
                .in_base_set(GameSet::PreUpdate)
                .run_if(in_state(GameState::Playing)),
        );
    }
}

#[derive(Debug, Resource)]
struct EntityKdTree {
    tree: KdTree<f32, u32, 3, 32, u32>,
    entity_to_index: HashMap<u32, (Entity, [f32;3])>,
    next_index: u32,
}

impl Default for EntityKdTree {
    fn default() -> Self {
        Self {
            tree: KdTree::new(),
            entity_to_index: HashMap::new(),
            next_index: 0,
        }
    }
}

impl EntityKdTree {
    fn nearest(&self, point: &[f32;3], amount: usize) -> Vec<Entity> {
        self.tree.nearest_n(point, amount).iter().map(|x| *self.entity_to_index.iter().find(|(_, (index, _))| index == x).unwrap().0).collect()
    }

}

#[derive(Component, Debug, Clone, Copy)]
struct TrackedByKdTree;

/// This system iterates over all not yet TrackedByKdTree entities and adds them.
fn insert(
    mut commands: Commands,
    mut tree: ResMut<EntityKdTree>,
    query: Query<(Entity, &Transform), (Without<TrackedByKdTree>,
        Or<(With<StaticSolid>, With<MovableSolid>)>)>,
) {
    for (entity, transform) in query.iter() {
        if !tree.entity_to_index.contains_key(&entity) {
            let mut translation = [transform.translation.x, transform.translation.y, transform.translation.z];
            let index = tree.next_index;

            tree.next_index += 1;
            tree.entity_to_index.insert(entity, (index, translation));
            tree.tree.add(&translation, index);
        }
    }
}

fn setup(mut commands: Commands) {
    commands.insert_resource(EntityKdTree::default());
}

fn update(mut entity_kd_tree: ResMut<EntityKdTree>, query: Query<(Entity, &Transform), (With<TrackedByKdTree>, Changed<Transform>)>) {
    for (entity, transform) in query.iter() {
        let (index, last_known_coords) = *entity_kd_tree.entity_to_index.get(&entity).unwrap();

        entity_kd_tree.tree.remove(&last_known_coords, index);
        entity_kd_tree.tree.add(&[transform.translation.x, transform.translation.y, transform.translation.z], index);

        entity_kd_tree.entity_to_index.get_mut(&entity).unwrap().1 = [transform.translation.x, transform.translation.y, transform.translation.z];

    }
}