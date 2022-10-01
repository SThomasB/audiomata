

use alsa::{Direction, ValueOr};
use alsa::pcm::{PCM,HwParams, Format, Access, State};
use std::f32::consts::PI;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{Sender, Receiver, channel};
use std::thread::sleep;
use std::time::Duration;
use vektoria::{VektorType};
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
static ref MIDI_NOTES: Vec<f32> = (0..129).map(|x| 440.0 * (2f32).powf(((x as f32)-69.0)/12.0)).collect();
}


lazy_static! {
static ref WAVE_TABLE: Vec<Vec<f32>> = (0..129).map(
        |x| (0..5380).map(|n| (MIDI_NOTES[x as usize] * 2_f32 * PI * (n as f32) / SAMPLERATE).sin()).collect()
    ).collect();
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



const BUFFER_SIZE: u16 = 264;
const SAMPLERATE: f32 = 44100.0;
const FRAME_SIZE: usize = 264;
const NUMBER_OF_NOTES: usize = 129;
fn sinegen(note: &u8, velocity: &u8, transmitter: Sender<Vec<f32>>) {
    //let nf = MIDI_NOTES[note.to_owned() as usize] / SAMPLERATE;

    loop {

        transmitter.send(
            WAVE_TABLE[note.to_owned() as usize][0..256].to_owned()
        ).unwrap();

        std::thread::sleep(std::time::Duration::from_millis(4));
    }
}
struct FramePackage {
    frame: [i16; FRAME_SIZE],
    timing: u64
}
type Buffer = [i16; BUFFER_SIZE as usize];
fn run() -> Result<(),String> {

    // SETUP PCM
    let pcm = PCM::new("default", Direction::Playback, false).unwrap();
    let hwp = HwParams::any(&pcm).unwrap();
    hwp.set_rate(SAMPLERATE as u32, ValueOr::Nearest);
    hwp.set_format(Format::s16()).unwrap();
    hwp.set_access(Access::RWInterleaved).unwrap();
    pcm.hw_params(&hwp).unwrap();
    let io = pcm.io_i16().unwrap();
    let hwp = pcm.hw_params_current().unwrap();
    let swp = pcm.sw_params_current().unwrap();
    swp.set_start_threshold(hwp.get_buffer_size().unwrap()).unwrap();
    pcm.sw_params(&swp).unwrap();


    let (tx, rx) = channel();
    let midi_input = midir::MidiInput::new("midi-input").unwrap();
    let port = &midi_input.ports()[1];
    let (package_sender, package_receiver) : (Sender<FramePackage>, Receiver<FramePackage>) = channel();
    let (buffer_sender, buffer_receiver): (Sender<Buffer>, Receiver<Buffer>) = channel();
    let pressed_lut = Arc::new(Mutex::new([false; NUMBER_OF_NOTES as usize]));
    let pressed = Arc::clone(&pressed_lut);
    let buffer_global = Arc::new(Mutex::new([0_i16; BUFFER_SIZE as usize]));

    std::thread::spawn(move || {
            let mut status: u8;
            let mut param1: u8;
            let mut param2: u8;
            let mut timing: u64;
            for received in rx {
                (status, param1, param2, timing) = received;
                if status == 144_u8 {
                     let mut pressed = pressed.lock().unwrap();
                     if param2 != 0_u8 {
                        pressed[param1 as usize] = true;
                        drop(pressed);
                        let transmitter = package_sender.clone();
                        let pressed = Arc::clone(&pressed_lut);
                        std::thread::spawn( move || {
                            let nf = MIDI_NOTES[param1 as usize]/SAMPLERATE;
                            let n = (1.0/nf).ceil() as usize;
                            let mut nms = 0;
                            let mut i = 0;
                            let mut frame : [i16; FRAME_SIZE as usize] = [0; FRAME_SIZE as usize];
                            loop {
                                let view_pressed = pressed.lock().unwrap();
                                if view_pressed[param1 as usize]==false{break;}
                                drop(view_pressed);
                                //buffer = WAVE_TABLE[param1 as usize][i..(i+256)].to_owned().iter().map(
                                //    |x| (x*8192.0).to_owned() as i16
                                //).collect::<Vec<i16>>().try_into().unwrap();
                                for (n, a) in frame.iter_mut().enumerate() {
                                    *a = (((n+i) as f32 * 2.0 * PI * nf).sin() * 8192.0) as i16;
                                }
                                transmitter.send(
                                    FramePackage{frame, timing}
                                ).unwrap();
                                std::thread::sleep(std::time::Duration::from_micros((265.0/44100.0 * 10_f64.powf(6.0)) as u64));
                                i += FRAME_SIZE as usize;
                                nms += 5;
                                if i % n == 0 && i != 0{
                                    i = 0;

                                }
                            }
                        });
                     }
                     else {
                        pressed[param1 as usize] = false;
                        drop(pressed);
                     }

            }
                println!("{}", status);
                println!("{}", param1);
                println!("{}", param2);
            }
        }
     );


    let _conn_in = midi_input.connect(&port, "M32", move |timing , msg, _| -> () {
        tx.send((msg[0], msg[1], msg[2], timing)).unwrap();
    }, ());
    let buffer = Arc::clone(&buffer_global);
    std::thread::spawn(move || {
//        let start_time = std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap();
//        let mut timing = 0;

        for package in package_receiver {
            let mut buffer = buffer.lock().unwrap();
            for (n, b) in buffer.iter_mut().enumerate() {
                *b += package.frame[n];
            }
        }

    });

   //loop {
   //    sleep(Duration::from_micros(2000));
   //};

   loop {
        let mut buffer = buffer_global.lock().unwrap();
        assert_eq!(io.writei(&buffer[..]).unwrap(), BUFFER_SIZE as usize);
        *buffer = [0_i16; BUFFER_SIZE as usize];
        drop(buffer);
        sleep(Duration::from_micros((265.0/44100.0 * 10_f64.powf(6.0)) as u64));
   };
   if pcm.state() != State::Running { pcm.start().unwrap() };
   pcm.drain().unwrap();
   Ok(())
}


fn main() {
    match run() {
        Ok(()) => return,
        Err(err) => println!("{}", err),
    }
}



