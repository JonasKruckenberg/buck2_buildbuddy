use std::io::Result;

fn main() -> Result<()> {
    let buck2_dir = "../proto/buck2";
    let bes_dir = "../proto/bes";

    let mut config = prost_build::Config::new();
    config.type_attribute(".", "#[derive(::serde::Deserialize)]");
    let fds = config.load_fds(
        &[
            format!("{buck2_dir}/data.proto"),
            format!("{buck2_dir}/error.proto"),
            format!("{buck2_dir}/host_sharing.proto"),
        ],
        &[buck2_dir],
    )?;
    config.compile_well_known_types();
    config.type_attribute(".", "#[serde(rename_all = \"camelCase\")]");
    config.compile_fds(fds)?;
    // config.compile_protos(
    //     &[
    //         format!("{buck2_dir}/data.proto"),
    //         format!("{buck2_dir}/error.proto"),
    //         format!("{buck2_dir}/host_sharing.proto"),
    //     ],
    //     &[buck2_dir],
    // )?;

    prost_build::compile_protos(
        &[
            format!("{bes_dir}/build_event_stream.proto"),
            format!("{bes_dir}/action_cache.proto"),
            format!("{bes_dir}/analysis_cache_service_metadata_status.proto"),
            format!("{bes_dir}/command_line.proto"),
            format!("{bes_dir}/failure_details.proto"),
            format!("{bes_dir}/invocation_policy.proto"),
            format!("{bes_dir}/option_filters.proto"),
            format!("{bes_dir}/package_load_metrics.proto"),
            format!("{bes_dir}/strategy_policy.proto"),
        ],
        &[bes_dir],
    )?;

    Ok(())
}
