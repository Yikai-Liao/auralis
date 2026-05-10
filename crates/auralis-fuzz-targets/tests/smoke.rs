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
        b"allpass 1000 0.707q",
        b"allpass -1 500",
        b"band -n 1000 2q",
        b"bandpass -c 1000 2q",
        b"bandreject 1000 2q",
        b"bass 6 100 0.5s",
        b"treble -6 3000 0.5s",
        b"delay 2s 1s",
        b"downsample 3",
        b"upsample 3",
        b"speed 100c",
        b"stretch 1.5 10 q 0.75 0.25",
        b"tempo 1.25",
        b"tempo -q -s 1.25 35 12 10",
        b"pitch -q 1200 60 10 8",
        b"bend -f 40 -o 8 0s,100,+200s",
        b"splice -t 48s,4s,0s",
        b"stat -s 2 -rms -j",
        b"stats -x 16 -w 0.1 -j",
        b"synth -n 16s sine 1000 square 500",
        b"rate 44.1k",
        b"rate -q 24k",
        b"rate -Q 1 48000",
        b"rate -m 24k",
        b"rate -Q 7 48000",
        b"rate -h -M -s -A 95 -a -R 120 24k",
        b"chorus -l -t 0.5 1 1 0.25 1 0 -sine",
        b"compand 0.3,1 6:-70,-60,-20,-20,0,0 -3 -90 0.01",
        b"mcompand '0,0 -60,-60,0,0' 1k '0.01,0.1 -70,-60,0,-3'",
        b"fir 0.5 0.25",
        b"firfit 20 0 10k -3",
        b"hilbert -n 5",
        b"sinc -n 11 1000-4000",
        b"dither -S -p 8",
        b"noiseprof profile.prof",
        b"noisered profile.prof 0.25",
        b"silence -l 1 1s 0% -1 2s -40d",
        b"vad -T 0.01 -t 1 -g 0.1 -p 0.001",
        b"flanger -q -t 1 2 25 100 1 sine 50 none",
        b"phaser -q -t 0.8 0.74 3 0.4 0.5 -s",
        b"reverb -w 75 25 50 0 10 -3",
        b"echo 0.5 1 1 0.5",
        b"echos 0.5 1 1 0.5",
        b"highpass -1 500",
        b"lowpass -1 500",
        b"biquad 0.5 0 0 1 -0.5 0",
        b"centercut -a 0.5 -b -w 16",
        b"channels 2",
        b"norm -6",
        b"contrast 25",
        b"overdrive 12 25",
        b"saturation sqrt 0.75 0.1 0.25",
        b"swap",
        b"oops",
        b"repeat 2",
        b"remix -a -p 1v0.5,2p-6 0 -2i0",
        b"tremolo 5 75",
        b"vol 2 amplitude 0.05",
        b"vol -0.25 power",
        b"pad 1 2@3 4",
        b"deemph",
        b"riaa",
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
