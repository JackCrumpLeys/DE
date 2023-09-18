use ahash::AHashMap;
use bevy::{
    ecs::{entity::Entities, system::SystemParam},
    prelude::*,
};
use de_core::{gconfig::GameConfig, schedule::PreMovement, state::AppState};
use de_messages::{EntityNet, ToPlayers};
use de_types::{objects::ActiveObjectType, player::Player};

use crate::messages::{FromPlayersEvent, MessagesSet};

/// This plugin handles incoming player messages during a multiplayer game.
pub(crate) struct PlayerMsgPlugin;

impl Plugin for PlayerMsgPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<NetRecvSpawnActiveEvent>()
            .add_event::<NetRecvDespawnActiveEvent>()
            .add_event::<NetRecvHealthEvent>()
            .add_systems(OnEnter(AppState::InGame), setup)
            .add_systems(OnExit(AppState::InGame), cleanup)
            .add_systems(
                PreMovement,
                recv_messages
                    .run_if(on_event::<FromPlayersEvent>())
                    .run_if(in_state(AppState::InGame))
                    .in_set(GameNetSet::Messages)
                    .after(MessagesSet::RecvMessages),
            );
    }
}

#[derive(Copy, Clone, Hash, Debug, PartialEq, Eq, SystemSet)]
pub enum GameNetSet {
    Messages,
}

/// This event is sent when a new entity of a non-local player is to be
/// spawned. An empty ECS entity is spawned to obtain local entity ID. The rest
/// is kept to the handling event systems.
///
/// This event is send during [`GameNetSet::Messages`] set.
#[derive(Event)]
pub struct NetRecvSpawnActiveEvent {
    player: Player,
    entity: Entity,
    object_type: ActiveObjectType,
    transform: Transform,
}

impl NetRecvSpawnActiveEvent {
    fn new(
        player: Player,
        entity: Entity,
        object_type: ActiveObjectType,
        transform: Transform,
    ) -> Self {
        Self {
            player,
            entity,
            object_type,
            transform,
        }
    }

    pub fn player(&self) -> Player {
        self.player
    }

    /// Local (empty) entity ID.
    pub fn entity(&self) -> Entity {
        self.entity
    }

    pub fn object_type(&self) -> ActiveObjectType {
        self.object_type
    }

    pub fn transform(&self) -> Transform {
        self.transform
    }
}

/// This event is sent when an active entity of a non-local player is to be
/// despawned.
///
/// This event is send during [`GameNetSet::Messages`] set.
#[derive(Event)]
pub struct NetRecvDespawnActiveEvent {
    entity: Entity,
}

impl NetRecvDespawnActiveEvent {
    fn new(entity: Entity) -> Self {
        Self { entity }
    }

    pub fn entity(&self) -> Entity {
        self.entity
    }
}

#[derive(Event)]
pub struct NetRecvHealthEvent {
    entity: Entity,
    delta: f32,
}

impl NetRecvHealthEvent {
    /// # Panics
    ///
    /// Panics if delta is not a finite number.
    fn new(entity: Entity, delta: f32) -> Self {
        assert!(delta.is_finite());
        Self { entity, delta }
    }

    pub fn entity(&self) -> Entity {
        self.entity
    }

    pub fn delta(&self) -> f32 {
        self.delta
    }
}

#[derive(SystemParam)]
pub struct NetEntities<'w> {
    config: Res<'w, GameConfig>,
    map: Res<'w, EntityIdMapRes>,
}

impl<'w> NetEntities<'w> {
    /// Translates a local entity ID to a remote entity ID. This works for both
    /// locally simulated and non-local entities.
    ///
    /// It is assumed that the entity exists.
    pub fn net_id(&self, entity: Entity) -> EntityNet {
        match self.map.translate_local(entity) {
            Some(id) => id,
            None => self.local_net_id(entity),
        }
    }

    /// Translates a local entity ID to a remote entity ID. This works only for
    /// locally simulated entities.
    ///
    /// It is assumed that the entity exists.
    pub fn local_net_id(&self, entity: Entity) -> EntityNet {
        let player = self.config.locals().playable();
        EntityNet::new(player, entity.index())
    }
}

#[derive(SystemParam)]
pub struct NetEntityCommands<'w> {
    entities: &'w Entities,
    map: ResMut<'w, EntityIdMapRes>,
}

impl<'w> NetEntityCommands<'w> {
    fn register(&mut self, remote: EntityNet, local: Entity) {
        self.map.register(remote, local)
    }

    fn deregister(&mut self, remote: EntityNet) -> Entity {
        self.map.deregister(remote)
    }

    fn local_id(&self, entity: EntityNet) -> Option<Entity> {
        self.map
            .translate_remote(entity)
            .or_else(|| self.entities.resolve_from_id(entity.index()))
    }
}

/// Mapping between remote and local entity IDs for non-locally simulated
/// entities.
#[derive(Resource)]
struct EntityIdMapRes {
    remote_to_local: AHashMap<EntityNet, Entity>,
    local_to_remote: AHashMap<Entity, EntityNet>,
}

impl EntityIdMapRes {
    fn new() -> Self {
        Self {
            remote_to_local: AHashMap::new(),
            local_to_remote: AHashMap::new(),
        }
    }

    /// Registers a new remote entity.
    ///
    /// # Arguments
    ///
    /// * `remote` - remote entity identification.
    ///
    /// * `local` - local entity (present in the local ECS).
    ///
    /// # Panics
    ///
    /// Panics if the remote entity is already registered.
    fn register(&mut self, remote: EntityNet, local: Entity) {
        let result = self.remote_to_local.insert(remote, local);
        assert!(result.is_none());
        let result = self.local_to_remote.insert(local, remote);
        assert!(result.is_none());
    }

    /// De-registers an existing remote entity.
    ///
    /// See [`Self::register`].
    ///
    /// # Panics
    ///
    /// Panics if the entity is not registered.
    fn deregister(&mut self, remote: EntityNet) -> Entity {
        let local = self.remote_to_local.remove(&remote).unwrap();
        self.local_to_remote.remove(&local).unwrap();
        local
    }

    /// Translates local entity ID to a remote entity ID in case the entity is
    /// not locally simulated.
    fn translate_local(&self, local: Entity) -> Option<EntityNet> {
        self.local_to_remote.get(&local).copied()
    }

    /// Translates remote entity ID to a local entity ID in case the entity is
    /// not locally simulated.
    fn translate_remote(&self, remote: EntityNet) -> Option<Entity> {
        self.remote_to_local.get(&remote).copied()
    }
}

fn setup(mut commands: Commands) {
    commands.insert_resource(EntityIdMapRes::new());
}

fn cleanup(mut commands: Commands) {
    commands.remove_resource::<EntityIdMapRes>();
}

fn recv_messages(
    mut commands: Commands,
    mut net_commands: NetEntityCommands,
    mut inputs: EventReader<FromPlayersEvent>,
    mut spawn_events: EventWriter<NetRecvSpawnActiveEvent>,
    mut despawn_events: EventWriter<NetRecvDespawnActiveEvent>,
    mut health_events: EventWriter<NetRecvHealthEvent>,
) {
    for input in inputs.iter() {
        match input.message() {
            ToPlayers::Spawn {
                entity,
                player,
                object_type,
                transform,
            } => {
                let local = commands.spawn_empty().id();
                net_commands.register(*entity, local);

                spawn_events.send(NetRecvSpawnActiveEvent::new(
                    *player,
                    local,
                    *object_type,
                    transform.into(),
                ));
            }
            ToPlayers::Despawn { entity } => {
                let local = net_commands.deregister(*entity);
                despawn_events.send(NetRecvDespawnActiveEvent::new(local));
            }
            ToPlayers::ChangeHealth { entity, delta } => {
                let Some(local) = net_commands.local_id(*entity) else {
                    warn!("Received net health update of unrecognized entity: {entity:?}");
                    continue;
                };

                health_events.send(NetRecvHealthEvent::new(local, delta.into()));
            }
            _ => (),
        }
    }
}