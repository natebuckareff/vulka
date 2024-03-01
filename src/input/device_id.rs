use super::DeviceId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawDeviceId {
    Keyboard(winit::event::DeviceId),
    Mouse(winit::event::DeviceId),
    Gamepad(gilrs::GamepadId),
}

#[derive(Debug)]
pub enum DeviceKind {
    Keyboard,
    Mouse,
    Gamepad,
}

impl DeviceId for RawDeviceId {
    type Kind = DeviceKind;
    fn kind(&self) -> DeviceKind {
        match self {
            RawDeviceId::Keyboard(_) => DeviceKind::Keyboard,
            RawDeviceId::Mouse(_) => DeviceKind::Mouse,
            RawDeviceId::Gamepad(_) => DeviceKind::Gamepad,
        }
    }
}
