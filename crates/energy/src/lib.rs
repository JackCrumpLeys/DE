mod battery;

use bevy::{app::PluginGroupBuilder, prelude::PluginGroup};
pub use battery::component::Battery;

pub struct EnergyPluginGroup;

impl PluginGroup for EnergyPluginGroup {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>().add(battery::BatteryPlugin)
    }
}
