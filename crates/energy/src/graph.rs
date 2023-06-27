use bevy::prelude::*;
use de_core::baseset::GameSet;
use de_core::gamestate::GameState;
use de_index::{EntityKdTree, SpatialQuery};
use petgraph::prelude::*;
use de_core::projection::ToFlat;


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

/// The power grid resource is used to store the power grid graph.
#[derive(Resource, Debug, Clone)]
pub(crate) struct PowerGrid {
    /// The power grid graph.
    graph: GraphMap<Entity, f64, Undirected>,
}

impl Default for PowerGrid {
    fn default() -> Self {
        Self {
            graph: GraphMap::new(),
        }
    }
}

fn setup(mut commands: Commands) {
    commands.insert_resource(PowerGrid::default());
}

fn clean_up(mut commands: Commands) {
    commands.remove_resource::<PowerGrid>();
}

fn update_nodes_helper(power_grid: &mut ResMut<PowerGrid>, nodes_to_remove: &mut Vec<Entity>) {
    // Remove nodes that were despawned from the graph
    for node_index in nodes_to_remove.iter() {
        power_grid.graph.remove_node(*node_index);
    }
    nodes_to_remove.clear();
}

fn update_edges_helper(
    power_grid: &PowerGrid,
    kd_tree: &EntityKdTree,
    transforms: &Query<&Transform, Or<(With<EnergyProducer>, With<EnergyReceiver>)>>,
    node_one: Entity,
    node_location: Vec3,
    edges_to_add: &mut Vec<(Entity, Entity, f64)>,
) {
    let mut collected:usize = 0;
    println!("considering {:?}, node_location: {:?}", node_one, node_location);
    for edge in kd_tree
        .radius(node_location.to_flat().as_ref(), MAX_DISTANCE)
        .iter() {
        println!("edge: {:?}", edge);
        if edge.1 == node_one {
            println!("edge is self");
            continue;
        }
        println!("edge: {:?}, dist {:?}", edge.1, edge.0);
        if power_grid.graph.contains_edge(node_one, edge.1) {
            println!("edge already exists");
            continue;
        }
        if collected >= MAX_EDGES {
            println!("max edges reached");
            break;
        }
        println!("adding edge");
        edges_to_add.push((node_one, edge.1, MAX_TRANSFER_RATE));
        collected += 1;
    }



    // edges_to_add.append(
    //     &mut
    //         .map(|(distance, node_two)| (node_one, *node_two, MAX_TRANSFER_RATE))
    //         .collect(),
    // )
}

fn update_power_grid(
    mut power_grid: ResMut<PowerGrid>,
    kd_tree: Res<EntityKdTree>,
    power_query: Query<&Transform, Or<(With<EnergyProducer>, With<EnergyReceiver>)>>,
    changed_transforms: Query<
        Entity,
        (
            Changed<Transform>,
            Or<(With<EnergyProducer>, With<EnergyReceiver>)>,
        ),
    >,
    new_entities: Query<Entity, Or<(Added<EnergyProducer>, Added<EnergyReceiver>)>>,
    mut removed_receivers: RemovedComponents<EnergyReceiver>,
    mut removed_producers: RemovedComponents<EnergyProducer>,
    spacial_index: SpatialQuery<Entity, Or<(With<EnergyProducer>, With<EnergyReceiver>)>>,
) {
    let system_run_time = std::time::Instant::now();

    let mut nodes_to_remove: Vec<Entity> = Vec::new();

    // combine removed receivers and producers
    let removed_entities: Vec<Entity> = removed_receivers
        .iter()
        .chain(removed_producers.iter())
        .collect();

    // Remove entities that were despawned
    for entity in removed_entities.iter() {
        nodes_to_remove.push(*entity);
    }

    // Remove nodes that were despawned
    update_nodes_helper(&mut power_grid, &mut nodes_to_remove);

    // println!(
    //     "Power grid node despawning took {:?}",
    //     system_run_time.elapsed()
    // );
    //
    // println!(
    //     "Power grid update entities took {:?}",
    //     system_run_time.elapsed()
    // );
    //
    // println!(
    //     "Power grid update transforms took {:?}",
    //     system_run_time.elapsed()
    // );

    // // Update edges based on new and updated entities. Assemble graph
    // let mut edges_to_add_lock: RwLock<Vec<(Entity, Entity, f64)>> = RwLock::new(Vec::new());
    // let mut edges_to_remove_lock: RwLock<Vec<(Entity, Entity)>> = RwLock::new(Vec::new());
    //
    // let power_grid_lock = RwLock::new(power_grid);

    let mut edges_to_add: Vec<(Entity, Entity, f64)> = Vec::new();

    for entity in new_entities.iter() {
        println!("new entity: {:?}", entity);
        let node = power_grid.graph.add_node(entity);
        let node_location = power_query.get(entity).unwrap().translation;

        update_edges_helper(
            power_grid.as_ref(),
            kd_tree.as_ref(),
            &power_query,
            node,
            node_location,
            &mut edges_to_add,
        );
    }

    for entity in changed_transforms.iter() {
        println!("changed entity: {:?}", entity);
        let node_location = power_query.get(entity).unwrap().translation;

        update_edges_helper(
            power_grid.as_ref(),
            kd_tree.as_ref(),
            &power_query,
            entity,
            node_location,
            &mut edges_to_add,
        );
    }

    // look for entities that have had the

    // println!(
    //     "Power grid update edges took {:?}",
    //     system_run_time.elapsed()
    // );

    let mut considered_nodes = Vec::new();
    let mut edges_to_remove = Vec::new();

    for (node_one, node_two, weight) in edges_to_add {
        // println!(
        //     "Adding edge {:?} <-> {:?} with weight {}",
        //     node_one, node_two, weight
        // );
        if !considered_nodes.contains(&node_one) {
            considered_nodes.push(node_one);

            edges_to_remove.append(
                &mut power_grid
                    .graph
                    .edges(node_one)
                    .map(|edge| (edge.source(), edge.target()))
                    .collect::<Vec<(Entity, Entity)>>()
                    .clone(),
            );
        }
        power_grid.graph.add_edge(node_one, node_two, weight);
    }

    for edge in edges_to_remove {
        // println!("Removing edge {:?} <-> {:?}", edge.0, edge.1);
        power_grid.graph.remove_edge(edge.0, edge.1);
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
