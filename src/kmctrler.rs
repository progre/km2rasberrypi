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
