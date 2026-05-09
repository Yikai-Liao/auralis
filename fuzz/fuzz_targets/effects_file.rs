#![no_main]

use auralis_fuzz_targets::fuzz_effects_file;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    fuzz_effects_file(data);
});
