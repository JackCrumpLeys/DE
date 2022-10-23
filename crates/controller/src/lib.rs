//! This crate implements handling of user input.

use areaselect::AreaSelectPlugin;
use bevy::{app::PluginGroupBuilder, prelude::*};
use command::CommandPlugin;
use draft::DraftPlugin;
use dragselect::DragSelectPlugin;
use mouse::MousePlugin;
use pointer::PointerPlugin;
use selection::SelectionPlugin;

mod areaselect;
mod command;
mod draft;
mod dragselect;
mod frustum;
mod keyboard;
mod mouse;
mod pointer;
mod selection;

pub struct ControllerPluginGroup;

impl PluginGroup for ControllerPluginGroup {
    fn build(&mut self, group: &mut PluginGroupBuilder) {
        group
            .add(DragSelectPlugin)
            .add(AreaSelectPlugin)
            .add(MousePlugin)
            .add(PointerPlugin)
            .add(CommandPlugin)
            .add(SelectionPlugin)
            .add(DraftPlugin);
    }
}

#[derive(Copy, Clone, Hash, Debug, PartialEq, Eq, SystemLabel)]
enum Labels {
    PreInputUpdate,
    InputUpdate,
}
