use std::env;
use std::ffi::OsString;
use std::io::Result;
use std::path::PathBuf;

fn main() -> Result<()> {
    let proto_dir = PathBuf::from(env::var("BUCK_PROTO_SRCS").unwrap());

    let protos: Vec<PathBuf> = std::fs::read_dir(&proto_dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .collect();

    for proto_file in &protos {
        println!("cargo:rerun-if-changed={}", proto_file.display());
    }

    // Buck likes to set $OUT in a genrule, while Cargo likes to set $OUT_DIR.
    let out_dir = get_env("OUT_DIR")
        .or_else(|| get_env("OUT"))
        .expect("OUT_DIR or OUT must be set");

    tonic_prost_build::configure()
        .build_server(false)
        .build_client(true)
        .compile_well_known_types(true)
        .protoc_arg("--experimental_allow_proto3_optional")
        .type_attribute(".", "#[derive(::serde::Deserialize)]")
        .type_attribute(".", "#[serde(rename_all = \"camelCase\")]")
        .out_dir(out_dir)
        .compile_protos(&protos, &[proto_dir])?;

    Ok(())
}

fn get_env(key: &str) -> Option<OsString> {
    println!("cargo:rerun-if-env-changed={key}");
    env::var_os(key)
}
