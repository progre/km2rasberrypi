use getset::{CopyGetters, Getters};

#[derive(CopyGetters, Default, Getters)]
pub struct State {
    #[get = "pub"]
    keys: [bool; 24],
    #[get_copy = "pub"]
    wheel_up: bool,
    #[get_copy = "pub"]
    wheel_down: bool,
    #[get_copy = "pub"]
    start: bool,
    #[get_copy = "pub"]
    select: bool,
}

impl State {
    pub fn update(&mut self, ev: &Event) {
        match ev {
            Event::Press(Input::Key(key)) => self.keys[*key as usize] = true,
            Event::Release(Input::Key(key)) => self.keys[*key as usize] = false,
            Event::Press(Input::WheelUp) => self.wheel_up = true,
            Event::Release(Input::WheelUp) => self.wheel_up = false,
            Event::Press(Input::WheelDown) => self.wheel_down = true,
            Event::Release(Input::WheelDown) => self.wheel_down = false,
            Event::Press(Input::Start) => self.start = true,
            Event::Release(Input::Start) => self.start = false,
            Event::Press(Input::Select) => self.select = true,
            Event::Release(Input::Select) => self.select = false,
        }
    }

    pub fn reset_select_start(&mut self) {
        self.select = false;
        self.start = false;
    }
}

pub enum Input {
    Key(u8),
    WheelUp,
    WheelDown,
    Start,
    Select,
}

impl Input {
    pub fn from_raw(value: u16) -> Option<Self> {
        match value {
            304..=316 => Some(Self::Key((value - 304) as u8)),
            317 => Some(Self::Select),
            318..=320 => Some(Self::Key((value - 318 + 13) as u8)),
            704..=707 => Some(Self::Key((value - 704 + 15) as u8)),
            708 => Some(Self::Start),
            709..=713 => Some(Self::Key((value - 709 + 19) as u8)),
            714 => Some(Self::WheelUp),
            715 => Some(Self::WheelDown),
            745..=750 => None,
            _ => panic!("{value} is not a valid input"),
        }
    }
}

pub enum Event {
    Press(Input),
    Release(Input),
}
