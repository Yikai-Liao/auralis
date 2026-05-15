//! Integration tests for the `auralis convert` command.

mod support;

use support::*;

const POP_FLAC: &[u8] = &[
    0x66, 0x4c, 0x61, 0x43, 0x80, 0x00, 0x00, 0x22, 0x10, 0x00, 0x10, 0x00, 0x00, 0x00, 0x3f, 0x00,
    0x00, 0x3f, 0x0a, 0xc4, 0x40, 0xf0, 0x00, 0x00, 0x00, 0x64, 0x68, 0x46, 0x42, 0x88, 0xfa, 0x5e,
    0x19, 0x83, 0x55, 0x16, 0x97, 0x2d, 0xcf, 0x47, 0x22, 0x3c, 0xff, 0xf8, 0x69, 0x08, 0x00, 0x63,
    0x33, 0x18, 0x00, 0x00, 0x08, 0x04, 0x10, 0x01, 0x17, 0xee, 0x00, 0xa3, 0x2e, 0x4e, 0x8a, 0x65,
    0xda, 0x6a, 0x95, 0x2e, 0x62, 0x89, 0xe9, 0x2f, 0x32, 0xa4, 0xd7, 0x27, 0x49, 0xcc, 0x56, 0xde,
    0x37, 0xed, 0x66, 0x73, 0x14, 0x8e, 0xcb, 0xae, 0xf2, 0x59, 0x14, 0x87, 0x31, 0x53, 0xa3, 0xb2,
    0xcd, 0xe4, 0x8a, 0x47, 0xdb, 0xcd, 0x60, 0xb5, 0x75,
];

#[test]
fn convert_wav_to_flac_writes_flac_header() {
    let input = temp_path("auralis-cli-convert-wav-to-flac-input", "wav");
    let output = temp_path("auralis-cli-convert-wav-to-flac-output", "flac");
    write_pcm16_wav(
        &input,
        1,
        &[
            -16_384, -14_336, -12_288, -10_240, -8_192, -6_144, -4_096, -2_048, 0, 2_048, 4_096,
            6_144, 8_192, 10_240, 12_288, 14_336,
        ],
    );

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "convert",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        command_output.status.success(),
        "stderr: {}",
        stderr(&command_output)
    );
    let bytes = fs::read(&output).unwrap();
    assert_eq!(&bytes[..4], b"fLaC");

    fs::remove_file(input).unwrap();
    fs::remove_file(output).unwrap();
}

#[test]
fn convert_flac_to_wav_decodes_into_pcm16_wav() {
    let input = temp_path("auralis-cli-convert-flac-to-wav-input", "flac");
    let output = temp_path("auralis-cli-convert-flac-to-wav-output", "wav");
    fs::write(&input, POP_FLAC).unwrap();

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "convert",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        command_output.status.success(),
        "stderr: {}",
        stderr(&command_output)
    );
    let (sample_rate, channels, samples) = read_pcm16_wav_with_sample_rate(&output);
    assert_eq!(sample_rate, 44_100);
    assert_eq!(channels, 1);
    assert_eq!(samples.len(), 100);

    fs::remove_file(input).unwrap();
    fs::remove_file(output).unwrap();
}

#[test]
fn convert_rejects_wav_sample_for_non_wav_output() {
    let input = temp_path("auralis-cli-convert-sample-input", "wav");
    let output = temp_path("auralis-cli-convert-sample-output", "flac");
    write_pcm16_wav(&input, 1, &[0]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "convert",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--sample",
            "pcm24",
        ])
        .output()
        .unwrap();

    assert!(!command_output.status.success());
    let stderr = stderr(&command_output);
    assert!(
        stderr.contains("error: --sample is supported only for WAV output"),
        "{stderr}"
    );

    fs::remove_file(input).unwrap();
    let _ = fs::remove_file(output);
}
