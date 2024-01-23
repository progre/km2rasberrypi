use std::{
    collections::HashMap,
    sync::mpsc::{self, RecvError},
    thread::sleep,
    time::Duration,
};

use crate::{
    kmctrler::{self, Input},
    settings::{KeyboardSettings, SynthesizerSettings},
};

use super::{
    v1::{toggle_chorus, toggle_reverb},
    Event,
};

fn noteon(chan: u8, virtual_key: u8, keyboard: &KeyboardSettings) -> Event {
    let vel = keyboard.velocity_per_program()[keyboard.program_no() as usize];
    Event::Noteon(chan, virtual_key, vel)
}

fn key_to_program_no(keys: &[bool; 24]) -> u8 {
    keys[5] as u8 * 0b01000000
        + keys[6] as u8 * 0b00100000
        + keys[7] as u8 * 0b00010000
        + keys[8] as u8 * 0b00001000
        + keys[9] as u8 * 0b00000100
        + keys[10] as u8 * 0b00000010
        + keys[11] as u8
        - 1
}

fn velocity_per_program(settings: &mut SynthesizerSettings, chan: u8, key: u8) -> bool {
    let keyboard = settings.get_or_create_keyboard_mut(chan);
    let program_no = keyboard.program_no();
    let velocity_per_program = keyboard.velocity_per_program_mut();
    let vel = match key {
        12 => 127 - 9 * 6,
        14 => 127 - 9 * 5,
        16 => 127 - 9 * 4,
        17 => 127 - 9 * 3,
        19 => 127 - 9 * 2,
        21 => 127 - 9,
        23 => 127,
        _ => return false,
    };
    println!("program_no: {}, vel: {}", program_no, vel);
    velocity_per_program[program_no as usize] = vel;
    settings.queue_save();
    true
}

pub fn octave_shift_down(settings: &mut SynthesizerSettings, chan: u8) {
    let keyboard = settings.get_or_create_keyboard_mut(chan);
    if keyboard.octave() == 0 {
        return;
    }
    keyboard.set_octave(keyboard.octave() - 1);
    settings.queue_save();
}

pub fn octave_shift_up(settings: &mut SynthesizerSettings, chan: u8) {
    let keyboard = settings.get_or_create_keyboard_mut(chan);
    if keyboard.octave() >= 9 {
        return;
    }
    keyboard.set_octave(keyboard.octave() + 1);
    settings.queue_save();
}

fn program_change(settings: &mut SynthesizerSettings, chan: u8, program_no: u8) {
    settings
        .get_or_create_keyboard_mut(chan)
        .set_program_no(program_no);
    settings.queue_save();
}

pub fn percussion(event_queue: &mut Vec<Event>, no: i32) -> Event {
    if no == 1 {
        event_queue.push(Event::Noteoff(9, 42));
        event_queue.push(Event::Noteon(9, 42, 127));
        event_queue.push(Event::Noteoff(9, 42));
        Event::Noteon(9, 42, 127)
    } else {
        event_queue.push(Event::Noteoff(9, 36));
        event_queue.push(Event::Noteon(9, 36, 127));
        event_queue.push(Event::Noteoff(9, 36));
        Event::Noteon(9, 36, 127)
    }
}

fn add_on_sfx(event_queue: &mut Vec<Event>, chan: u8, keyboard: &KeyboardSettings) {
    event_queue.push(Event::Noteoff(chan, 79));
    event_queue.push(noteon(chan, 79, keyboard));
    event_queue.push(Event::Noteoff(chan, 76));
    event_queue.push(noteon(chan, 76, keyboard));
    event_queue.push(Event::Noteoff(chan, 72));
    event_queue.push(noteon(chan, 72, keyboard));
}

fn add_off_sfx(event_queue: &mut Vec<Event>, chan: u8, keyboard: &KeyboardSettings) {
    event_queue.push(Event::Noteoff(chan, 72));
    event_queue.push(noteon(chan, 72, keyboard));
    event_queue.push(Event::Noteoff(chan, 76));
    event_queue.push(noteon(chan, 76, keyboard));
    event_queue.push(Event::Noteoff(chan, 79));
    event_queue.push(noteon(chan, 79, keyboard));
}

fn normal_mode_action(
    settings: &mut SynthesizerSettings,
    chan: u8,
    ev: &kmctrler::Event,
) -> Result<Event, bool> {
    match ev {
        kmctrler::Event::Press(Input::WheelUp) => Ok(Event::ModulationOn(chan)),
        kmctrler::Event::Release(Input::WheelUp) => Ok(Event::ModulationOff(chan)),
        kmctrler::Event::Press(Input::WheelDown) => Ok(Event::HoldOn(chan)),
        kmctrler::Event::Release(Input::WheelDown) => Ok(Event::HoldOff(chan)),
        kmctrler::Event::Release(Input::Select) => {
            octave_shift_down(settings, chan);
            Err(true)
        }
        kmctrler::Event::Release(Input::Start) => {
            octave_shift_up(settings, chan);
            Err(true)
        }
        _ => Err(false),
    }
}

pub fn config_mode_action(
    settings: &mut SynthesizerSettings,
    event_queue: &mut Vec<Event>,
    state: &kmctrler::State,
    chan: u8,
    ev: &kmctrler::Event,
) -> Result<Event, bool> {
    // C#3
    if state.keys()[1] {
        match ev {
            kmctrler::Event::Press(Input::Key(1)) => {
                let keyboard = settings.get_or_create_keyboard(chan);
                let program_no = keyboard.program_no();
                // 2進数の各要素に分解
                let mut notes: Vec<_> = (0..7)
                    .filter(|i| ((program_no + 1) >> i) & 1 == 1)
                    .map(|i| 11 - i)
                    .flat_map(|key| {
                        let octave = keyboard.octave();
                        let virtual_key = key + octave * 12;
                        [
                            Event::Noteoff(chan, virtual_key),
                            noteon(chan, virtual_key, keyboard),
                        ]
                    })
                    .collect();
                let first = notes.pop().unwrap();
                event_queue.append(&mut notes);
                return Ok(first);
            }
            kmctrler::Event::Press(Input::Key(key)) => {
                if (5..=11).contains(key) {
                    let program_no = key_to_program_no(state.keys());
                    program_change(settings, chan, program_no);
                    let keyboard = settings.get_or_create_keyboard(chan);
                    let octave = keyboard.octave();
                    let virtual_key = key + octave * 12;
                    event_queue.push(Event::Noteoff(chan, virtual_key));
                    event_queue.push(noteon(chan, virtual_key, keyboard));
                    return Ok(Event::ProgramChange(chan, program_no));
                }
                if !velocity_per_program(settings, chan, *key) {
                    return Err(true);
                }
            }
            kmctrler::Event::Release(Input::Key(_)) => {}
            kmctrler::Event::Press(Input::WheelDown) => {
                let current_program_no = settings.get_or_create_keyboard(chan).program_no();
                let new_program_no = current_program_no.checked_sub(1).unwrap_or(127);
                program_change(settings, chan, new_program_no);
                event_queue.push(Event::Noteoff(chan, 69));
                event_queue.push(noteon(chan, 69, settings.get_or_create_keyboard(chan)));
                return Ok(Event::ProgramChange(chan, new_program_no));
            }
            kmctrler::Event::Press(Input::WheelUp) => {
                let current_program_no = settings.get_or_create_keyboard(chan).program_no();
                let new_program_no = (current_program_no as i8).checked_add(1).unwrap_or(0) as u8;
                program_change(settings, chan, new_program_no);
                event_queue.push(Event::Noteoff(chan, 69));
                event_queue.push(noteon(chan, 69, settings.get_or_create_keyboard(chan)));
                return Ok(Event::ProgramChange(chan, new_program_no));
            }
            _ => return Err(true),
        }
    }
    if state.start() {
        if let kmctrler::Event::Press(Input::Key(key)) = ev {
            return Ok(Event::Tuning(*key as i32 - 12));
        }
        return Err(true);
    }
    match ev {
        kmctrler::Event::Press(Input::Key(13)) => {
            let keyboard = settings.get_or_create_keyboard(chan);
            if keyboard.reverb() {
                add_off_sfx(event_queue, chan, keyboard);
            } else {
                add_on_sfx(event_queue, chan, keyboard);
            }
            return Ok(toggle_reverb(settings, chan));
        }
        kmctrler::Event::Press(Input::Key(15)) => {
            let keyboard = settings.get_or_create_keyboard(chan);
            if keyboard.chorus() {
                add_off_sfx(event_queue, chan, keyboard);
            } else {
                add_on_sfx(event_queue, chan, keyboard);
            }
            return Ok(toggle_chorus(settings, chan));
        }
        _ => {}
    }
    Err(false)
}

pub fn common_action(
    settings: &mut SynthesizerSettings,
    keydown_octave_table: &mut HashMap<u8, [u8; 24]>,
    chan: u8,
    ev: &kmctrler::Event,
) -> Option<Event> {
    match ev {
        kmctrler::Event::Press(Input::Key(key)) => {
            let keyboard = settings.get_or_create_keyboard(chan);
            let octave = keyboard.octave();
            keydown_octave_table.entry(chan).or_default()[*key as usize] = octave;
            let virtual_key = key + octave * 12;
            Some(noteon(chan, virtual_key, keyboard))
        }
        kmctrler::Event::Release(Input::Key(key)) => {
            let octave = keydown_octave_table.entry(chan).or_default()[*key as usize];
            let virtual_key = key + octave * 12;
            Some(Event::Noteoff(chan, virtual_key))
        }
        _ => None,
    }
}

/// モード切替 .... Select + Start
/// 演奏モード
///   オクターブシフト .... Select / Start
///   モジュレーション(ビブラート) .... WheelUp
///   ホールド .... WheelDown
/// 調整モード
///   チューニング .... Start + Key
///   プログラムチェンジ .... C#3 + Key / WheelUp / WheelDown
///   プログラムの音量の変更 .... C#3 + Key
///   リバーブ(toggle) .... C#4
///   コーラス(toggle) .... D#4
pub struct SynthCtrler {
    rx: mpsc::Receiver<(usize, kmctrler::Event)>,
    settings: SynthesizerSettings,
    mode_config: bool,
    kmctrler_states: HashMap<u8, kmctrler::State>,
    keydown_octave_table: HashMap<u8, [u8; 24]>,
    event_queue: Vec<Event>,
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
            mode_config: false,
            kmctrler_states: HashMap::new(),
            keydown_octave_table: HashMap::new(),
            event_queue: Vec::new(),
        }
    }

    #[allow(unused)]
    pub fn recv(&mut self) -> Result<Event, RecvError> {
        if let Some(event) = self.event_queue.pop() {
            if let Event::Noteoff(_, _) = event {
                sleep(Duration::from_millis(100));
            }
            return Ok(event);
        }
        loop {
            let (idx, ev) = self.rx.recv()?;
            let chan = idx as u8;
            let state = self.kmctrler_states.entry(chan).or_default();
            state.update(&ev);

            if state.select() && state.start() {
                self.mode_config = !self.mode_config;
                state.reset_select_start();
                return Ok(percussion(
                    &mut self.event_queue,
                    if self.mode_config { 1 } else { 0 },
                ));
            }
            if self.mode_config {
                match config_mode_action(
                    &mut self.settings,
                    &mut self.event_queue,
                    state,
                    chan,
                    &ev,
                ) {
                    Ok(event) => return Ok(event),
                    Err(true) => continue,
                    Err(false) => {}
                }
            } else {
                match normal_mode_action(&mut self.settings, chan, &ev) {
                    Ok(event) => return Ok(event),
                    Err(true) => continue,
                    Err(false) => {}
                }
            }
            if let Some(event) = common_action(
                &mut self.settings,
                &mut self.keydown_octave_table,
                chan,
                &ev,
            ) {
                return Ok(event);
            }
        }
    }
}
