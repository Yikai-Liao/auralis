#![no_main]

use auralis_fuzz_targets::fuzz_wav_parser;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    fuzz_wav_parser(data);
});
