pub mod client;
pub mod translate;

#[cfg(test)]
mod tests {
    use super::*;
    use buck2bes_proto::buck::daemon::CommandProgress;
    use buck2bes_proto::buck::daemon::command_progress::Progress;
    use buck2bes_proto::buck::data::Invocation;
    use prost::Message;
    use prost::bytes::Bytes;

    #[tokio::test]
    async fn translate() {
        let compressed = include_bytes!("./testlog.pb.zst");
        let decompressed = zstd::stream::decode_all(&compressed[..]).unwrap();
        let mut buf = Bytes::from_owner(decompressed);

        let header = Invocation::decode_length_delimited(&mut buf).unwrap();

        let mut events = vec![];
        let mut translator = crate::translate::BuckEventTranslator::new();

        while let Ok(CommandProgress {
            progress: Some(Progress::Event(buck_event)),
        }) = CommandProgress::decode_length_delimited(&mut buf)
        {
            let mut bes_events = translator.translate(&buck_event);
            events.append(&mut bes_events);
        }

        // let api_key = std::env::var("BUILDBUDDY_API_KEY").unwrap();
        let mut client = crate::client::BesClient::connect(
            "https://remote.buildbuddy.io",
            "8MWg2RSiImiL2Bks9gCn",
            "0",
            "0",
            "k23",
        )
        .await
        .unwrap();

        client.publish(events, true).await.unwrap();

        // panic!("{header:?} {events:?}");
    }
}
