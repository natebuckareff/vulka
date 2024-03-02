use super::{Control, InputKind, InputValue, RawDeviceId, RawEvent};
use enumflags2::BitFlags;
use gilrs::{Event, EventType};

#[derive(Debug)]
pub struct RawGamepadEvent {
    pub device_id: gilrs::GamepadId,
    pub event: gilrs::EventType,
}

impl RawGamepadEvent {
    pub fn from_gilrs_event(event: Event) -> Self {
        RawGamepadEvent {
            device_id: event.id,
            event: event.event,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GamepadControl {
    Connection,
    Button(gilrs::Button),
    Axis(gilrs::Axis),
}

impl RawEvent<RawDeviceId> for RawGamepadEvent {
    type Control = GamepadControl;

    fn get_device_id(&self) -> RawDeviceId {
        RawDeviceId::Gamepad(self.device_id)
    }

    fn get_control(&self) -> Self::Control {
        match self.event {
            EventType::ButtonPressed(button, _) => GamepadControl::Button(button),
            EventType::ButtonRepeated(_, _) => todo!(),
            EventType::ButtonReleased(button, _) => GamepadControl::Button(button),
            EventType::ButtonChanged(button, _, _) => GamepadControl::Button(button),
            EventType::AxisChanged(axis, _, _) => GamepadControl::Axis(axis),
            EventType::Connected => GamepadControl::Connection,
            EventType::Disconnected => GamepadControl::Connection,
            EventType::Dropped => todo!(),
        }
    }

    fn get_input_value(&self) -> InputValue {
        match self.event {
            EventType::ButtonPressed(_, _) => InputValue::Digital(true),
            EventType::ButtonRepeated(_, _) => todo!(),
            EventType::ButtonReleased(_, _) => InputValue::Digital(false),
            EventType::ButtonChanged(_, value, _) => InputValue::Analog(f64::from(value)),
            EventType::AxisChanged(_, value, _) => InputValue::Analog(f64::from(value)),
            EventType::Connected => InputValue::Digital(true),
            EventType::Disconnected => InputValue::Digital(false),
            EventType::Dropped => todo!(),
        }
    }
}

impl Control for GamepadControl {
    fn kind(&self) -> BitFlags<InputKind> {
        match self {
            GamepadControl::Connection => InputKind::Digital.into(),
            GamepadControl::Button(_) => InputKind::Digital | InputKind::Analog,
            GamepadControl::Axis(_) => InputKind::Analog.into(),
        }
    }
}
