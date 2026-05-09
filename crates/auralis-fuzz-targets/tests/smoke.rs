//! Deterministic smoke tests for L7 fuzz target drivers.

use auralis_fuzz_targets::{
    fuzz_effect_command, fuzz_effects_file, fuzz_golden_manifest, fuzz_unsupported_wav_format,
    fuzz_wav_parser,
};

const MINIMAL_PCM16_WAV: &[u8] = &[
    b'R', b'I', b'F', b'F', 38, 0, 0, 0, b'W', b'A', b'V', b'E', b'f', b'm', b't', b' ', 16, 0, 0,
    0, 1, 0, 1, 0, 0x80, 0xbb, 0, 0, 0, 0x77, 1, 0, 2, 0, 16, 0, b'd', b'a', b't', b'a', 2, 0, 0,
    0, 0, 0,
];

#[test]
fn fuzz_target_drivers_accept_seed_inputs() {
    for seed in seeds() {
        fuzz_wav_parser(seed);
        fuzz_unsupported_wav_format(seed);
        fuzz_effect_command(seed);
        fuzz_effects_file(seed);
        fuzz_golden_manifest(seed);
    }
}

fn seeds() -> Vec<&'static [u8]> {
    vec![
        b"",
        b"not a wav",
        MINIMAL_PCM16_WAV,
        b"gain -3",
        b"gain -n -3",
        b"gain -l 6",
        b"gain -e -3",
        b"gain -Bn 6",
        b"gain -h -6 gain -r",
        b"norm -6",
        b"contrast 25",
        b"vol 2 amplitude 0.05",
        b"vol -0.25 power",
        b"pad 1 2@3 4",
        b"dcshift 0.25 : reverse\nfade l 2 3\n",
        b"'unterminated",
        br#"[id.case]
input = "in.wav"
corpus_id = "sine_1k_mono_480"
auralis = ["run"]
sox_ng = ["gain", "-3"]
max_abs = 0.0
rms = 0.0
snr_db = 120.0
"#,
        &[0, 255, 128, 64, b'R', b'I', b'F', b'F'],
    ]
}
