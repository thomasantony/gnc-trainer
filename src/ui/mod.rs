use bevy::prelude::*;
use monaco::{api::CodeEditor, api::CodeEditorOptions, sys::editor::BuiltinTheme};
use std::rc::Rc;
use wasm_bindgen::{JsCast, UnwrapThrowExt};
use web_sys::HtmlElement;
use yew::prelude::*;

pub mod messages;
use messages::*;

#[derive(Properties, PartialEq)]
pub struct AppProps {
    pub event_channel: GameEventChannel,
}

#[function_component(App)]
fn app(props: &AppProps) -> Html {
    let editor_ref = use_node_ref();
    let editor = use_state(|| None::<CodeEditor>);

    {
        let editor_ref = editor_ref.clone();
        let editor = editor.clone();

        use_effect_with((), move |_| {
            if let Some(element) = editor_ref.cast::<HtmlElement>() {
                let options = CodeEditorOptions::default()
                    .with_language("javascript".into())
                    .with_builtin_theme(BuiltinTheme::VsDark)
                    .with_value("// Your code here".into());

                editor.set(Some(CodeEditor::create(&element, Some(options))));
            }
            || ()
        });
    }

    // Grab the Bevy canvas and move it into our layout structure
    use_effect_with((), |_| {
        if let Some(window) = web_sys::window() {
            if let Some(document) = window.document() {
                // Find Bevy's canvas (it's the first one)
                if let Some(canvas) = document.query_selector("canvas").ok().flatten() {
                    // Add our layout classes/styles
                    canvas
                        .set_attribute("style", "width: 100%; height: 100%; display: block;")
                        .ok();
                    // Move it into our container
                    if let Some(container) = document.get_element_by_id("bevy-container") {
                        container.append_child(&canvas).ok();
                    }
                }
            }
        }
        || ()
    });

    let on_code_change = {
        let sender = props.event_channel.game_sender.clone();
        Callback::from(move |code: String| {
            sender.0.try_send(UiToGame::UpdateCode(code)).ok();
        })
    };

    html! {
        <div class="root" style="display: flex; width: 100%; height: 100vh;">
            <div id="bevy-container" class="left-panel" style="width: 50%; height: 100vh;">
                // Bevy's canvas will be moved here
            </div>
            <div class="right-panel" style="width: 50%; height: 100vh; display: flex; flex-direction: column; background-color: #1e1e1e; color: white;">
                <h1 style="padding: 16px; margin: 0;">{"GNC Trainer"}</h1>

                <div class="editor-container" style="flex-grow: 1; min-height: 0; margin: 16px; position: relative;">
                    <div ref={editor_ref}
                         style="position: absolute; left: 0; top: 0; right: 0; bottom: 0; height: 100%; width: 100%;" />
                </div>

                <div class="console" style="height: 150px; margin: 16px; background-color: #2d2d2d; overflow: auto;">
                    <div style="padding: 8px; font-family: monospace;">
                        {"Console output will go here"}
                    </div>
                </div>

                <div class="controls" style="display: flex; gap: 8px; margin: 16px; margin-top: 0;">
                    <button style="flex: 1; background-color: #4a4a4a; color: white; padding: 8px 16px;
                                 border: none; border-radius: 4px; cursor: pointer;">
                        {"Run"}
                    </button>
                    <button style="flex: 1; background-color: #4a4a4a; color: white; padding: 8px 16px;
                                 border: none; border-radius: 4px; cursor: pointer;">
                        {"Reset"}
                    </button>
                </div>
            </div>
        </div>
    }
}

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        let (channel, ui_channel) = UiEventChannel::new();

        app.add_event::<UiToGame>()
            .add_event::<GameToUi>()
            .insert_resource(channel)
            .add_systems(Update, (handle_ui_messages, handle_game_messages));

        #[cfg(target_arch = "wasm32")]
        {
            yew::Renderer::<App>::with_props(AppProps {
                event_channel: ui_channel,
            })
            .render();
        }
    }
}

fn handle_ui_messages(channel: Res<UiEventChannel>, mut events: EventWriter<UiToGame>) {
    while let Ok(msg) = channel.game_receiver.try_recv() {
        events.send(msg);
    }
}

fn handle_game_messages(channel: Res<UiEventChannel>, mut events: EventReader<GameToUi>) {
    for event in events.read() {
        let _ = channel.ui_sender.try_send(event.clone());
    }
}

#[cfg(target_arch = "wasm32")]
pub fn start_ui() {
    let (channel, ui_channel) = UiEventChannel::new();

    yew::Renderer::<App>::with_props(AppProps {
        event_channel: ui_channel,
    })
    .render();
}
