use crate::bindings::{
    _fluid_audio_driver_t, _fluid_hashtable_t, _fluid_synth_t, delete_fluid_audio_driver,
    delete_fluid_settings, delete_fluid_synth, fluid_settings_setint, fluid_settings_setstr,
    fluid_synth_activate_tuning, fluid_synth_all_notes_off, fluid_synth_cc, fluid_synth_noteoff,
    fluid_synth_noteon, fluid_synth_program_change, fluid_synth_sfload, fluid_synth_tune_notes,
    new_fluid_audio_driver, new_fluid_settings, new_fluid_synth, FLUID_OK,
};

pub struct FluidSynth {
    settings: *mut _fluid_hashtable_t,
    synth: *mut _fluid_synth_t,
    driver: *mut _fluid_audio_driver_t,
}

impl FluidSynth {
    pub fn new() -> Self {
        unsafe {
            let settings = new_fluid_settings();

            fluid_settings_setstr(
                settings,
                "audio.driver\0".as_ptr() as *const _,
                "alsa\0".as_ptr() as *const _,
            );
            fluid_settings_setstr(
                settings,
                "audio.sdl2.device\0".as_ptr() as *const _,
                "default\0".as_ptr() as *const _,
            );
            fluid_settings_setint(settings, "audio.periods\0".as_ptr() as *const _, 4);
            fluid_settings_setint(settings, "audio.period-size\0".as_ptr() as *const _, 444);

            let synth = new_fluid_synth(settings);
            let driver = new_fluid_audio_driver(settings, synth);
            fluid_synth_sfload(
                synth,
                "/usr/share/sounds/sf2/FluidR3_GM.sf2\0".as_ptr() as *const _,
                1,
            );
            Self {
                settings,
                synth,
                driver,
            }
        }
    }

    pub fn noteon(&self, chan: u8, key: u8, vel: u8) -> bool {
        debug_assert!((1..=127).contains(&vel));
        (unsafe { fluid_synth_noteon(self.synth, chan as i32, key as i32, vel as i32) }) as u32
            == FLUID_OK
    }

    pub fn noteoff(&self, chan: u8, key: u8) -> bool {
        (unsafe { fluid_synth_noteoff(self.synth, chan as i32, key as i32) }) as u32 == FLUID_OK
    }

    pub fn all_notes_off(&self, chan: u8) -> bool {
        (unsafe { fluid_synth_all_notes_off(self.synth, chan as i32) }) as u32 == FLUID_OK
    }

    pub fn tuning(&self, tuning: i32) -> bool {
        let keys: Vec<i32> = (0..=127).collect();
        let pitch: Vec<f64> = keys
            .iter()
            .map(|&i| (i as f64 + tuning as f64 / 12.0) * 100.0)
            .collect();
        (0..16).all(|chan| {
            (unsafe {
                fluid_synth_tune_notes(self.synth, 0, 0, 128, keys.as_ptr(), pitch.as_ptr(), 1)
            }) as u32
                == FLUID_OK
                && (unsafe { fluid_synth_activate_tuning(self.synth, chan, 0, 0, 1) }) as u32
                    == FLUID_OK
        })
    }

    pub fn hold(&self, chan: u8, value: bool) -> bool {
        (unsafe { fluid_synth_cc(self.synth, chan as i32, 64, if value { 127 } else { 0 }) }) as u32
            == FLUID_OK
    }

    pub fn modulation(&self, chan: u8, value: bool) -> bool {
        (unsafe { fluid_synth_cc(self.synth, chan as i32, 1, if value { 127 } else { 0 }) }) as u32
            == FLUID_OK
    }

    pub fn reverb(&self, chan: u8, value: bool) -> bool {
        (unsafe { fluid_synth_cc(self.synth, chan as i32, 91, if value { 127 } else { 0 }) }) as u32
            == FLUID_OK
    }

    pub fn chorus(&self, chan: u8, value: bool) -> bool {
        (unsafe { fluid_synth_cc(self.synth, chan as i32, 93, if value { 127 } else { 0 }) }) as u32
            == FLUID_OK
    }

    pub fn program_change(&self, chan: u8, program: u8) -> bool {
        (unsafe { fluid_synth_program_change(self.synth, chan as i32, program as i32) }) as u32
            == FLUID_OK
    }
}

impl Drop for FluidSynth {
    fn drop(&mut self) {
        unsafe {
            delete_fluid_audio_driver(self.driver);
            delete_fluid_synth(self.synth);
            delete_fluid_settings(self.settings);
        }
    }
}
