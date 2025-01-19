use bevy::prelude::*;
use monaco::{api::CodeEditorOptions, sys::editor::BuiltinTheme, yew::CodeEditor};
use std::rc::Rc;
use yew::{html, Component, Context, Html, Properties};

// Message types for Bevy<->Yew communication
#[derive(Clone, Debug)]
pub enum UiToGame {
    UpdateCode(String),
    RunSimulation,
    ResetSimulation,
}

#[derive(Clone, Debug)]
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

// Props for our main App component
#[derive(Properties, Clone, PartialEq)]
pub struct AppProps {
    pub event_handle: EventHandle,
}

// Main App component
pub struct App {
    options: Rc<CodeEditorOptions>,
    console_output: Vec<String>,
    editor_value: String,
    simulation_running: bool,
    props: AppProps,
}

pub enum AppMsg {
    CodeChanged(String),
    ConsoleMessage(String),
    RunClicked,
    ResetClicked,
    TelemetryUpdate(GameToUi),
}

impl Component for App {
    type Message = AppMsg;
    type Properties = AppProps;

    fn create(ctx: &Context<Self>) -> Self {
        let options = CodeEditorOptions::default()
            .with_language("javascript".to_owned())
            .with_value(String::new())
            .with_builtin_theme(BuiltinTheme::VsDark)
            .with_automatic_layout(true);

        Self {
            options: Rc::new(options),
            console_output: Vec::new(),
            editor_value: String::new(),
            simulation_running: false,
            props: ctx.props().clone(),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            AppMsg::CodeChanged(code) => {
                self.editor_value = code.clone();
                // Send to Bevy
                self.props
                    .event_handle
                    .sender
                    .try_send(UiToGame::UpdateCode(code))
                    .ok();
                true
            }
            AppMsg::ConsoleMessage(msg) => {
                self.console_output.push(msg);
                true
            }
            AppMsg::RunClicked => {
                self.simulation_running = !self.simulation_running;
                self.props
                    .event_handle
                    .sender
                    .try_send(if self.simulation_running {
                        UiToGame::RunSimulation
                    } else {
                        UiToGame::ResetSimulation
                    })
                    .ok();
                true
            }
            AppMsg::ResetClicked => {
                self.simulation_running = false;
                self.props
                    .event_handle
                    .sender
                    .try_send(UiToGame::ResetSimulation)
                    .ok();
                true
            }
            AppMsg::TelemetryUpdate(update) => {
                match update {
                    GameToUi::ConsoleOutput(msg) => {
                        self.console_output.push(msg);
                    }
                    GameToUi::SimulationStatus { running, crashed } => {
                        self.simulation_running = running;
                    }
                    _ => {}
                }
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let on_code_change = ctx.link().callback(AppMsg::CodeChanged);
        let on_run = ctx.link().callback(|_| AppMsg::RunClicked);
        let on_reset = ctx.link().callback(|_| AppMsg::ResetClicked);

        html! {
            <div class="flex w-full h-screen">
                // Left side - Bevy canvas
                <canvas id="bevy" class="w-1/2 h-full" />

                // Right side - Editor and controls
                <div class="w-1/2 h-full flex flex-col bg-gray-900 text-white p-4">
                    // Monaco editor
                    <div class="h-2/3">
                        <CodeEditor
                            classes="h-full"
                            options={self.options.to_sys_options()}
                            on_change={on_code_change}
                        />
                    </div>

                    // Console output
                    <div class="h-1/4 mt-4 bg-gray-800 p-2 overflow-y-auto font-mono">
                        { for self.console_output.iter().map(|msg| {
                            html! { <div class="text-green-400">{ msg }</div> }
                        })}
                    </div>

                    // Control buttons
                    <div class="mt-4 flex space-x-4">
                        <button
                            onclick={on_run}
                            class="bg-blue-600 px-4 py-2 rounded">
                            { if self.simulation_running { "Pause" } else { "Run" } }
                        </button>
                        <button
                            onclick={on_reset}
                            class="bg-gray-600 px-4 py-2 rounded">
                            { "Reset" }
                        </button>
                    </div>
                </div>
            </div>
        }
    }
}

// Bevy plugin for handling UI communication
pub struct UiPlugin {
    pub handle: EventHandle,
}

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.handle.clone())
            .add_event::<UiToGame>()
            .add_event::<GameToUi>()
            .add_systems(Update, (handle_ui_messages, send_telemetry));
    }
}

// System to handle messages from UI
fn handle_ui_messages(
    mut commands: Commands,
    handle: Res<EventHandle>,
    mut events: EventWriter<UiToGame>,
) {
    while let Ok(msg) = handle.receiver.try_recv() {
        events.send(msg);
    }
}

// System to send telemetry to UI
fn send_telemetry(handle: Res<EventHandle>, mut events: EventReader<GameToUi>) {
    for event in events.read() {
        handle.sender.try_send(event.clone()).ok();
    }
}
