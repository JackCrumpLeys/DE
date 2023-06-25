mod battery;
mod graph;

pub use battery::Battery;
use bevy::{app::PluginGroupBuilder, prelude::PluginGroup};
pub use graph::EnergyReceiver;

use crate::battery::BatteryPlugin;
use crate::graph::PowerGridPlugin;

pub struct EnergyPluginGroup;

impl PluginGroup for EnergyPluginGroup {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(BatteryPlugin)
            .add(PowerGridPlugin)
    }
}
