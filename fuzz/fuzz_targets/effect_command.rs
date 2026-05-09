#![no_main]

use auralis_fuzz_targets::fuzz_effect_command;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    fuzz_effect_command(data);
});
