use std::{
    collections::HashMap,
    sync::mpsc::{self, RecvError},
    thread::sleep,
    time::Duration,
};

use crate::{
    kmctrler::{self, Input},
    settings::SynthesizerSettings,
};

use super::{
    v2::{common_action, config_mode_action, octave_shift_down, octave_shift_up, percussion},
    Event,
};

fn octave_shift_down_without_save(settings: &mut SynthesizerSettings, chan: u8) {
    let keyboard = settings.get_or_create_keyboard_mut(chan);
    if keyboard.octave() == 0 {
        return;
    }
    keyboard.set_octave(keyboard.octave() - 1);
}

fn octave_shift_up_without_save(settings: &mut SynthesizerSettings, chan: u8) {
    let keyboard = settings.get_or_create_keyboard_mut(chan);
    if keyboard.octave() >= 9 {
        return;
    }
    keyboard.set_octave(keyboard.octave() + 1);
}

fn normal_mode_action(
    settings: &mut SynthesizerSettings,
    state: &kmctrler::State,
    chan: u8,
    ev: &kmctrler::Event,
) -> Result<Event, bool> {
    match (state.start(), ev) {
        (_, kmctrler::Event::Press(Input::WheelUp)) => {
            octave_shift_down(settings, chan);
            Err(true)
        }
        (_, kmctrler::Event::Press(Input::WheelDown)) => {
            octave_shift_up(settings, chan);
            Err(true)
        }
        (false, kmctrler::Event::Release(Input::WheelUp)) => {
            octave_shift_up_without_save(settings, chan);
            Err(true)
        }
        (false, kmctrler::Event::Release(Input::WheelDown)) => {
            octave_shift_down_without_save(settings, chan);
            Err(true)
        }
        (_, kmctrler::Event::Press(Input::Select)) => Ok(Event::ModulationOn(chan)),
        (_, kmctrler::Event::Release(Input::Select)) => Ok(Event::ModulationOff(chan)),
        _ => Err(false),
    }
}

/// モード切替 .... Select + Start
/// 演奏モード
///   オクターブシフト .... Start + WheelUp / WheelDown
///   一時的なオクターブシフト .... WheelUp / WheelDown
///   モジュレーション(ビブラート) .... Select
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
                match normal_mode_action(&mut self.settings, state, chan, &ev) {
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
