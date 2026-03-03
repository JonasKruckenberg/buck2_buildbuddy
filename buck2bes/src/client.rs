use buck2bes_proto::build_event_stream;
use buck2bes_proto::google::devtools::build::v1::{
    BuildEvent, BuildStatus, OrderedBuildEvent, PublishBuildToolEventStreamRequest,
    PublishLifecycleEventRequest, StreamId, build_event, build_status,
    publish_build_event_client::PublishBuildEventClient,
    publish_lifecycle_event_request::ServiceLevel, stream_id::BuildComponent,
};
use buck2bes_proto::google::protobuf::{Any, Timestamp};
use prost::Message;
use tonic::metadata::MetadataValue;
use tonic::service::interceptor::InterceptedService;
use tonic::transport::Channel;

type Interceptor =
    Box<dyn FnMut(tonic::Request<()>) -> Result<tonic::Request<()>, tonic::Status> + Send>;

pub struct BesClient<'a> {
    client: PublishBuildEventClient<InterceptedService<Channel, Interceptor>>,
    build_id: &'a str,
    invocation_id: &'a str,
    project_id: &'a str,
}

impl<'a> BesClient<'a> {
    pub async fn connect(
        endpoint: &'static str,
        api_key: &str,
        build_id: &'a str,
        invocation_id: &'a str,
        project_id: &'a str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let channel = Channel::from_static(endpoint)
            .http2_keep_alive_interval(std::time::Duration::from_secs(10))
            .connect()
            .await?;

        let api_key: MetadataValue<_> = api_key.parse()?;

        let interceptor: Interceptor = Box::new(move |mut req: tonic::Request<()>| {
            req.metadata_mut()
                .insert("x-buildbuddy-api-key", api_key.clone());
            Ok(req)
        });
        let client = PublishBuildEventClient::with_interceptor(channel, interceptor);

        Ok(BesClient {
            client,
            build_id,
            invocation_id,
            project_id,
        })
    }

    /// Publish a complete build event stream following the BES lifecycle protocol.
    ///
    /// Takes already-translated BES `BuildEvent` protos (from `BuckEventTranslator`)
    /// and wraps them in the full lifecycle:
    /// 1. BuildEnqueued
    /// 2. InvocationAttemptStarted
    /// 3. Stream all build events + ComponentStreamFinished
    /// 4. InvocationAttemptFinished
    /// 5. BuildFinished
    pub async fn publish(
        &mut self,
        events: Vec<build_event_stream::BuildEvent>,
        success: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let now = current_timestamp();

        // 1. BuildEnqueued
        self.publish_lifecycle_event(
            1,
            BuildComponent::Controller,
            build_event::Event::BuildEnqueued(build_event::BuildEnqueued { details: None }),
            &now,
        )
        .await?;

        // 2. InvocationAttemptStarted
        self.publish_lifecycle_event(
            1,
            BuildComponent::Controller,
            build_event::Event::InvocationAttemptStarted(build_event::InvocationAttemptStarted {
                attempt_number: 1,
                details: None,
            }),
            &now,
        )
        .await?;

        // 3. Stream build tool events
        self.publish_build_tool_event_stream(events).await?;

        // 4. InvocationAttemptFinished
        let result = if success {
            build_status::Result::CommandSucceeded
        } else {
            build_status::Result::CommandFailed
        };
        self.publish_lifecycle_event(
            2,
            BuildComponent::Controller,
            build_event::Event::InvocationAttemptFinished(build_event::InvocationAttemptFinished {
                invocation_status: Some(BuildStatus {
                    result: result as i32,
                    final_invocation_id: self.invocation_id.to_string(),
                    build_tool_exit_code: None,
                    error_message: String::new(),
                    details: None,
                }),
                details: None,
            }),
            &now,
        )
        .await?;

        // 5. BuildFinished
        self.publish_lifecycle_event(
            3,
            BuildComponent::Controller,
            build_event::Event::BuildFinished(build_event::BuildFinished {
                status: Some(BuildStatus {
                    result: result as i32,
                    final_invocation_id: self.invocation_id.to_string(),
                    build_tool_exit_code: None,
                    error_message: String::new(),
                    details: None,
                }),
                details: None,
            }),
            &now,
        )
        .await?;

        Ok(())
    }

    async fn publish_lifecycle_event(
        &mut self,
        sequence_number: i64,
        component: BuildComponent,
        event: build_event::Event,
        timestamp: &Timestamp,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let request = PublishLifecycleEventRequest {
            service_level: ServiceLevel::Interactive as i32,
            build_event: Some(OrderedBuildEvent {
                stream_id: Some(StreamId {
                    build_id: self.build_id.to_string(),
                    invocation_id: self.invocation_id.to_string(),
                    component: component as i32,
                }),
                sequence_number,
                event: Some(BuildEvent {
                    event_time: Some(timestamp.clone()),
                    event: Some(event),
                }),
            }),
            stream_timeout: None,
            notification_keywords: vec![],
            project_id: self.project_id.to_string(),
            check_preceding_lifecycle_events_present: false,
        };

        self.client.publish_lifecycle_event(request).await?;
        Ok(())
    }

    async fn publish_build_tool_event_stream(
        &mut self,
        events: Vec<build_event_stream::BuildEvent>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let stream_id = StreamId {
            build_id: self.build_id.to_string(),
            invocation_id: self.invocation_id.to_string(),
            component: BuildComponent::Tool as i32,
        };

        let mut requests = Vec::with_capacity(events.len() + 1);
        let now = current_timestamp();

        // Wrap each BES BuildEvent in an Any and assign sequence numbers
        for (i, bes_event) in events.into_iter().enumerate() {
            let any = Any {
                type_url: "type.googleapis.com/build_event_stream.BuildEvent".to_string(),
                value: bes_event.encode_to_vec(),
            };

            requests.push(PublishBuildToolEventStreamRequest {
                ordered_build_event: Some(OrderedBuildEvent {
                    stream_id: Some(stream_id.clone()),
                    sequence_number: (i + 1) as i64,
                    event: Some(BuildEvent {
                        event_time: Some(now.clone()),
                        event: Some(build_event::Event::BazelEvent(any)),
                    }),
                }),
                notification_keywords: vec![],
                project_id: self.project_id.to_string(),
                check_preceding_lifecycle_events_present: false,
            });
        }

        // Final event: ComponentStreamFinished
        let final_seq = requests.len() as i64 + 1;
        requests.push(PublishBuildToolEventStreamRequest {
            ordered_build_event: Some(OrderedBuildEvent {
                stream_id: Some(stream_id.clone()),
                sequence_number: final_seq,
                event: Some(BuildEvent {
                    event_time: Some(now),
                    event: Some(build_event::Event::ComponentStreamFinished(
                        build_event::BuildComponentStreamFinished {
                            r#type:
                                build_event::build_component_stream_finished::FinishType::Finished
                                    as i32,
                        },
                    )),
                }),
            }),
            notification_keywords: vec![],
            project_id: self.project_id.to_string(),
            check_preceding_lifecycle_events_present: false,
        });

        // Send as a client stream, consume ACK responses
        let response = self
            .client
            .publish_build_tool_event_stream(tokio_stream::iter(requests))
            .await?;

        let mut ack_stream = response.into_inner();
        while let Some(_ack) = ack_stream.message().await? {}

        Ok(())
    }
}

fn current_timestamp() -> Timestamp {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    Timestamp {
        seconds: now.as_secs() as i64,
        nanos: now.subsec_nanos() as i32,
    }
}
