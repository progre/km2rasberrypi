use std::{
    fs::{self, read_to_string},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    thread::{sleep, spawn},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use getset::{CopyGetters, Getters, MutGetters, Setters};
use toml_edit::{ArrayOfTables, Document, Item, Table, Value};

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

#[derive(Clone, CopyGetters, Getters, MutGetters, Setters)]
pub struct KeyboardSettings {
    #[getset(get_copy = "pub", set = "pub")]
    octave: u8,
    #[getset(get_copy = "pub", set = "pub")]
    program_no: u8,
    #[getset(get = "pub", get_mut = "pub")]
    velocity_per_program: [u8; 128],
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
            velocity_per_program: [100; 128],
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
                    velocity_per_program: [100; 128],
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
