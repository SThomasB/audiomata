
use alsa::{Direction, ValueOr};
use alsa::pcm::{PCM,HwParams, Format, Access, State};
use std::f32::consts::PI;
use std::sync::mpsc;


use midir;
#[macro_use]
extern crate lazy_static;
// Open default playback device



fn midiname(message: &[u8]) -> &str {
    let code = message[0];
    let note = message[1];
    let velocity = message[2];

    let note = MIDI_NAMES[note as usize];
    note
}

lazy_static! {
static ref MIDI_NOTES: Vec<f64> = (0..129).map(|x| 440.0 * (2f64).powf(((x as f64)-69.0)/12.0)).collect();
}


const MIDI_NAMES: [&str; 129] = [
    "-0", "-1", "-2", "-3", "-4", "-5", "-6", "-7", "-8", "-9", "-10", "-11", "-12", "-13", "-14", "-15", "-16", "-17", "-18", "-19", "-20",
    "A0", "A#0/Bb0", "B0", "C1", "C#1/Db1", "D1", "D#1/Eb1", "E1", "F1", "F#1/Gb1", "G1", "G#1/Ab1",
    "A1", "A#1/Bb1", "B1", "C2", "C#2/Db2", "D2", "D#2/Eb2", "E2", "F2", "F#2/Gb2", "G2", "G#2/Ab2",
    "A2", "A#2/Bb2", "B2", "C3", "C#3/Db3", "D3", "D#3/Eb3", "E3", "F3", "F#3/Gb3", "G3", "G#3/Ab3",
    "A3", "A#3/Bb3", "B3", "C4", "C#4/Db4", "D4", "D#4/Eb4", "E4", "F4", "F#4/Gb4", "G4", "G#4/Ab4",
    "A4", "A#4/Bb4", "B4", "C5", "C#5/Db5", "D5", "D#5/Eb5", "E5", "F5", "F#5/Gb5", "G5", "G#5/Ab5",
    "A5", "A#5/Bb5", "B5", "C6", "C#6/Db6", "D6", "D#6/Eb6", "E6", "F6", "F#6/Gb6", "G6", "G#6/Ab6",
    "A6", "A#6/Bb6", "B6", "C7", "C#7/Db7", "D7", "D#7/Eb7", "E7", "F7", "F#7/Gb7", "G7", "G#7/Ab7",
    "A7", "A#7/Bb7", "B7", "C8", "C#8/Db8", "D8", "D#8/Eb8", "E8", "F8", "F#8/Gb8", "G8", "G#8/Ab8",
    "A8", "A#8/Bb8", "B8", "C9", "C#9/Db9", "D9", "D#9/Eb9", "E9", "F9", "F#9/Gb9", "G9", "G#9/Ab9"];

fn main() {

  let pcm = PCM::new("default", Direction::Playback, false).unwrap();
  let hwp = HwParams::any(&pcm).unwrap();
  hwp.set_rate(44100, ValueOr::Nearest).unwrap();
  hwp.set_format(Format::s16()).unwrap();
  hwp.set_access(Access::RWInterleaved).unwrap();
  pcm.hw_params(&hwp).unwrap();

  let io = pcm.io_i16().unwrap();
  let midi_input = midir::MidiInput::new("alsa").unwrap();
  let (tx, rx) = mpsc::channel();


  for port in midi_input.ports(){
      match midi_input.port_name(&port) {
        Ok(s) => println!("{}", s),
        _ => panic!(),
      }
  }
  let port = &midi_input.ports()[1];

  let _conn_in = midi_input.connect(&port, "M32", move |x, y, _| -> () {
        tx.send(y.to_owned()).unwrap();
    } ,());




  let hwp = pcm.hw_params_current().unwrap();
  let swp = pcm.sw_params_current().unwrap();
  swp.set_start_threshold(hwp.get_buffer_size().unwrap()).unwrap();
  pcm.sw_params(&swp).unwrap();

// Make a sine wave
  let fs = 44100.0;
  const f: f32 = 880.0/44100.0;
  const bufsize: usize = (1.0/f) as usize;
  let max= 8192.0;
  let amp = 0.5;
  static buf = [0i16; 256];



// play it back for 2 seconds.
//
  for _ in 0..2*44100/bufsize {
      assert_eq!(io.writei(&buf[..]).unwrap(), bufsize)
  }


// In case the buffer was larger than 2 seconds, start the stream manually.
  if pcm.state() != State::Running { pcm.start().unwrap() };
  pcm.drain().unwrap();


  for received in &rx {
      let val = MIDI_NOTES[received[1] as usize];
      //let bufsize: usize = (1.0/f) as usize;
      //let mut buf = [0i16; bufsize];
      println!("rec: {:?}", val);

  }


// making sure we don't start the stream too early


}
