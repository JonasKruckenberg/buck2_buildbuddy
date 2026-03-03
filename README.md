# Buck2 Remote Execution 

This is a test case for building a not so simple Rust application using [`buck2`](https://buck2.build) and [`BuildBuddy`](https://buildbuddy.io).

## Overview

`buck2bes_proto` is a Rust crate that reexports code generated from protobuf files in `proto`. It uses the same pattern as the `buck2` project, 
where the actual code generation is done by a Rust build script and a simple build rule simply wraps the build script, code generated, and
rust library. This rule in in `proto_defs.bzl`.

`buck2bes` is a Rust crate that imports `buck2bes_proto` and other third-party Rust crates from crates.io (managed by [reindeer](https://github.com/facebookincubator/reindeer)).
This crate is the main entrypoint of this workspace.

`platforms` defines the local and remote execution platforms. These are set up so we can force buck2 to use either one (and not cheat by only running locally).
If you want to switch between local and remote execution, simply switch which one is uncommented in `platforms/BUCK` line ~20.

`toolchains` contains the toolchain definitions. This repo uses the [`buck2tf`](https://github.com/JonasKruckenberg/buck2tf) hermetic Rust toolchain instead of the default one.


