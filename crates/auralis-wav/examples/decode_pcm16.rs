//! Decodes a PCM16 WAV file and prints its metadata plus planar samples.

use std::{env, process::ExitCode};

use auralis_wav::decode_pcm16_path;

fn main() -> ExitCode {
    let Some(path) = env::args_os().nth(1) else {
        eprintln!("usage: decode_pcm16 <input.wav>");
        return ExitCode::FAILURE;
    };

    match decode_pcm16_path(path) {
        Ok(audio) => {
            println!("sample_rate={}", audio.spec().sample_rate().as_u32());
            println!("channels={}", audio.spec().channels().as_u16());
            println!("frames={}", audio.frames().as_u64());
            print!("planar=");
            for (index, sample) in audio.as_planar_f32().iter().enumerate() {
                if index > 0 {
                    print!(",");
                }
                print!("{sample:.10}");
            }
            println!();
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
