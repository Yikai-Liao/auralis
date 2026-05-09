#![no_main]

use auralis_fuzz_targets::fuzz_unsupported_wav_format;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    fuzz_unsupported_wav_format(data);
});
