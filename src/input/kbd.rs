use super::{Control, InputKind, InputValue, RawDeviceId, RawEvent};
use enumflags2::BitFlags;

#[derive(Debug)]
pub struct RawKeyboardEvent {
    pub device_id: winit::event::DeviceId,
    pub event: winit::event::KeyEvent,
}

impl RawEvent<RawDeviceId> for RawKeyboardEvent {
    type Control = winit::keyboard::PhysicalKey;

    fn get_device_id(&self) -> RawDeviceId {
        RawDeviceId::Keyboard(self.device_id)
    }

    fn get_control(&self) -> Self::Control {
        self.event.physical_key
    }

    fn get_input_value(&self) -> InputValue {
        InputValue::Digital(self.event.state.is_pressed())
    }
}

impl Control for winit::keyboard::PhysicalKey {
    fn kind(&self) -> BitFlags<InputKind> {
        InputKind::Digital.into()
    }
}
