//! SoX-ng reference tests for PCM16 WAV decode and encode behavior.

use std::{env, fs, process::Command};

use auralis_wav::decode_pcm16_path;

mod support;

use support::{
    assert_close_by_one_lsb, audio_buffer, encode_temp_wav, interleave_planar, read_le_f32,
    temp_path, write_le_f32, write_temp_wav,
};

#[test]
fn mono_decode_matches_sox_ng_golden_raw_float_reference() {
    compare_sox_ng(1, &[-32768, -1234, 0, 12_345, 32_767]);
}

#[test]
fn stereo_decode_matches_sox_ng_golden_raw_float_reference() {
    compare_sox_ng(2, &[-32768, 32_767, -12_000, 12_000, 0, 4096]);
}

#[test]
fn encoded_stereo_samples_match_sox_ng_pcm16_reference() {
    let channels = 2;
    let audio = audio_buffer(2, 3, &[-0.75, 0.0, 0.75, -0.5, 0.5, 0.999_969_5]);
    let auralis_output = encode_temp_wav("auralis-wav-sox-encode-auralis", &audio);
    let raw_input = temp_path("auralis-wav-sox-encode-input", "f32");
    let sox_output = temp_path("auralis-wav-sox-encode-output", "wav");
    write_le_f32(
        &raw_input,
        &interleave_planar(audio.as_planar_f32(), channels),
    )
    .unwrap();

    let sox_ng = env::var("AURALIS_SOX_NG_BIN").unwrap_or_else(|_| "sox_ng".to_owned());
    let status = Command::new(sox_ng)
        .args([
            "-R",
            "-D",
            "-t",
            "f32",
            "-r",
            "48000",
            "-c",
            "2",
            raw_input.to_str().unwrap(),
            "-b",
            "16",
            "-e",
            "signed-integer",
            sox_output.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());

    let auralis = decode_pcm16_path(&auralis_output).unwrap();
    let sox = decode_pcm16_path(&sox_output).unwrap();

    fs::remove_file(auralis_output).unwrap();
    fs::remove_file(raw_input).unwrap();
    fs::remove_file(sox_output).unwrap();

    assert_close_by_one_lsb(auralis.as_planar_f32(), sox.as_planar_f32());
}

fn compare_sox_ng(channels: u16, samples: &[i16]) {
    let input = write_temp_wav("auralis-wav-sox-input", channels, samples).unwrap();
    let output = temp_path("auralis-wav-sox-output", "f32");
    let sox_ng = env::var("AURALIS_SOX_NG_BIN").unwrap_or_else(|_| "sox_ng".to_owned());
    let status = Command::new(sox_ng)
        .args([
            "-R",
            "-D",
            input.to_str().unwrap(),
            "-t",
            "f32",
            output.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());

    let audio = decode_pcm16_path(&input).unwrap();
    let sox_samples = read_le_f32(&output).unwrap();

    fs::remove_file(input).unwrap();
    fs::remove_file(output).unwrap();

    assert_eq!(
        interleave_planar(audio.as_planar_f32(), channels),
        sox_samples
    );
}
