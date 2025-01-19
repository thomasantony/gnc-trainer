// src/ui/messages.rs
use bevy::prelude::*;

#[derive(Event, Clone, Debug)]
pub enum UiToGame {
    UpdateCode(String),
    RunSimulation,
    ResetSimulation,
}

#[derive(Event, Clone, Debug)]
pub enum GameToUi {
    ConsoleOutput(String),
    SimulationStatus {
        running: bool,
        crashed: bool,
    },
    TelemetryUpdate {
        position: Vec2,
        velocity: Vec2,
        fuel: f32,
    },
}

// Wrapper types that we can implement PartialEq for
#[derive(Clone)]
pub struct UiSender(pub crossbeam_channel::Sender<UiToGame>);
#[derive(Clone)]
pub struct UiReceiver(pub crossbeam_channel::Receiver<GameToUi>);

// Simple equality - channels are always equal for UI purposes
impl PartialEq for UiSender {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl PartialEq for UiReceiver {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

// For thread-safe communication between UI and Bevy
#[derive(Resource)]
pub struct UiEventChannel {
    pub ui_sender: crossbeam_channel::Sender<GameToUi>,
    pub game_receiver: crossbeam_channel::Receiver<UiToGame>,
}

// Channel type used by the UI side
#[derive(Clone, PartialEq)]
pub struct GameEventChannel {
    pub game_sender: UiSender,
    pub ui_receiver: UiReceiver,
}

impl UiEventChannel {
    pub fn new() -> (Self, GameEventChannel) {
        let (game_tx, game_rx) = crossbeam_channel::unbounded();
        let (ui_tx, ui_rx) = crossbeam_channel::unbounded();

        (
            Self {
                ui_sender: ui_tx,
                game_receiver: game_rx,
            },
            GameEventChannel {
                game_sender: UiSender(game_tx),
                ui_receiver: UiReceiver(ui_rx),
            },
        )
    }
}
