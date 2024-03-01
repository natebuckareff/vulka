use super::{InputValue, RawDeviceId, RawEvent};

#[derive(Debug)]
pub struct RawKeyboardEvent {
    pub device_id: winit::event::DeviceId,
    pub event: winit::event::KeyEvent,
}

impl RawEvent<RawDeviceId> for RawKeyboardEvent {
    type RawControl = winit::keyboard::PhysicalKey;

    fn get_device_id(&self) -> RawDeviceId {
        RawDeviceId::Keyboard(self.device_id)
    }

    fn get_raw_control(&self) -> Self::RawControl {
        self.event.physical_key
    }

    fn get_input_value(&self) -> InputValue {
        InputValue::Digital(self.event.state.is_pressed())
    }
}
