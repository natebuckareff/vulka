use std::collections::HashMap;
use std::hash::Hash;
use std::time::{Duration, Instant};

pub trait DeviceId: Clone + Copy + PartialEq + Eq {
    type Kind;
    fn kind(&self) -> Self::Kind;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ControlId(u64);

#[derive(Debug, Clone, Copy)]
pub enum InputValue {
    Digital(bool),
    Analog(f64),
    Analog2d(f64, f64),
}

#[derive(Debug)]
pub struct InputEvent<DId: DeviceId> {
    pub index: u64,
    pub created_at: Duration,
    pub device_id: DId,
    pub control_id: ControlId,
    pub value: InputValue,
}

pub trait RawEvent<DId: DeviceId> {
    type RawControl: Copy + Clone + Eq + PartialEq + Hash;
    fn get_device_id(&self) -> DId;
    fn get_raw_control(&self) -> Self::RawControl;
    fn get_input_value(&self) -> InputValue;
}

pub struct ControlManager<DId: DeviceId, REvent: RawEvent<DId>> {
    start_time: Instant,
    control_ids: HashMap<REvent::RawControl, ControlId>,
    control_ids_rev: HashMap<ControlId, REvent::RawControl>,
    next_control_id: u64,
    next_index: u64,
}

impl<DId: DeviceId, REvent: RawEvent<DId>> ControlManager<DId, REvent> {
    pub fn new(start_time: Instant) -> Self {
        Self {
            start_time,
            control_ids: HashMap::new(),
            control_ids_rev: HashMap::new(),
            next_control_id: 0,
            next_index: 0,
        }
    }

    pub fn get_control_id(&self, raw_control: &REvent::RawControl) -> Option<&ControlId> {
        self.control_ids.get(raw_control)
    }

    pub fn get_raw_control(&self, control_id: &ControlId) -> Option<&REvent::RawControl> {
        self.control_ids_rev.get(control_id)
    }

    pub fn get_input_event(&mut self, raw_event: &REvent) -> InputEvent<DId> {
        let index = self.next_index;
        self.next_index += 1;

        let device_id = raw_event.get_device_id();
        let control_id = self._get_or_init_control_id(&raw_event.get_raw_control());
        let value = raw_event.get_input_value();

        InputEvent {
            index,
            created_at: self.start_time.elapsed(),
            device_id,
            control_id,
            value,
        }
    }

    fn _get_or_init_control_id(&mut self, raw_control: &REvent::RawControl) -> ControlId {
        match self.control_ids.get(&raw_control) {
            None => {
                let control_id = ControlId(self.next_control_id);
                self.next_control_id += 1;
                self.control_ids.insert(*raw_control, control_id.clone());
                self.control_ids_rev.insert(control_id, raw_control.clone());
                control_id
            }
            Some(control_id) => *control_id,
        }
    }
}
