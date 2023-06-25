use std::collections::HashMap;

use bevy::prelude::*;
use de_core::baseset::GameSet;
use de_core::gamestate::GameState;
use petgraph::dot::{Config, Dot};
use petgraph::graph::UnGraph;
use petgraph::prelude::*;

use crate::Battery;

// The max distance (in meters) between two entities for them to be consider neighbors in the graph
const MAX_DISTANCE: f32 = 10.0;
// The max transfer rate (in joules per second) between two entities
const MAX_TRANSFER_RATE: f64 = 1_000_000.0;
// The max edges per node
const MAX_EDGES: usize = 4;

pub(crate) struct PowerGridPlugin;

impl Plugin for PowerGridPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(update_power_grid.in_base_set(GameSet::Update)
                           .run_if(in_state(GameState::Playing)).add_system(
            transfer_energy
                .in_base_set(GameSet::Update)
                .run_if(in_state(GameState::Playing)), // TODO: Run after
        );
    }
}

/// The energy receiver component is used to mark an entity as an energy receiver.
#[derive(Component, Debug, Clone, Copy)]
pub struct EnergyReceiver;

/// The energy producer component is used to mark an entity as an energy producer.
#[derive(Component, Debug, Clone, Copy)]
pub struct EnergyProducer;

/// A member of the power grid.
#[derive(Debug, Clone, Copy)]
struct Member {
    /// The entity of the member.
    entity: Entity,
    /// The energy receiver component of the member.
    location: Vec3,
}

/// The power grid resource is used to store the power grid graph.
#[derive(Resource, Debug, Clone)]
pub(crate) struct PowerGrid {
    /// The power grid graph.
    graph: UnGraph<Member, f64>,
    /// A map from entities to their corresponding node index in the graph.
    entity_to_node: HashMap<Entity, NodeIndex>,
}

impl Default for PowerGrid {
    fn default() -> Self {
        Self {
            graph: UnGraph::new_undirected(),
            entity_to_node: HashMap::new(),
        }
    }
}

fn update_power_grid(
    mut commands: Commands,
    mut power_grid: ResMut<PowerGrid>,
    producers_query: Query<(Entity, &EnergyProducer, &Transform)>,
    receivers_query: Query<(Entity, &EnergyReceiver, &Transform)>,
) {
    let mut nodes_to_remove: Vec<NodeIndex> = Vec::new();

    // Remove entities that were despawned
    power_grid.entity_to_node.retain(|&entity, node_index| {
        if !producers_query.get_component::<Transform>(entity).is_ok()
            && !receivers_query.get_component::<Transform>(entity).is_ok()
        {
            nodes_to_remove.push(*node_index);
            false
        } else {
            true
        }
    });

    // Remove nodes that were despawned
    for node_index in nodes_to_remove {
        power_grid.graph.remove_node(node_index);
    }

    let mut entities_to_add: Vec<(Entity, Vec3)> = Vec::new();

    let mut updated_transforms: Vec<(NodeIndex, Vec3)> = Vec::new();

    // Find new and updated entities
    for (entity, _receiver, transform) in receivers_query.iter() {
        if let Some(node_index) = power_grid.entity_to_node.get(&entity) {
            if power_grid.graph[*node_index].location != transform.translation {
                updated_transforms.push((*node_index, transform.translation));
            }
        } else {
            entities_to_add.push((entity, transform.translation));
        }
    }

    for (entity, _producer, transform) in producers_query.iter() {
        if let Some(node_index) = power_grid.entity_to_node.get(&entity) {
            if power_grid.graph[*node_index].location != transform.translation {
                updated_transforms.push((*node_index, transform.translation));
            }
        } else {
            entities_to_add.push((entity, transform.translation));
        }
    }

    // Update transforms
    for (index, transform) in updated_transforms {
        power_grid.graph[index].location = transform;
    }

    // Update edges based on new and updated entities. Assemble graph
    for (entity, node_index) in entities_to_add {
        let mut new_node = Member {
            entity,
            location: node_index,
        };

        let mut new_node_index = power_grid.graph.add_node(new_node);
        power_grid.entity_to_node.insert(entity, new_node_index);

        let mut neighbors: Vec<NodeIndex> = Vec::new();

        for (other_entity, _producer, other_transform) in producers_query.iter() {
            if let Some(other_node_index) = power_grid.entity_to_node.get(&other_entity) {
                if other_transform.translation.distance(new_node.location) <= MAX_DISTANCE {
                    neighbors.push(*other_node_index);
                }
            }
        }

        for neighbor in neighbors {
            power_grid
                .graph
                .add_edge(new_node_index, neighbor, MAX_TRANSFER_RATE);
        }
    }

    // debug print visual graph
    println!(
        "{:?}",
        Dot::with_config(&power_grid.graph, &[Config::EdgeNoLabel])
    );
}

fn transfer_energy(
    mut power_grid: ResMut<PowerGrid>,
    mut producers_query: Query<(&EnergyProducer,)>,
    mut receivers_query: Query<(&EnergyReceiver,)>,
) {
    // use power_grid.graph and petgraph's algorithms to transfer energy between producers and receivers

    // for each producer, find the shortest path to a receiver (this can travle though many nodes)
}
