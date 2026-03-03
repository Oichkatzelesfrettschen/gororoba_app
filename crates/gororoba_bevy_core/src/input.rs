// Game input abstractions.
//
// Maps raw Bevy keyboard/mouse/gamepad input to semantic game actions,
// decoupling game logic from specific key bindings.

use bevy::prelude::*;

pub struct GameInputPlugin;

impl Plugin for GameInputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InputBindings>()
            .add_message::<GameAction>()
            .add_systems(Update, input_action_system);
    }
}

/// Semantic game actions emitted as messages.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameAction {
    Pause,
    ToggleHud,
    TogglePedagogy,
    Interact,
}

/// Maps key codes to game actions.
#[derive(Resource)]
pub struct InputBindings {
    pub bindings: Vec<(KeyCode, GameAction)>,
}

impl Default for InputBindings {
    fn default() -> Self {
        Self {
            bindings: vec![
                (KeyCode::Escape, GameAction::Pause),
                (KeyCode::F1, GameAction::ToggleHud),
                (KeyCode::F2, GameAction::TogglePedagogy),
                (KeyCode::KeyF, GameAction::Interact),
            ],
        }
    }
}

fn input_action_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    bindings: Res<InputBindings>,
    mut actions: MessageWriter<GameAction>,
) {
    for (key, action) in &bindings.bindings {
        if keyboard.just_pressed(*key) {
            actions.write(*action);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_bindings_has_expected_actions() {
        let bindings = InputBindings::default();
        let actions: Vec<_> = bindings.bindings.iter().map(|(_, a)| *a).collect();
        assert!(actions.contains(&GameAction::Pause));
        assert!(actions.contains(&GameAction::ToggleHud));
        assert!(actions.contains(&GameAction::TogglePedagogy));
        assert!(actions.contains(&GameAction::Interact));
    }
}
