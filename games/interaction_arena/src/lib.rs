// Interaction Arena: game semantics + classical game theory strategy game.
//
// 10 levels teach arenas, strategies, well-bracketing, innocence,
// composition, Nash equilibria, and Abramsky's semantic cube.

use bevy::prelude::*;

pub mod builder;
pub mod execution;
pub mod states;
pub mod ui;

pub struct InteractionArenaPlugin;

impl Plugin for InteractionArenaPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(states::InteractionStatesPlugin)
            .add_plugins(builder::BuilderPlugin)
            .add_plugins(execution::ExecutionPlugin)
            .add_plugins(ui::InteractionUiPlugin);
    }
}
