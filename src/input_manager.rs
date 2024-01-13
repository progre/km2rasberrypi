use std::{
    collections::HashMap,
    sync::{mpsc, Arc, RwLock},
    thread::{sleep, spawn},
    time::Duration,
};

use evdev::{Device, InputEventKind};

use crate::kmctrler::{Event, Input};

fn is_km_ctrler(dev: &Device) -> bool {
    dev.name() == Some("KONAMI USB Multipurpose Controller")
        && dev.supported_keys().unwrap().iter().count() == 34
}

pub fn start_inputs() -> mpsc::Receiver<(usize, Event)> {
    let devices = Arc::new(RwLock::new(HashMap::new()));

    let (tx, rx) = mpsc::channel();
    {
        let devices = devices.clone();
        spawn(move || loop {
            evdev::enumerate()
                .filter(|(_, dev)| {
                    is_km_ctrler(dev)
                        && !devices
                            .read()
                            .unwrap()
                            .contains_key(dev.physical_path().unwrap())
                })
                .for_each(|(_, mut dev)| {
                    let physical_path = dev.physical_path().unwrap().to_owned();
                    println!("{:?}", physical_path);
                    devices.write().unwrap().insert(physical_path.clone(), ());
                    let devices = devices.clone();
                    let tx = tx.clone();
                    spawn(move || loop {
                        let Ok(events) = dev.fetch_events() else {
                            devices.write().unwrap().remove(&physical_path);
                            return;
                        };
                        let idx = {
                            let dev = devices.read().unwrap();
                            let mut keys: Vec<_> = dev.keys().collect();
                            keys.sort();
                            keys.binary_search(&&physical_path).unwrap()
                        };
                        events
                            .filter_map(|ev| {
                                let InputEventKind::Key(key) = ev.kind() else {
                                    return None;
                                };
                                Input::from_raw(key.0).map(|input| {
                                    if ev.value() == 0 {
                                        Event::Release(input)
                                    } else {
                                        Event::Press(input)
                                    }
                                })
                            })
                            .for_each(|ev| {
                                tx.send((idx, ev)).unwrap();
                            });
                    });
                });
            sleep(Duration::from_secs(3));
        });
    }
    rx
}
