use std::env;
use std::ffi::OsString;
use std::io;
use std::io::Result;
use std::path::Path;
use std::path::PathBuf;

fn main() -> Result<()> {
    let proto_dir = env::var("BUCK_PROTO_SRCS").unwrap();

    let protos: Vec<PathBuf> = std::fs::read_dir(&proto_dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .collect();

    Builder::new()
        .type_attribute(".", "#[derive(::serde::Deserialize)]")
        .type_attribute(".", "#[serde(rename_all = \"camelCase\")]")
        .compile(&protos, &[proto_dir])?;

    Ok(())
}

/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

fn get_env(key: &str) -> Option<OsString> {
    println!("cargo:rerun-if-env-changed={key}");
    env::var_os(key)
}

pub struct Builder {
    prost: prost_build::Config,
}

impl Builder {
    pub fn new() -> Self {
        let mut prost = prost_build::Config::new();
        prost.compile_well_known_types();
        // We want to use optional everywhere
        prost.protoc_arg("--experimental_allow_proto3_optional");

        Self { prost }
    }

    pub fn type_attribute<P: AsRef<str>, A: AsRef<str>>(mut self, path: P, attribute: A) -> Self {
        self.prost.type_attribute(path, attribute);
        self
    }

    pub fn field_attribute<P: AsRef<str>, A: AsRef<str>>(mut self, path: P, attribute: A) -> Self {
        self.prost.field_attribute(path, attribute);
        self
    }

    pub fn compile(
        self,
        protos: &[impl AsRef<Path>],
        includes: &[impl AsRef<Path>],
    ) -> io::Result<()> {
        let Self { mut prost } = self;

        // Buck likes to set $OUT in a genrule, while Cargo likes to set $OUT_DIR.
        // If we have $OUT set only, move it into the config
        if get_env("OUT_DIR").is_none() {
            if let Some(out) = get_env("OUT") {
                prost.out_dir(out);
            }
        }

        // Tell Cargo that if the given file changes, to rerun this build script.
        for proto_file in protos {
            println!("cargo:rerun-if-changed={}", proto_file.as_ref().display());
        }

        prost.compile_protos(protos, includes)
    }
}
