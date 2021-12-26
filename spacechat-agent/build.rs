use std::env;
use std::fs;
use std::path::Path;

use protobuf_codegen::Codegen;
use protobuf_codegen::Customize;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();

    let generated_dir = format!("{}/generated", out_dir);

    if Path::new(&generated_dir).exists() {
        fs::remove_dir_all(&generated_dir).unwrap();
    }
    fs::create_dir(&generated_dir).unwrap();

    Codegen::new()
        .pure()
        .customize(Customize {
            serde_derive: Some(true),
            gen_mod_rs: Some(true),
            ..Default::default()
        })
        .out_dir(generated_dir)
        .input("src/protos/spacechat-agent.proto")
        .include("src/protos")
        .run_from_script();
}

