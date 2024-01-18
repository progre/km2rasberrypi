pub mod v1;
pub mod v2;
pub mod v3;

pub enum Event {
    Noteon(u8, u8, u8),
    Noteoff(u8, u8),
    AllNotesOff(u8),
    ProgramChange(u8, u8),
    Tuning(i32),
    HoldOn(u8),
    HoldOff(u8),
    ModulationOn(u8),
    ModulationOff(u8),
    ReverbOn(u8),
    ReverbOff(u8),
    ChorusOn(u8),
    ChorusOff(u8),
}
