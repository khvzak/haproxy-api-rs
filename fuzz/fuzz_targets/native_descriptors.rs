#![no_main]

use haproxy_api::fuzz::exercise_descriptors;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    exercise_descriptors(data);
});
