mod bindings;
mod fluid_synth;
mod input_manager;
mod kmctrler;
mod settings;
mod synthctrler;

use fluid_synth::FluidSynth;
use input_manager::start_inputs;
use settings::SynthesizerSettings;
use synthctrler::{v2::SynthCtrler, Event};

fn process_event(synth: &FluidSynth, ev: Event) {
    match ev {
        Event::Noteon(chan, key, vel) => synth.noteon(chan, key, vel),
        Event::Noteoff(chan, key) => synth.noteoff(chan, key),
        Event::AllNotesOff(chan) => synth.all_notes_off(chan),
        Event::ProgramChange(chan, program) => synth.program_change(chan, program),
        Event::Tuning(tuning) => synth.tuning(tuning),
        Event::HoldOn(chan) => synth.hold(chan, true),
        Event::HoldOff(chan) => synth.hold(chan, false),
        Event::ModulationOn(chan) => synth.modulation(chan, true),
        Event::ModulationOff(chan) => synth.modulation(chan, false),
        Event::ReverbOn(chan) => synth.reverb(chan, true),
        Event::ReverbOff(chan) => synth.reverb(chan, false),
        Event::ChorusOn(chan) => synth.chorus(chan, true),
        Event::ChorusOff(chan) => synth.chorus(chan, false),
    };
}

fn init(settings: &mut SynthesizerSettings) -> Vec<Event> {
    settings
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

fn main() {
    let rx = start_inputs();

    let mut settings = SynthesizerSettings::load();
    let events = init(&mut settings);
    let synth = FluidSynth::new();
    let mut synth_ctrler = SynthCtrler::new(settings, rx);
    events.into_iter().for_each(|ev| process_event(&synth, ev));
    loop {
        let ev = synth_ctrler.recv().unwrap();
        process_event(&synth, ev);
    }
}
