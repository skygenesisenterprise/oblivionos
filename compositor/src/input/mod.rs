use std::collections::{HashMap, VecDeque};
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InputDeviceId(u32);

impl InputDeviceId {
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputDeviceType {
    Keyboard,
    Mouse,
    Touchpad,
    Touchscreen,
    Tablet,
}

#[derive(Debug, Clone)]
pub struct InputEvent {
    pub timestamp: Instant,
    pub device_id: InputDeviceId,
    pub event_type: InputEventType,
}

#[derive(Debug, Clone)]
pub enum InputEventType {
    KeyPress { key: u32, state: KeyState, modifiers: ModifiersState },
    MouseMotion { x: f64, y: f64, dx: f64, dy: f64 },
    MouseButton { button: u32, state: ButtonState },
    MouseWheel { axis: Axis, value: f64 },
    TouchDown { x: f64, y: f64, slot: u32 },
    TouchMove { x: f64, y: f64, dx: f64, dy: f64, slot: u32 },
    TouchUp { slot: u32 },
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ModifiersState {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub logo: bool,
    pub caps_lock: bool,
    pub num_lock: bool,
    pub scroll_lock: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyState {
    Pressed,
    Released,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonState {
    Pressed,
    Released,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone)]
pub struct KeyboardRepeatConfig {
    pub delay_ms: u32,
    pub interval_ms: u32,
}

pub struct InputHandler {
    devices: HashMap<InputDeviceId, InputDeviceType>,
    events: VecDeque<InputEvent>,
    focused_window: Option<super::window::WindowId>,
    pointer_position: (f64, f64),
    modifiers: ModifiersState,
    mouse_buttons: HashMap<u32, ButtonState>,
    keyboard_repeat: Option<KeyboardRepeatConfig>,
    enabled: bool,
}

impl InputHandler {
    pub fn new() -> Self {
        Self {
            devices: HashMap::new(),
            events: VecDeque::new(),
            focused_window: None,
            pointer_position: (0.0, 0.0),
            modifiers: ModifiersState::default(),
            mouse_buttons: HashMap::new(),
            keyboard_repeat: Some(KeyboardRepeatConfig { delay_ms: 500, interval_ms: 50 }),
            enabled: true,
        }
    }

    pub fn process(&mut self) {
        if !self.enabled {
            return;
        }
        while let Some(event) = self.events.pop_front() {
            self.handle_event(event);
        }
    }

    fn handle_event(&mut self, event: InputEvent) {
        match event.event_type {
            InputEventType::KeyPress { key: _, state: _, modifiers } => {
                self.modifiers = modifiers;
            }
            InputEventType::MouseMotion { x, y, .. } => {
                self.pointer_position = (x, y);
            }
            InputEventType::MouseButton { button, state } => {
                self.mouse_buttons.insert(button, state);
            }
            InputEventType::TouchDown { x, y, .. } | InputEventType::TouchMove { x, y, .. } => {
                self.pointer_position = (x, y);
            }
            _ => {}
        }
    }

    pub fn set_focus(&mut self, window: Option<super::window::WindowId>) {
        self.focused_window = window;
    }

    pub fn get_focus(&self) -> Option<super::window::WindowId> {
        self.focused_window
    }

    pub fn get_modifiers(&self) -> ModifiersState {
        self.modifiers
    }

    pub fn get_pointer_position(&self) -> (f64, f64) {
        self.pointer_position
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn register_device(&mut self, id: InputDeviceId, device_type: InputDeviceType) {
        self.devices.insert(id, device_type);
    }

    pub fn unregister_device(&mut self, id: InputDeviceId) {
        self.devices.remove(&id);
    }

    pub fn queue_event(&mut self, event: InputEvent) {
        self.events.push_back(event);
    }
}

impl Default for InputHandler {
    fn default() -> Self {
        Self::new()
    }
}