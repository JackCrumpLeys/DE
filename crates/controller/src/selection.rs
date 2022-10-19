use ahash::AHashSet;
use bevy::{ecs::system::SystemParam, prelude::*};
use de_core::{
    objects::{MovableSolid, ObjectType},
    stages::GameStage,
    state::GameState,
};
use de_objects::{IchnographyCache, ObjectCache};
use de_terrain::CircleMarker;
use iyes_loopless::prelude::*;

use crate::Labels;

pub(crate) struct SelectionPlugin;

impl Plugin for SelectionPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SelectEvent>().add_system_to_stage(
            GameStage::Input,
            update_selection
                .run_in_state(GameState::Playing)
                .after(Labels::InputUpdate),
        );
    }
}

pub(crate) struct SelectEvent {
    entities: Vec<Entity>,
    mode: SelectionMode,
}

impl SelectEvent {
    pub(crate) fn none(mode: SelectionMode) -> Self {
        Self {
            entities: Vec::new(),
            mode,
        }
    }

    pub(crate) fn single(entity: Entity, mode: SelectionMode) -> Self {
        Self {
            entities: vec![entity],
            mode,
        }
    }

    pub(crate) fn many(entities: Vec<Entity>, mode: SelectionMode) -> Self {
        Self { entities, mode }
    }

    fn entities(&self) -> &[Entity] {
        self.entities.as_slice()
    }

    fn mode(&self) -> SelectionMode {
        self.mode
    }
}

#[derive(Component)]
pub(crate) struct Selected;

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum SelectionMode {
    Replace,
    /// Toggle selection for all updated entities, and keep other entities
    /// untouched.
    AddToggle,
}

#[derive(SystemParam)]
struct Selector<'w, 's> {
    commands: Commands<'w, 's>,
    cache: Res<'w, ObjectCache>,
    selected: Query<'w, 's, Entity, With<Selected>>,
    movable: Query<'w, 's, &'static ObjectType, With<MovableSolid>>,
}

impl<'w, 's> Selector<'w, 's> {
    fn select(&mut self, entities: &[Entity], mode: SelectionMode) {
        let selected: AHashSet<Entity> = self.selected.iter().collect();
        let updated: AHashSet<Entity> = entities.iter().cloned().collect();

        let (select, deselect): (AHashSet<Entity>, AHashSet<Entity>) = match mode {
            SelectionMode::Replace => (&updated - &selected, &selected - &updated),
            SelectionMode::AddToggle => (&updated - &selected, &updated & &selected),
        };

        for entity in deselect {
            let mut entity_commands = self.commands.entity(entity);
            entity_commands.remove::<Selected>();
            if self.movable.contains(entity) {
                entity_commands.remove::<CircleMarker>();
            }
        }

        for entity in select {
            let mut entity_commands = self.commands.entity(entity);
            entity_commands.insert(Selected);
            if let Ok(&object_type) = self.movable.get(entity) {
                let radius = self.cache.get_ichnography(object_type).radius();
                entity_commands.insert(CircleMarker::new(radius));
            }
        }
    }
}

fn update_selection(mut events: EventReader<SelectEvent>, mut selector: Selector) {
    for event in events.iter() {
        selector.select(event.entities(), event.mode());
    }
}
