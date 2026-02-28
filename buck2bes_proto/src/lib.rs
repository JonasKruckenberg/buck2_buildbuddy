pub mod buck {
    pub mod data {
        include!(concat!(env!("OUT_DIR"), "/buck.data.rs"));

        pub mod error {
            include!(concat!(env!("OUT_DIR"), "/buck.data.error.rs"));
        }
    }

    pub mod daemon {
        include!(concat!(env!("OUT_DIR"), "/buck.daemon.rs"));
    }

    pub mod subscription {
        include!(concat!(env!("OUT_DIR"), "/buck.subscription.rs"));
    }

    pub mod host_sharing {
        include!(concat!(env!("OUT_DIR"), "/buck.host_sharing.rs"));
    }
}

pub mod google {
    pub mod protobuf {
        include!(concat!(env!("OUT_DIR"), "/google.protobuf.rs"));
    }
}

pub mod build_event_stream {
    include!(concat!(env!("OUT_DIR"), "/build_event_stream.rs"));
}

pub mod blaze {
    include!(concat!(env!("OUT_DIR"), "/blaze.rs"));

    pub mod invocation_policy {
        include!(concat!(env!("OUT_DIR"), "/blaze.invocation_policy.rs"));
    }

    pub mod strategy_policy {
        include!(concat!(env!("OUT_DIR"), "/blaze.strategy_policy.rs"));
    }
}

pub mod command_line {
    include!(concat!(env!("OUT_DIR"), "/command_line.rs"));
}

pub mod failure_details {
    include!(concat!(env!("OUT_DIR"), "/failure_details.rs"));
}

pub mod options {
    include!(concat!(env!("OUT_DIR"), "/options.rs"));
}

pub mod devtools_blaze_proto {
    include!(concat!(env!("OUT_DIR"), "/devtools_blaze_proto.rs"));
}

pub mod devtools {
    pub mod build {
        pub mod lib {
            pub mod packages {
                pub mod metrics {
                    include!(concat!(
                        env!("OUT_DIR"),
                        "/devtools.build.lib.packages.metrics.rs"
                    ));
                }
            }
        }
    }
}
