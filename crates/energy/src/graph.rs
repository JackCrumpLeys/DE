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
        app.add_system(setup.in_schedule(OnEnter(GameState::Playing)))
            .add_system(
                update_power_grid
                    .in_base_set(GameSet::Update)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_system(
                transfer_energy
                    .in_base_set(GameSet::Update)
                    .after(GraphSage::Update)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_system(clean_up.in_schedule(OnExit(GameState::Playing)));
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, SystemSet)]
enum GraphSage {
    Update,
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

fn setup(mut commands: Commands) {
    commands.insert_resource(PowerGrid::default());
}

fn clean_up(mut commands: Commands) {
    commands.remove_resource::<PowerGrid>();
}

// fn update_power_grid(
//     mut commands: Commands,
//     mut power_grid: ResMut<PowerGrid>,
//     producers_query: Query<(Entity, &EnergyProducer, &Transform)>,
//     receivers_query: Query<(Entity, &EnergyReceiver, &Transform)>,
// ) {
//     let system_run_time = std::time::Instant::now();
//
//     let mut nodes_to_remove: Vec<NodeIndex> = Vec::new();
//
//     // Remove entities that were despawned
//     power_grid.entity_to_node.retain(|&entity, node_index| {
//         if !producers_query.get_component::<Transform>(entity).is_ok()
//             && !receivers_query.get_component::<Transform>(entity).is_ok()
//         {
//             nodes_to_remove.push(*node_index);
//             false
//         } else {
//             true
//         }
//     });
//
//     // Remove nodes that were despawned
//     for node_index in nodes_to_remove {
//         power_grid.graph.remove_node(node_index);
//     }
//
//     let mut entities_to_add: Vec<(Entity, Vec3)> = Vec::new();
//
//     let mut updated_transforms: Vec<(NodeIndex, Vec3)> = Vec::new();
//
//     // Find new and updated entities
//     for (entity, _receiver, transform) in receivers_query.iter() {
//         if let Some(node_index) = power_grid.entity_to_node.get(&entity) {
//             if power_grid.graph[*node_index].location != transform.translation {
//                 updated_transforms.push((*node_index, transform.translation));
//             }
//         } else {
//             entities_to_add.push((entity, transform.translation));
//         }
//     }
//
//     for (entity, _producer, transform) in producers_query.iter() {
//         if let Some(node_index) = power_grid.entity_to_node.get(&entity) {
//             if power_grid.graph[*node_index].location != transform.translation {
//                 updated_transforms.push((*node_index, transform.translation));
//             }
//         } else {
//             entities_to_add.push((entity, transform.translation));
//         }
//     }
//
//     // Update transforms
//     for (index, transform) in updated_transforms {
//         power_grid.graph[index].location = transform;
//     }
//
//     // Update edges based on new and updated entities. Assemble graph
//     for (entity, node_index) in entities_to_add {
//         let new_node = Member {
//             entity,
//             location: node_index,
//         };
//
//         let new_node_index = power_grid.graph.add_node(new_node);
//         power_grid.entity_to_node.insert(entity, new_node_index);
//     }
//
//     let mut edges_to_add: Vec<(NodeIndex, NodeIndex, f64)> = Vec::new();
//
//     for (entity, node) in power_grid.entity_to_node.iter() {
//         let node_index = *node;
//         let node_location = power_grid.graph[node_index].location;
//
//         let mut edges = 0;
//
//         for (other_entity, other_node) in power_grid.entity_to_node.iter() {
//             if entity == other_entity {
//                 continue;
//             }
//
//             let other_node_index = *other_node;
//             let other_node_location = power_grid.graph[other_node_index].location;
//
//             let distance = node_location.distance(other_node_location);
//
//             if distance <= MAX_DISTANCE {
//                 let edge = power_grid.graph.find_edge(node_index, other_node_index);
//
//                 if edge.is_none() && !edges_to_add.contains(&(node_index, other_node_index, MAX_TRANSFER_RATE)) {
//                     edges_to_add.push((node_index, other_node_index, MAX_TRANSFER_RATE));
//                 }
//
//                 edges += 1;
//                 if edges == MAX_EDGES {
//                     break;
//                 }
//             }
//         }
//     }
//     for (node_one, node_two, weight) in edges_to_add {
//         println!("Adding edge {:?} <-> {:?} with weight {}", node_one, node_two, weight);
//         power_grid.graph.add_edge(node_one, node_two, weight);
//     }
//     println!("Power grid update took {:?}", system_run_time.elapsed());
// }

fn update_nodes_helper(
    commands: &mut Commands,
    power_grid: &mut ResMut<PowerGrid>,
    nodes_to_remove: &mut Vec<NodeIndex>,
) {
    // Remove nodes that were despawned from the graph
    for node_index in nodes_to_remove.iter() {
        power_grid.graph.remove_node(*node_index);
    }
    nodes_to_remove.clear();
}

fn update_edges_helper(
    power_grid: &mut ResMut<PowerGrid>,
    node_one: NodeIndex,
    node_location: Vec3,
    edges_to_add: &mut Vec<(NodeIndex, NodeIndex, f64)>,
    edges_to_remove: &mut Vec<(NodeIndex, NodeIndex)>,
) {
    let mut edges = 0;
    for (other_node_index, other_node) in power_grid
        .graph
        .node_indices()
        .map(|i| (i, &power_grid.graph[i]))
    {
        if node_one == other_node_index {
            continue;
        }

        let other_node_location = other_node.location;
        let distance = node_location.distance(other_node_location);

        if distance <= MAX_DISTANCE
            && power_grid
                .graph
                .find_edge(node_one, other_node_index)
                .is_none()
            && !edges_to_add.contains(&(node_one, other_node_index, MAX_TRANSFER_RATE))
        {
            edges_to_add.push((node_one, other_node_index, MAX_TRANSFER_RATE));

            edges += 1;
            if edges == MAX_EDGES {
                break;
            }
        }

        if distance > MAX_DISTANCE
            && power_grid
                .graph
                .find_edge(node_one, other_node_index)
                .is_some()
        {
            edges_to_remove.push((node_one, other_node_index));
        }
    }
}

fn update_power_grid(
    mut commands: Commands,
    mut power_grid: ResMut<PowerGrid>,
    producers_query: Query<(Entity, &EnergyProducer, &Transform)>,
    receivers_query: Query<(Entity, &EnergyReceiver, &Transform)>,
) {
    let system_run_time = std::time::Instant::now();

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
    update_nodes_helper(&mut commands, &mut power_grid, &mut nodes_to_remove);

    println!(
        "Power grid node despawning took {:?}",
        system_run_time.elapsed()
    );

    let mut updated_transforms: Vec<(NodeIndex, Vec3)> = Vec::new();

    // Find new and updated entities
    for (entity, _receiver, transform) in receivers_query.iter() {
        if let Some(node_index) = power_grid.entity_to_node.get(&entity) {
            if power_grid.graph[*node_index].location != transform.translation {
                updated_transforms.push((*node_index, transform.translation));
            }
        } else {
            let new_node = Member {
                entity,
                location: transform.translation,
            };

            let new_node_index = power_grid.graph.add_node(new_node);
            power_grid.entity_to_node.insert(entity, new_node_index);
        }
    }

    for (entity, _producer, transform) in producers_query.iter() {
        if let Some(node_index) = power_grid.entity_to_node.get(&entity) {
            if power_grid.graph[*node_index].location != transform.translation {
                updated_transforms.push((*node_index, transform.translation));
            }
        } else {
            let new_node = Member {
                entity,
                location: transform.translation,
            };

            let new_node_index = power_grid.graph.add_node(new_node);
            power_grid.entity_to_node.insert(entity, new_node_index);
        }
    }

    for (node, transform) in updated_transforms {
        power_grid.graph[node].location = transform;
    }

    println!(
        "Power grid update transforms took {:?}",
        system_run_time.elapsed()
    );

    // Update edges based on new and updated entities. Assemble graph
    let mut edges_to_add: Vec<(NodeIndex, NodeIndex, f64)> = Vec::new();
    let mut edges_to_remove: Vec<(NodeIndex, NodeIndex)> = Vec::new();

    for node_one in power_grid.graph.node_indices() {
        let node = power_grid.graph[node_one];
        let node_location = node.location;

        update_edges_helper(&mut power_grid, node_one, node_location, &mut edges_to_add, &mut edges_to_remove);
    }

    println!(
        "Power grid update edges took {:?}",
        system_run_time.elapsed()
    );

    for (node_one, node_two, weight) in edges_to_add {
        println!(
            "Adding edge {:?} <-> {:?} with weight {}",
            node_one, node_two, weight
        );
        power_grid.graph.add_edge(node_one, node_two, weight);
    }
    println!("Power grid update took {:?}", system_run_time.elapsed());
}

fn transfer_energy(
    mut power_grid: ResMut<PowerGrid>,
    mut producers_query: Query<(&EnergyProducer,)>,
    mut receivers_query: Query<(&EnergyReceiver,)>,
) {
    // use power_grid.graph and petgraph's algorithms to transfer energy between producers and receivers

    // for each producer, find the shortest path to a receiver (this can travle though many nodes)
}
