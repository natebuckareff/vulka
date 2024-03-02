use enumflags2::{bitflags, BitFlags};
use std::cell::OnceCell;
use std::collections::HashMap;
use std::hash::Hash;
use std::time::{Duration, Instant};

pub trait DeviceId: Clone + Copy + PartialEq + Eq {
    type Kind;
    fn kind(&self) -> Self::Kind;
}

#[derive(Debug, Clone, Copy)]
pub enum InputValue {
    Digital(bool),
    Analog(f64),
    Analog2d(f64, f64),
}

#[bitflags]
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputKind {
    Digital,
    Analog,
    Analog2d,
}

impl InputValue {
    pub fn kind(&self) -> InputKind {
        match &self {
            InputValue::Digital(_) => InputKind::Digital,
            InputValue::Analog(_) => InputKind::Analog,
            InputValue::Analog2d(_, _) => InputKind::Analog2d,
        }
    }
}

#[derive(Debug)]
pub struct InputEvent<DId: DeviceId, Action: Hash> {
    pub index: u64,
    pub created_at: Duration,
    pub device_id: DId,
    pub action: Action,
    pub value: InputValue,
}

pub trait RawEvent<DId: DeviceId> {
    type Control: Control;
    fn get_device_id(&self) -> DId;
    fn get_control(&self) -> Self::Control;
    fn get_input_value(&self) -> InputValue;
}

pub trait Control: Copy + Clone + Eq + PartialEq + Hash {
    fn kind(&self) -> BitFlags<InputKind>;
}

pub struct InputManager<DId, REvent, Action>
where
    DId: DeviceId,
    REvent: RawEvent<DId>,
    Action: Copy + Clone + Eq + Hash,
{
    start_time: Instant,
    control_map: HashMap<REvent::Control, (Action, Option<BitFlags<InputKind>>)>,
    control_map_rev: HashMap<Action, REvent::Control>,
    wildcard_actions: Vec<(Action, Option<BitFlags<InputKind>>)>,
    input_events: Vec<InputEvent<DId, Action>>,
    next_index: u64,
}

impl<DId, REvent, Action> InputManager<DId, REvent, Action>
where
    DId: DeviceId,
    REvent: RawEvent<DId>,
    Action: Copy + Clone + Eq + Hash,
{
    pub fn new(start_time: Instant) -> Self {
        Self {
            start_time,
            control_map: HashMap::new(),
            control_map_rev: HashMap::new(),
            wildcard_actions: vec![],
            input_events: vec![],
            next_index: 0,
        }
    }

    pub fn set_action(
        &mut self,
        control: REvent::Control,
        action: Action,
        mask: Option<BitFlags<InputKind>>,
    ) {
        self.control_map.insert(control, (action, mask));
        self.control_map_rev.insert(action, control);
    }

    pub fn set_wildcard_action(&mut self, action: Action, mask: Option<BitFlags<InputKind>>) {
        self.wildcard_actions.push((action, mask));
    }

    pub fn get_action(
        &self,
        control: &REvent::Control,
    ) -> Option<&(Action, Option<BitFlags<InputKind>>)> {
        self.control_map.get(control)
    }

    pub fn get_control(&self, action: &Action) -> Option<&REvent::Control> {
        self.control_map_rev.get(action)
    }

    pub fn update(&mut self, raw_event: &REvent) -> usize {
        let mut count: usize = 0;
        let device_id = raw_event.get_device_id();
        let raw_control = raw_event.get_control();
        let control_action = self.control_map.get(&raw_control);
        let value: OnceCell<InputValue> = OnceCell::new();

        if let Some((action, mask)) = control_action {
            let value = value.get_or_init(|| raw_event.get_input_value());
            if Self::_push_input_event(
                &mut self.next_index,
                &mut self.input_events,
                self.start_time.elapsed(),
                device_id,
                *action,
                *value,
                mask,
            ) {
                count += 1;
            }
        }

        for (action, mask) in &self.wildcard_actions {
            let value = value.get_or_init(|| raw_event.get_input_value());
            if Self::_push_input_event(
                &mut self.next_index,
                &mut self.input_events,
                self.start_time.elapsed(),
                device_id,
                *action,
                *value,
                mask,
            ) {
                count += 1;
            }
        }

        count
    }

    pub fn get_input_event_count(&self) -> usize {
        self.input_events.len()
    }

    pub fn get_nth_last_input_event(&self, offset: usize) -> Option<&InputEvent<DId, Action>> {
        if (offset + 1) > self.input_events.len() {
            return None;
        }
        Some(&self.input_events[self.input_events.len() - (offset + 1)])
    }

    pub fn flush_input_events(&mut self) {
        self.input_events.clear();
    }

    fn _push_input_event(
        next_index: &mut u64,
        input_events: &mut Vec<InputEvent<DId, Action>>,
        created_at: Duration,
        device_id: DId,
        action: Action,
        value: InputValue,
        mask: &Option<BitFlags<InputKind>>,
    ) -> bool {
        if let Some(mask) = mask {
            if !mask.intersects(value.kind()) {
                return false;
            }
        }
        let index = *next_index;
        *next_index += 1;
        input_events.push(InputEvent {
            index,
            created_at,
            device_id,
            action,
            value,
        });
        true
    }
}
