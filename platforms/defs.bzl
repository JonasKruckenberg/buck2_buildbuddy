load("@prelude//configurations:util.bzl", "util")
load("@prelude//cfg/exec_platform:marker.bzl", "get_exec_platform_marker")

def _remote_execution_platform_impl(ctx: AnalysisContext) -> list[Provider]:
    constraints = dict()
    constraints.update(ctx.attrs.cpu_configuration[ConfigurationInfo].constraints)
    constraints.update(ctx.attrs.os_configuration[ConfigurationInfo].constraints)
    cfg = ConfigurationInfo(constraints = constraints, values = {})

    name = ctx.label.raw_target()
    platform = ExecutionPlatformInfo(
        label = name,
        configuration = cfg,
        executor_config = CommandExecutorConfig(
            local_enabled = False,
            remote_enabled = True,
            # use_limited_hybrid = True,
            use_windows_path_separators = False,
            remote_execution_properties = {
                "OSFamily": "Linux",
                "container-image": "docker://gcr.io/flame-public/rbe-ubuntu24-04:latest",
            },
            remote_execution_use_case = "buck2-default",
            remote_output_paths = "output_paths",
        ),
    )

    return [
        DefaultInfo(),
        platform,
        PlatformInfo(label = str(name), configuration = cfg),
        ExecutionPlatformRegistrationInfo(
            platforms = [platform],
            exec_marker_constraint = get_exec_platform_marker(),
        ),
    ]

remote_execution_platform = rule(
    impl = _remote_execution_platform_impl,
    attrs = {
        "cpu_configuration": attrs.dep(providers = [ConfigurationInfo]),
        "os_configuration": attrs.dep(providers = [ConfigurationInfo]),
    },
)

def _execution_platforms_impl(ctx: AnalysisContext) -> list[Provider]:
    exec_platforms = []
    for platforms in ctx.attrs.platforms:
        exec_platforms = exec_platforms + platforms[ExecutionPlatformRegistrationInfo].platforms

    return [
        DefaultInfo(),
        ExecutionPlatformRegistrationInfo(platforms = exec_platforms),
    ]

execution_platforms = rule(
    impl = _execution_platforms_impl,
    attrs = {
        "platforms": attrs.list(attrs.dep(providers = [ExecutionPlatformRegistrationInfo])),
    },
)
