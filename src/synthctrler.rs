use std::{
    collections::HashMap,
    sync::mpsc::{self, RecvError},
};

use crate::{
    kmctrler::{self, Input},
    settings::SynthesizerSettings,
};

pub enum Event {
    Noteon(u8, u16),
    Noteoff(u8, u16),
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

/// キーボード全体演奏と排他
///   チューニング .... Select + Start + Key
/// キーボードごと演奏と排他
///   プログラムチェンジ .... Start + Key
///   オクターブシフト .... Select + Key
///   リバーブ(toggle) .... Select + Start + WheelUp
///   コーラス(toggle) .... Select + Start + WheelDown
/// キーボードごと演奏と同時
///   ホールド .... WheelDown
///   モジュレーション(ビブラート) .... WheelUp
pub struct SynthCtrler {
    rx: mpsc::Receiver<(usize, kmctrler::Event)>,
    settings: SynthesizerSettings,
    buf_programs: HashMap<u8, u8>,
    buf_select: u32,
    buf_start: u32,
}

impl SynthCtrler {
    pub fn new(rx: mpsc::Receiver<(usize, kmctrler::Event)>) -> Self {
        Self {
            rx,
            settings: SynthesizerSettings::load(),
            buf_programs: HashMap::new(),
            buf_select: 0,
            buf_start: 0,
        }
    }

    pub fn init(&mut self) -> Vec<Event> {
        self.settings
            .keyboards()
            .iter()
            .enumerate()
            .flat_map(|(chan, keyboard)| {
                let chan = chan as u8;
                [
                    Event::ProgramChange(chan, keyboard.program_no()),
                    if keyboard.reverb() {
                        Event::ReverbOn(chan)
                    } else {
                        Event::ReverbOff(chan)
                    },
                    if keyboard.chorus() {
                        Event::ChorusOn(chan)
                    } else {
                        Event::ChorusOff(chan)
                    },
                ]
            })
            .collect()
    }

    fn program_change(&mut self, chan: u8, key: u8) -> Option<Event> {
        let bit = match key {
            0 => 0b_01000000,
            2 => 0b_00100000,
            4 => 0b_00010000,
            5 => 0b_00001000,
            7 => 0b_00000100,
            9 => 0b_00000010,
            11 => 0b00000001,
            _ => return None,
        };
        self.buf_programs
            .insert(chan, self.buf_programs.get(&chan).unwrap_or(&0) | bit);
        let program_no = self.buf_programs[&chan] - 1;
        self.settings
            .get_or_create_keyboard_mut(chan)
            .set_program_no(program_no);
        self.settings.queue_save();
        Some(Event::ProgramChange(chan, program_no))
    }

    fn octave_change(&mut self, chan: u8, key: u8) -> Option<Event> {
        let octave = match key {
            0 => 0,
            2 => 1,
            4 => 2,
            5 => 3,
            7 => 4,
            9 => 5,
            11 => 6,
            12 => 7,
            14 => 8,
            _ => return None,
        };
        self.settings
            .get_or_create_keyboard_mut(chan)
            .set_octave(octave);
        self.settings.queue_save();
        Some(Event::AllNotesOff(chan))
    }

    fn toggle_reverb(&mut self, chan: u8) -> Option<Event> {
        let keyboard = self.settings.get_or_create_keyboard_mut(chan);
        let new_reverb = !keyboard.reverb();
        keyboard.set_reverb(new_reverb);
        self.settings.queue_save();
        Some(if new_reverb {
            Event::ReverbOn(chan)
        } else {
            Event::ReverbOff(chan)
        })
    }

    fn toggle_chorus(&mut self, chan: u8) -> Option<Event> {
        let keyboard = self.settings.get_or_create_keyboard_mut(chan);
        let new_chorus = !keyboard.chorus();
        keyboard.set_chorus(new_chorus);
        self.settings.queue_save();
        Some(if new_chorus {
            Event::ChorusOn(chan)
        } else {
            Event::ChorusOff(chan)
        })
    }

    pub fn recv(&mut self) -> Result<Event, RecvError> {
        loop {
            let (idx, ev) = self.rx.recv()?;
            let chan = idx as u8;
            match ev {
                kmctrler::Event::Press(Input::Key(key)) => {
                    if self.buf_start >> chan & 0x01 != 0 && self.buf_select >> chan & 0x01 != 0 {
                        return Ok(Event::Tuning(key as i32 - 12));
                    }
                    if self.buf_start >> chan & 0x01 != 0 {
                        let Some(event) = self.program_change(chan, key) else {
                            continue;
                        };
                        return Ok(event);
                    }
                    if self.buf_select >> chan & 0x01 != 0 {
                        let Some(event) = self.octave_change(chan, key) else {
                            continue;
                        };
                        return Ok(event);
                    }
                    return Ok(Event::Noteon(
                        chan,
                        key as u16
                            + (self.settings.get_or_create_keyboard(chan).octave() as u16) * 12,
                    ));
                }
                kmctrler::Event::Release(Input::Key(key)) => {
                    self.buf_programs.remove(&chan);
                    return Ok(Event::Noteoff(
                        chan,
                        key as u16
                            + (self.settings.get_or_create_keyboard(chan).octave() as u16) * 12,
                    ));
                }
                kmctrler::Event::Press(Input::WheelUp) => {
                    if self.buf_start >> chan & 0x01 != 0 && self.buf_select >> chan & 0x01 != 0 {
                        let Some(event) = self.toggle_reverb(chan) else {
                            continue;
                        };
                        return Ok(event);
                    }
                    return Ok(Event::ModulationOn(chan));
                }
                kmctrler::Event::Release(Input::WheelUp) => return Ok(Event::ModulationOff(chan)),
                kmctrler::Event::Press(Input::WheelDown) => {
                    if self.buf_start >> chan & 0x01 != 0 && self.buf_select >> chan & 0x01 != 0 {
                        let Some(event) = self.toggle_chorus(chan) else {
                            continue;
                        };
                        return Ok(event);
                    }
                    return Ok(Event::HoldOn(chan));
                }
                kmctrler::Event::Release(Input::WheelDown) => return Ok(Event::HoldOff(chan)),
                kmctrler::Event::Press(Input::Select) => self.buf_select |= (0x01 << chan) as u32,
                kmctrler::Event::Release(Input::Select) => {
                    self.buf_select &= !((0x01 << chan) as u32)
                }
                kmctrler::Event::Press(Input::Start) => self.buf_start |= (0x01 << chan) as u32,
                kmctrler::Event::Release(Input::Start) => {
                    self.buf_start &= !((0x01 << chan) as u32)
                }
            };
        }
    }
}
