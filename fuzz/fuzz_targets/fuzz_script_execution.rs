#![no_main]

use libfuzzer_sys::fuzz_target;
use bitcoinsv::bitcoin::{Script, ScriptInterpreter};

fuzz_target!(|data: &[u8]| {
    // Create a script from the fuzzer input
    let script = Script { raw: bytes::Bytes::from(data.to_vec()) };
    
    // Try to execute the script
    let mut interpreter = ScriptInterpreter::new();
    let _ = interpreter.eval_script(&script);
    
    // Also test script parsing
    let _ = script.operations();
    
    // Test script size limits
    let _ = script.len();
});