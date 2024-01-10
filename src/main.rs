mod bindings;
mod fluid_synth;
mod input_manager;
mod kmctrler;
mod synthesizer;

use fluid_synth::FluidSynth;
use input_manager::start_inputs;
use synthesizer::{Event, SynthCtrler};

fn process_event(synth: &FluidSynth, ev: Event) {
    match ev {
        Event::Noteon(chan, key) => synth.noteon(chan, key),
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

fn main() {
    let rx = start_inputs();

    let synth = FluidSynth::new();
    let mut synth_ctrler = SynthCtrler::new(rx);
    synth_ctrler
        .init()
        .into_iter()
        .for_each(|ev| process_event(&synth, ev));
    loop {
        let ev = synth_ctrler.recv().unwrap();
        process_event(&synth, ev);
    }
}
