use std::{
    collections::HashMap,
    fs::{self, read_to_string},
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc::{self, RecvError},
        Arc,
    },
    thread::{sleep, spawn},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use getset::{CopyGetters, Getters, MutGetters, Setters};
use toml_edit::{ArrayOfTables, Document, Item, Table, Value};

use crate::kmctrler::{self, Input};

const PATH: &str = "/boot/km2rasberrypi.toml";

fn read() -> Document {
    read_to_string(PATH)
        .unwrap_or_default()
        .parse()
        .unwrap_or_default()
}

fn integer(table: &Table, key: &str) -> Option<i64> {
    table.get(key).and_then(|x| x.as_integer())
}

fn bool(table: &Table, key: &str) -> Option<bool> {
    table.get(key).and_then(|x| x.as_bool())
}

fn put(doc: &mut Table, key: &str, value: impl Into<Value>) {
    let value = value.into();
    if let Some(item) = doc.get_mut(key) {
        *item = Item::Value(value);
    } else {
        let _ = doc.insert(key, Item::Value(value));
    }
}

#[derive(Clone, CopyGetters, Setters)]
pub struct KeyboardSettings {
    #[getset(get_copy = "pub", set = "pub")]
    octave: u8,
    #[getset(get_copy = "pub", set = "pub")]
    program_no: u8,
    #[getset(get_copy = "pub", set = "pub")]
    reverb: bool,
    #[getset(get_copy = "pub", set = "pub")]
    chorus: bool,
}

impl Default for KeyboardSettings {
    fn default() -> Self {
        Self {
            octave: 5,
            program_no: 0,
            reverb: false,
            chorus: false,
        }
    }
}

#[derive(Clone, CopyGetters, Getters, MutGetters, Setters)]
pub struct SynthesizerSettings {
    #[get = "pub"]
    keyboards: Vec<KeyboardSettings>,
    last_modify_timestamp: Arc<AtomicU64>,
}

impl SynthesizerSettings {
    pub fn get_or_create_keyboard(&mut self, idx: u8) -> &KeyboardSettings {
        let idx = idx as usize;
        if idx >= self.keyboards.len() {
            self.keyboards
                .resize_with(idx + 1, KeyboardSettings::default);
        }
        &self.keyboards[idx]
    }

    pub fn get_or_create_keyboard_mut(&mut self, idx: u8) -> &mut KeyboardSettings {
        let idx = idx as usize;
        if idx >= self.keyboards.len() {
            self.keyboards
                .resize_with(idx + 1, KeyboardSettings::default);
        }
        &mut self.keyboards[idx]
    }

    pub fn load() -> Self {
        let doc = read();
        Self {
            keyboards: doc
                .get("keyboards")
                .and_then(|x| x.as_array_of_tables())
                .iter()
                .flat_map(|x| x.iter())
                .map(|item| KeyboardSettings {
                    octave: integer(item, "octave").unwrap_or(5) as u8,
                    program_no: integer(item, "program_no").unwrap_or(0) as u8,
                    reverb: bool(item, "reverb").unwrap_or(false),
                    chorus: bool(item, "chorus").unwrap_or(false),
                })
                .collect(),
            last_modify_timestamp: Arc::default(),
        }
    }

    fn save(&self) {
        let mut doc = read();
        let keyboards = doc
            .as_table_mut()
            .entry("keyboards")
            .or_insert_with(|| Item::ArrayOfTables(ArrayOfTables::new()))
            .as_array_of_tables_mut()
            .unwrap();
        (0..(self.keyboards.len() - keyboards.len())).for_each(|_| {
            keyboards.push(Table::new());
        });
        for (idx, keyboard) in self.keyboards.iter().enumerate() {
            let table = keyboards.get_mut(idx).unwrap();
            put(table, "octave", keyboard.octave as i64);
            put(table, "program_no", keyboard.program_no as i64);
            put(table, "reverb", keyboard.reverb);
            put(table, "chorus", keyboard.chorus);
            table.sort_values();
        }
        doc.sort_values();
        if let Err(err) = fs::write(PATH, doc.to_string()) {
            eprintln!("{}", err);
        }
    }

    pub fn queue_save(&mut self) {
        let last_modify_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        self.last_modify_timestamp
            .store(last_modify_timestamp, Ordering::Relaxed);
        let store = self.clone();
        spawn(move || {
            sleep(Duration::from_secs(1));
            if store.last_modify_timestamp.load(Ordering::Relaxed) == last_modify_timestamp {
                store.save();
                println!("saved({last_modify_timestamp})");
            }
        });
    }
}

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
