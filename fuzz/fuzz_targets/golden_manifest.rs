#![no_main]

use auralis_fuzz_targets::fuzz_golden_manifest;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    fuzz_golden_manifest(data);
});
