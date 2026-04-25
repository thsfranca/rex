use std::time::Instant;

use async_stream::stream;
use rex_proto::rex::v1::rex_service_server::RexService;
use rex_proto::rex::v1::{
    GetSystemStatusRequest, GetSystemStatusResponse, StreamInferenceRequest,
    StreamInferenceResponse,
};
use tokio::time::{sleep, Duration};
use tokio_stream::Stream;
use tonic::{Request, Response, Status};

use crate::domain::{build_mock_output, chunk_output, ACTIVE_MODEL_ID, DAEMON_VERSION};

pub struct RexDaemonService {
    started_at: Instant,
}

const STREAM_CHUNK_MAX_CHARS: usize = 8;
const STREAM_CHUNK_DELAY_MS: u64 = 35;

impl RexDaemonService {
    pub fn new(started_at: Instant) -> Self {
        Self { started_at }
    }

    pub fn build_inference_chunks(prompt: &str) -> Vec<Result<StreamInferenceResponse, Status>> {
        let text = build_mock_output(prompt);
        let mut chunks = Vec::new();
        let content_chunks = chunk_output(&text, STREAM_CHUNK_MAX_CHARS);

        for (index, chunk) in content_chunks.iter().enumerate() {
            chunks.push(Ok(StreamInferenceResponse {
                text: chunk.clone(),
                index: index as u64,
                done: false,
            }));
        }

        chunks.push(Ok(StreamInferenceResponse {
            text: String::new(),
            index: content_chunks.len() as u64,
            done: true,
        }));
        chunks
    }
}

#[tonic::async_trait]
impl RexService for RexDaemonService {
    async fn get_system_status(
        &self,
        _request: Request<GetSystemStatusRequest>,
    ) -> Result<Response<GetSystemStatusResponse>, Status> {
        Ok(Response::new(GetSystemStatusResponse {
            daemon_version: DAEMON_VERSION.to_string(),
            uptime_seconds: self.started_at.elapsed().as_secs(),
            active_model_id: ACTIVE_MODEL_ID.to_string(),
        }))
    }

    type StreamInferenceStream =
        std::pin::Pin<Box<dyn Stream<Item = Result<StreamInferenceResponse, Status>> + Send>>;

    async fn stream_inference(
        &self,
        request: Request<StreamInferenceRequest>,
    ) -> Result<Response<Self::StreamInferenceStream>, Status> {
        let prompt = request.into_inner().prompt;
        let chunks = Self::build_inference_chunks(&prompt);
        let output = stream! {
            for chunk in chunks {
                yield chunk;
                sleep(Duration::from_millis(STREAM_CHUNK_DELAY_MS)).await;
            }
        };

        Ok(Response::new(Box::pin(output)))
    }
}

#[cfg(test)]
mod tests {
    use super::RexDaemonService;

    #[test]
    fn stream_chunks_end_with_done_marker() {
        let chunks = RexDaemonService::build_inference_chunks("ping");
        assert!(chunks.len() >= 3);

        let first = chunks[0].as_ref().expect("first chunk should be ok");
        assert!(!first.done);
        assert!(!first.text.is_empty());

        for (index, chunk) in chunks.iter().enumerate() {
            let chunk = chunk.as_ref().expect("chunk should be ok");
            assert_eq!(chunk.index, index as u64);
            if index < chunks.len() - 1 {
                assert!(!chunk.done);
                assert!(!chunk.text.is_empty());
            }
        }

        let last = chunks[chunks.len() - 1]
            .as_ref()
            .expect("last chunk should be ok");
        assert!(last.done);
        assert_eq!(last.text, "");
    }
}
