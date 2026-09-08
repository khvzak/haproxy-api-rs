#![no_main]

use haproxy_api::fuzz::exercise_pairs;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = exercise_pairs(data);
});
