use super::{Control, InputKind, InputValue, RawDeviceId, RawEvent};
use enumflags2::BitFlags;
use winit::dpi::PhysicalPosition;
use winit::event::{DeviceId, ElementState, MouseButton, MouseScrollDelta, WindowEvent};

#[derive(Debug)]
pub struct RawMouseEvent {
    pub device_id: DeviceId,
    pub data: RawMouseEventData,
}

#[derive(Debug)]
pub enum RawMouseEventData {
    Button(MouseButton, ElementState),
    Wheel(MouseScrollDelta),
    Move(PhysicalPosition<f64>),
    Entered,
    Left,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseControl {
    Button(MouseButton),
    Wheel,
    Cursor,
}

impl RawMouseEvent {
    pub fn from_window_event(event: WindowEvent) -> Self {
        match event {
            WindowEvent::CursorMoved {
                device_id,
                position,
            } => RawMouseEvent {
                device_id,
                data: RawMouseEventData::Move(position),
            },
            WindowEvent::MouseWheel {
                device_id, delta, ..
            } => RawMouseEvent {
                device_id,
                data: RawMouseEventData::Wheel(delta),
            },
            WindowEvent::MouseInput {
                device_id,
                state,
                button,
            } => RawMouseEvent {
                device_id,
                data: RawMouseEventData::Button(button, state),
            },
            WindowEvent::CursorEntered { device_id } => RawMouseEvent {
                device_id,
                data: RawMouseEventData::Entered,
            },
            WindowEvent::CursorLeft { device_id } => RawMouseEvent {
                device_id,
                data: RawMouseEventData::Left,
            },
            _ => panic!(),
        }
    }
}

impl RawEvent<RawDeviceId> for RawMouseEvent {
    type Control = MouseControl;

    fn get_device_id(&self) -> RawDeviceId {
        RawDeviceId::Mouse(self.device_id)
    }

    fn get_control(&self) -> Self::Control {
        match &self.data {
            RawMouseEventData::Button(button, _) => MouseControl::Button(*button),
            RawMouseEventData::Wheel { .. } => MouseControl::Wheel,
            RawMouseEventData::Move { .. } => MouseControl::Cursor,
            RawMouseEventData::Entered => MouseControl::Cursor,
            RawMouseEventData::Left => MouseControl::Cursor,
        }
    }

    fn get_input_value(&self) -> InputValue {
        match &self.data {
            RawMouseEventData::Button(_, state) => InputValue::Digital(state.is_pressed()),
            RawMouseEventData::Wheel(delta) => match &delta {
                MouseScrollDelta::LineDelta(x, y) => {
                    InputValue::Analog2d(f64::from(*x), f64::from(*y))
                }
                MouseScrollDelta::PixelDelta(position) => {
                    InputValue::Analog2d(position.x, position.y)
                }
            },
            RawMouseEventData::Move(position) => InputValue::Analog2d(position.x, position.y),
            RawMouseEventData::Entered => InputValue::Digital(true),
            RawMouseEventData::Left => InputValue::Digital(false),
        }
    }
}

impl Control for MouseControl {
    fn kind(&self) -> BitFlags<InputKind> {
        match &self {
            MouseControl::Button(_) => InputKind::Digital.into(),
            MouseControl::Wheel => InputKind::Analog2d.into(),
            MouseControl::Cursor => InputKind::Analog2d.into(),
        }
    }
}
