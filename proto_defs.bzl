load("@prelude//rules.bzl", "genrule", "rust_binary", "rust_library")

ProtoSrcsInfo = provider(fields = ["srcs"])

def _proto_srcs_impl(ctx):
    srcs = {src.basename: src for src in ctx.attrs.srcs}
    for dep in ctx.attrs.deps:
        for src in dep[ProtoSrcsInfo].srcs:
            if src.basename in srcs:
                fail("Duplicate src:", src.basename)
            srcs[src.basename] = src
    out = ctx.actions.copied_dir(ctx.attrs.name, srcs)
    return [DefaultInfo(default_output = out), ProtoSrcsInfo(srcs = srcs.values())]

proto_srcs = rule(
    impl = _proto_srcs_impl,
    attrs = {
        "deps": attrs.list(attrs.dep(), default = []),
        "srcs": attrs.list(attrs.source(), default = []),
    },
)

def rust_protobuf_library(
        name,
        srcs,
        build_script,
        protos = None,
        deps = None,
        build_script_deps = None,
        build_env = None,
        proto_srcs = None,
        visibility = [],
        crate_name = None):
    """Compile protobuf files to Rust using prost-build.

    This mirrors the upstream buck2 proto_defs pattern:
    1. Compiles build_script into a rust_binary
    2. Runs it via genrule with PROTOC, PROTOC_INCLUDE, and BUCK_PROTO_SRCS set
    3. Wraps the generated .rs files in a rust_library with OUT_DIR

    Args:
        name: Name of the resulting rust_library target.
        srcs: Hand-written Rust source files (e.g. proto.rs with include! macros).
        build_script: Path to a build.rs that uses prost-build.
        protos: List of .proto files placed in the genrule cwd. Prefer proto_srcs.
        deps: Dependencies for the final rust_library.
        build_script_deps: Dependencies for the build script binary.
        build_env: Additional env vars for the genrule.
        proto_srcs: A proto_srcs() target; path is exposed as BUCK_PROTO_SRCS.
        visibility: Visibility of the resulting rust_library.
    """
    build_name = name + "-build"
    proto_name = name + "-proto"

    rust_binary(
        name = build_name,
        srcs = [build_script],
        crate_root = build_script,
        deps = build_script_deps or [],
        visibility = [],
    )

    env = build_env or {}
    env.update({
        "PROTOC": "$(exe toolchains//:protoc)",
        "PROTOC_INCLUDE": "$(location toolchains//:protoc_include)",
    })
    if proto_srcs:
        env["BUCK_PROTO_SRCS"] = "$(location {})".format(proto_srcs)

    genrule(
        name = proto_name,
        srcs = protos,
        cmd = "$(exe :{})".format(build_name),
        env = env,
        out = ".",
        visibility = [],
    )

    rust_library(
        name = crate_name or name,
        srcs = srcs,
        env = {
            "OUT_DIR": "$(location :{})".format(proto_name),
        },
        deps = (deps or []),
        visibility = visibility,
    )
