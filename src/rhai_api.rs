use bevy::prelude::*;
use rhai::{Dynamic, Engine, Map as RhaiMap, Scope, AST};
use std::sync::Arc;

#[derive(Clone)]
pub struct SimpleControl {
    pub thrust: f32,
}

#[derive(Clone)]
pub struct VectoredControl {
    pub thrust: f32,
    pub gimbal: f32,
}

#[derive(Clone)]
pub enum ControlOutput {
    Simple(SimpleControl),
    Vectored(VectoredControl),
}

#[derive(Clone)]
pub struct LanderState {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub rotation: f32,
    pub angular_vel: f32, // Added angular velocity
    pub fuel: f32,
}

#[derive(Resource)]
pub struct ScriptEngine {
    engine: Arc<Engine>,
    compiled_script: Option<Arc<AST>>,
    pub error_message: Option<String>,
    pub control_type: ControlType,
    pub user_state: RhaiMap,         // Persistent user state
    pub console_buffer: Vec<String>, // Console output buffer
}

#[derive(Clone)]
pub enum ControlType {
    Simple,
    Vectored,
}

impl Default for ScriptEngine {
    fn default() -> Self {
        let mut engine = Engine::new();

        // Register types
        engine.register_type::<SimpleControl>();
        engine.register_type::<VectoredControl>();

        // Register control functions
        engine.register_fn("simple_control", |thrust: f64| -> Dynamic {
            let control = SimpleControl {
                thrust: thrust as f32,
            };
            Dynamic::from(control)
        });

        engine.register_fn("vector_control", |thrust: f64, gimbal: f64| -> Dynamic {
            let control = VectoredControl {
                thrust: thrust as f32,
                gimbal: gimbal as f32,
            };
            Dynamic::from(control)
        });

        // Register print function
        let print_fn = move |text: &str| {
            CONSOLE_BUFFER.with(|buffer| {
                buffer.borrow_mut().push(text.to_string());
            });
        };
        engine.register_fn("print", print_fn);

        // Disable unsafe operations
        engine.set_max_expr_depths(64, 64);
        engine.set_max_operations(100_000);
        engine.set_max_modules(0);
        engine.set_max_string_size(1_000_000); // Allow larger strings for console output
        engine.disable_symbol("eval");

        Self {
            engine: Arc::new(engine),
            compiled_script: None,
            error_message: None,
            control_type: ControlType::Simple,
            user_state: RhaiMap::new(),
            console_buffer: Vec::new(),
        }
    }
}

// Thread-local storage for console buffer during script execution
thread_local! {
    static CONSOLE_BUFFER: std::cell::RefCell<Vec<String>> = std::cell::RefCell::new(Vec::new());
}

impl ScriptEngine {
    pub fn set_control_type(&mut self, control_type: ControlType) {
        self.control_type = control_type;
    }

    pub fn compile_script(&mut self, script: &str) -> Result<(), String> {
        self.error_message = None;
        match self.engine.compile(script) {
            Ok(ast) => {
                self.compiled_script = Some(Arc::new(ast));
                Ok(())
            }
            Err(e) => {
                let error = format!("Compilation error: {}", e);
                self.error_message = Some(error.clone());
                Err(error)
            }
        }
    }

    pub fn calculate_control(&mut self, state: LanderState) -> Option<ControlOutput> {
        if let Some(ast) = &self.compiled_script {
            // Clear console buffer for this execution
            CONSOLE_BUFFER.with(|buffer| buffer.borrow_mut().clear());

            // Create state map
            let mut map = RhaiMap::new();
            map.insert("x".into(), Dynamic::from_float(state.x as f64));
            map.insert("y".into(), Dynamic::from_float(state.y as f64));
            map.insert("vx".into(), Dynamic::from_float(state.vx as f64));
            map.insert("vy".into(), Dynamic::from_float(state.vy as f64));
            map.insert(
                "rotation".into(),
                Dynamic::from_float(state.rotation as f64),
            );
            map.insert(
                "angular_vel".into(),
                Dynamic::from_float(state.angular_vel as f64),
            );
            map.insert("fuel".into(), Dynamic::from_float(state.fuel as f64));

            // Create scope with state and user_state
            let mut scope = Scope::new();
            scope.push("state", map.clone());
            scope.push("user_state", self.user_state.clone());

            // Execute script
            match self.engine.eval_ast_with_scope::<Dynamic>(&mut scope, ast) {
                Ok(scope_copy) => {
                    // Extract updated user_state
                    if let Some(new_user_state) = scope_copy.try_cast::<RhaiMap>() {
                        self.user_state = new_user_state;
                    }

                    // Get console output
                    CONSOLE_BUFFER.with(|buffer| {
                        self.console_buffer.extend(buffer.borrow().iter().cloned());
                    });

                    // Call control function with current state
                    match self
                        .engine
                        .call_fn::<Dynamic>(&mut scope, &ast, "control", (map,))
                    {
                        Ok(result) => match self.control_type {
                            ControlType::Simple => {
                                let control: SimpleControl = result.cast();
                                Some(ControlOutput::Simple(control))
                            }
                            ControlType::Vectored => {
                                let control: VectoredControl = result.cast();
                                Some(ControlOutput::Vectored(control))
                            }
                        },
                        Err(e) => {
                            let error = format!("Runtime error: {}", e);
                            self.error_message = Some(error);
                            None
                        }
                    }
                }
                Err(e) => {
                    let error = format!("Runtime error: {}", e);
                    self.error_message = Some(error);
                    None
                }
            }
        } else {
            None
        }
    }

    pub fn take_console_output(&mut self) -> Vec<String> {
        std::mem::take(&mut self.console_buffer)
    }
}
