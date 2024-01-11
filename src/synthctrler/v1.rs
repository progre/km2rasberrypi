use std::{
    collections::HashMap,
    sync::mpsc::{self, RecvError},
};

use crate::{
    kmctrler::{self, Input},
    settings::SynthesizerSettings,
};

use super::Event;

pub fn toggle_reverb(settings: &mut SynthesizerSettings, chan: u8) -> Event {
    let keyboard = settings.get_or_create_keyboard_mut(chan);
    let new_reverb = !keyboard.reverb();
    keyboard.set_reverb(new_reverb);
    settings.queue_save();
    if new_reverb {
        Event::ReverbOn(chan)
    } else {
        Event::ReverbOff(chan)
    }
}

pub fn toggle_chorus(settings: &mut SynthesizerSettings, chan: u8) -> Event {
    let keyboard = settings.get_or_create_keyboard_mut(chan);
    let new_chorus = !keyboard.chorus();
    keyboard.set_chorus(new_chorus);
    settings.queue_save();
    if new_chorus {
        Event::ChorusOn(chan)
    } else {
        Event::ChorusOff(chan)
    }
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
    #[allow(unused)]
    pub fn new(
        settings: SynthesizerSettings,
        rx: mpsc::Receiver<(usize, kmctrler::Event)>,
    ) -> Self {
        Self {
            rx,
            settings,
            buf_programs: HashMap::new(),
            buf_select: 0,
            buf_start: 0,
        }
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

    #[allow(unused)]
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
                        key + self.settings.get_or_create_keyboard(chan).octave() * 12,
                        127,
                    ));
                }
                kmctrler::Event::Release(Input::Key(key)) => {
                    self.buf_programs.remove(&chan);
                    return Ok(Event::Noteoff(
                        chan,
                        key + self.settings.get_or_create_keyboard(chan).octave() * 12,
                    ));
                }
                kmctrler::Event::Press(Input::WheelUp) => {
                    if self.buf_start >> chan & 0x01 != 0 && self.buf_select >> chan & 0x01 != 0 {
                        return Ok(toggle_reverb(&mut self.settings, chan));
                    }
                    return Ok(Event::ModulationOn(chan));
                }
                kmctrler::Event::Release(Input::WheelUp) => return Ok(Event::ModulationOff(chan)),
                kmctrler::Event::Press(Input::WheelDown) => {
                    if self.buf_start >> chan & 0x01 != 0 && self.buf_select >> chan & 0x01 != 0 {
                        return Ok(toggle_chorus(&mut self.settings, chan));
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
