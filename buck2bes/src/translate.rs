use crate::proto::buck::data::{
    self, buck_event, command_start, instant_event, span_end_event, span_start_event, BuckEvent,
    ConsoleMessage, ConsoleWarning,
};
use crate::proto::build_event_stream::{
    self, build_event, build_event_id, BuildEvent, BuildEventId, BuildFinished, BuildStarted,
    Progress,
};
use crate::proto::google::protobuf as buck_pb;

/// Translates a stream of Buck2 `BuckEvent`s into Bazel Build Event Stream `BuildEvent`s.
///
/// Maintains state across events to:
/// - Track the build start timestamp (needed to compute `finish_time`)
/// - Assign sequential progress event IDs for proper BES event chaining
/// - Store the trace ID used as the BES build UUID
pub struct BuckEventTranslator {
    progress_count: i32,
    start_timestamp: Option<buck_pb::Timestamp>,
    trace_id: Option<String>,
}

impl BuckEventTranslator {
    pub fn new() -> Self {
        Self {
            progress_count: 0,
            start_timestamp: None,
            trace_id: None,
        }
    }

    /// Translate a single `BuckEvent` into zero or more BES `BuildEvent`s.
    ///
    /// Returns an empty vec for events that have no BES equivalent (yet).
    pub fn translate(&mut self, event: &BuckEvent) -> Vec<BuildEvent> {
        let data = match &event.data {
            Some(d) => d,
            None => return vec![],
        };

        match data {
            buck_event::Data::SpanStart(span_start) => self.translate_span_start(event, span_start),
            buck_event::Data::SpanEnd(span_end) => self.translate_span_end(event, span_end),
            buck_event::Data::Instant(instant) => self.translate_instant(event, instant),
            buck_event::Data::Record(_) => vec![],
        }
    }

    fn translate_span_start(
        &mut self,
        event: &BuckEvent,
        span_start: &data::SpanStartEvent,
    ) -> Vec<BuildEvent> {
        let data = match &span_start.data {
            Some(d) => d,
            None => return vec![],
        };

        match data {
            span_start_event::Data::Command(cmd) => self.translate_command_start(event, cmd),
            _ => vec![],
        }
    }

    fn translate_span_end(
        &mut self,
        event: &BuckEvent,
        span_end: &data::SpanEndEvent,
    ) -> Vec<BuildEvent> {
        let data = match &span_end.data {
            Some(d) => d,
            None => return vec![],
        };

        match data {
            span_end_event::Data::Command(cmd) => self.translate_command_end(event, span_end, cmd),
            _ => vec![],
        }
    }

    fn translate_instant(
        &mut self,
        _event: &BuckEvent,
        instant: &data::InstantEvent,
    ) -> Vec<BuildEvent> {
        let data = match &instant.data {
            Some(d) => d,
            None => return vec![],
        };

        match data {
            instant_event::Data::ConsoleMessage(msg) => self.translate_console_message(msg),
            instant_event::Data::ConsoleWarning(warn) => self.translate_console_warning(warn),
            _ => vec![],
        }
    }

    #[allow(deprecated)]
    fn translate_command_start(
        &mut self,
        event: &BuckEvent,
        cmd: &data::CommandStart,
    ) -> Vec<BuildEvent> {
        self.start_timestamp = event.timestamp.clone();
        self.trace_id = Some(event.trace_id.clone());

        let command_name = command_name(cmd);

        vec![BuildEvent {
            id: Some(BuildEventId {
                id: Some(build_event_id::Id::Started(
                    build_event_id::BuildStartedId {},
                )),
            }),
            children: vec![
                BuildEventId {
                    id: Some(build_event_id::Id::Progress(build_event_id::ProgressId {
                        opaque_count: 1,
                    })),
                },
                BuildEventId {
                    id: Some(build_event_id::Id::BuildFinished(
                        build_event_id::BuildFinishedId {},
                    )),
                },
            ],
            last_message: false,
            payload: Some(build_event::Payload::Started(BuildStarted {
                uuid: event.trace_id.clone(),
                start_time: event.timestamp,
                start_time_millis: 0,
                build_tool_version: String::new(),
                options_description: String::new(),
                command: command_name,
                working_directory: String::new(),
                workspace_directory: String::new(),
                server_pid: 0,
                host: String::new(),
                user: String::new(),
                java_version_info: None,
            })),
        }]
    }

    #[allow(deprecated)]
    fn translate_command_end(
        &mut self,
        event: &BuckEvent,
        span_end: &data::SpanEndEvent,
        cmd: &data::CommandEnd,
    ) -> Vec<BuildEvent> {
        let success = cmd.is_success;

        let (code, name) = if success {
            (0, "SUCCESS".to_string())
        } else {
            (1, "BUILD_FAILURE".to_string())
        };

        // Compute finish_time from start_timestamp + duration, or fall back to event timestamp
        let finish_time = match (&self.start_timestamp, &span_end.duration) {
            (Some(start), Some(dur)) => Some(add_duration(start, dur)),
            _ => event.timestamp,
        };

        vec![BuildEvent {
            id: Some(BuildEventId {
                id: Some(build_event_id::Id::BuildFinished(
                    build_event_id::BuildFinishedId {},
                )),
            }),
            children: vec![],
            last_message: true,
            payload: Some(build_event::Payload::Finished(BuildFinished {
                overall_success: success,
                exit_code: Some(build_event_stream::build_finished::ExitCode { name, code }),
                finish_time_millis: 0,
                finish_time,
                anomaly_report: None,
                failure_detail: None,
            })),
        }]
    }

    fn translate_console_message(&mut self, msg: &ConsoleMessage) -> Vec<BuildEvent> {
        self.make_progress_event(String::new(), msg.message.clone())
    }

    fn translate_console_warning(&mut self, warn: &ConsoleWarning) -> Vec<BuildEvent> {
        self.make_progress_event(String::new(), warn.message.clone())
    }

    fn make_progress_event(&mut self, stdout: String, stderr: String) -> Vec<BuildEvent> {
        self.progress_count += 1;
        let current_count = self.progress_count;

        vec![BuildEvent {
            id: Some(BuildEventId {
                id: Some(build_event_id::Id::Progress(build_event_id::ProgressId {
                    opaque_count: current_count,
                })),
            }),
            children: vec![BuildEventId {
                id: Some(build_event_id::Id::Progress(build_event_id::ProgressId {
                    opaque_count: current_count + 1,
                })),
            }],
            last_message: false,
            payload: Some(build_event::Payload::Progress(Progress { stdout, stderr })),
        }]
    }
}

/// Extract the command name from a `CommandStart` based on which variant is set.
fn command_name(cmd: &data::CommandStart) -> String {
    match &cmd.data {
        Some(command_start::Data::Build(_)) => "build",
        Some(command_start::Data::Test(_)) => "test",
        Some(command_start::Data::Targets(_)) => "targets",
        Some(command_start::Data::Query(_)) => "query",
        Some(command_start::Data::Cquery(_)) => "cquery",
        Some(command_start::Data::Aquery(_)) => "aquery",
        Some(command_start::Data::Audit(_)) => "audit",
        Some(command_start::Data::Docs(_)) => "docs",
        Some(command_start::Data::Clean(_)) => "clean",
        Some(command_start::Data::Install(_)) => "install",
        Some(command_start::Data::Materialize(_)) => "materialize",
        Some(command_start::Data::Profile(_)) => "profile",
        Some(command_start::Data::Bxl(_)) => "bxl",
        Some(command_start::Data::Lsp(_)) => "lsp",
        Some(command_start::Data::FileStatus(_)) => "file_status",
        Some(command_start::Data::Starlark(_)) => "starlark",
        Some(command_start::Data::Subscribe(_)) => "subscribe",
        Some(command_start::Data::Trace(_)) => "trace",
        Some(command_start::Data::Ctargets(_)) => "ctargets",
        Some(command_start::Data::StarlarkDebugAttach(_)) => "starlark_debug_attach",
        Some(command_start::Data::Explain(_)) => "explain",
        Some(command_start::Data::ExpandExternalCell(_)) => "expand_external_cell",
        Some(command_start::Data::Complete(_)) => "complete",
        None => "unknown",
    }
    .to_string()
}

/// Add a Duration to a Timestamp.
fn add_duration(ts: &buck_pb::Timestamp, dur: &buck_pb::Duration) -> buck_pb::Timestamp {
    let mut seconds = ts.seconds + dur.seconds;
    let mut nanos = ts.nanos + dur.nanos;
    if nanos >= 1_000_000_000 {
        seconds += 1;
        nanos -= 1_000_000_000;
    } else if nanos < 0 {
        seconds -= 1;
        nanos += 1_000_000_000;
    }
    buck_pb::Timestamp { seconds, nanos }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proto::buck::data::*;

    fn make_timestamp(seconds: i64) -> buck_pb::Timestamp {
        buck_pb::Timestamp { seconds, nanos: 0 }
    }

    fn make_buck_event(
        trace_id: &str,
        timestamp: buck_pb::Timestamp,
        data: buck_event::Data,
    ) -> BuckEvent {
        BuckEvent {
            timestamp: Some(timestamp),
            trace_id: trace_id.to_string(),
            span_id: 1,
            parent_id: 0,
            data: Some(data),
        }
    }

    #[test]
    fn test_command_start_produces_build_started() {
        let mut translator = BuckEventTranslator::new();
        let ts = make_timestamp(1700000000);
        let event = make_buck_event(
            "abc-123",
            ts.clone(),
            buck_event::Data::SpanStart(SpanStartEvent {
                data: Some(span_start_event::Data::Command(CommandStart {
                    metadata: Default::default(),
                    cli_args: vec!["build".into(), "//...".into()],
                    tags: vec![],
                    data: Some(command_start::Data::Build(BuildCommandStart {})),
                })),
            }),
        );

        let result = translator.translate(&event);
        assert_eq!(result.len(), 1);

        let bes_event = &result[0];
        match &bes_event.payload {
            Some(build_event::Payload::Started(started)) => {
                assert_eq!(started.uuid, "abc-123");
                assert_eq!(started.command, "build");
                assert_eq!(started.start_time, Some(ts));
            }
            other => panic!("Expected BuildStarted, got {:?}", other),
        }

        assert_eq!(bes_event.children.len(), 2);
    }

    #[test]
    fn test_command_end_produces_build_finished() {
        let mut translator = BuckEventTranslator::new();

        // Send CommandStart first to initialize state
        let start_event = make_buck_event(
            "abc-123",
            make_timestamp(1700000000),
            buck_event::Data::SpanStart(SpanStartEvent {
                data: Some(span_start_event::Data::Command(CommandStart {
                    metadata: Default::default(),
                    cli_args: vec![],
                    tags: vec![],
                    data: Some(command_start::Data::Build(BuildCommandStart {})),
                })),
            }),
        );
        translator.translate(&start_event);

        // Now send CommandEnd
        let end_event = make_buck_event(
            "abc-123",
            make_timestamp(1700000010),
            buck_event::Data::SpanEnd(SpanEndEvent {
                stats: None,
                duration: Some(buck_pb::Duration {
                    seconds: 10,
                    nanos: 0,
                }),
                data: Some(span_end_event::Data::Command(CommandEnd {
                    is_success: true,
                    build_result: None,
                    data: Some(command_end::Data::Build(BuildCommandEnd {
                        unresolved_target_patterns: vec![],
                    })),
                })),
            }),
        );

        let result = translator.translate(&end_event);
        assert_eq!(result.len(), 1);

        let bes_event = &result[0];
        assert!(bes_event.last_message);
        match &bes_event.payload {
            Some(build_event::Payload::Finished(finished)) => {
                let exit = finished.exit_code.as_ref().unwrap();
                assert_eq!(exit.code, 0);
                assert_eq!(exit.name, "SUCCESS");
                let ft = finished.finish_time.as_ref().unwrap();
                assert_eq!(ft.seconds, 1700000010);
            }
            other => panic!("Expected BuildFinished, got {:?}", other),
        }
    }

    #[test]
    fn test_console_message_produces_progress() {
        let mut translator = BuckEventTranslator::new();
        let event = make_buck_event(
            "abc-123",
            make_timestamp(1700000000),
            buck_event::Data::Instant(InstantEvent {
                data: Some(instant_event::Data::ConsoleMessage(ConsoleMessage {
                    message: "Building target //foo:bar".to_string(),
                })),
            }),
        );

        let result = translator.translate(&event);
        assert_eq!(result.len(), 1);

        match &result[0].payload {
            Some(build_event::Payload::Progress(progress)) => {
                assert_eq!(progress.stderr, "Building target //foo:bar");
                assert!(progress.stdout.is_empty());
            }
            other => panic!("Expected Progress, got {:?}", other),
        }

        match &result[0].id.as_ref().unwrap().id {
            Some(build_event_id::Id::Progress(p)) => assert_eq!(p.opaque_count, 1),
            other => panic!("Expected ProgressId, got {:?}", other),
        }
    }

    #[test]
    fn test_progress_events_chain_sequentially() {
        let mut translator = BuckEventTranslator::new();

        for i in 1..=3 {
            let event = make_buck_event(
                "abc-123",
                make_timestamp(1700000000),
                buck_event::Data::Instant(InstantEvent {
                    data: Some(instant_event::Data::ConsoleMessage(ConsoleMessage {
                        message: format!("msg {i}"),
                    })),
                }),
            );
            let result = translator.translate(&event);
            let bes = &result[0];

            match &bes.id.as_ref().unwrap().id {
                Some(build_event_id::Id::Progress(p)) => {
                    assert_eq!(p.opaque_count, i as i32)
                }
                other => panic!("Expected ProgressId, got {:?}", other),
            }

            match &bes.children[0].id {
                Some(build_event_id::Id::Progress(p)) => {
                    assert_eq!(p.opaque_count, i as i32 + 1)
                }
                other => panic!("Expected ProgressId child, got {:?}", other),
            }
        }
    }
}
