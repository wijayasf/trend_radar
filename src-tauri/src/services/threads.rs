pub trait ThreadsClient {
    fn source_name(&self) -> &'static str;
}

pub struct PlaceholderThreadsClient;

impl ThreadsClient for PlaceholderThreadsClient {
    fn source_name(&self) -> &'static str {
        "threads-api-placeholder"
    }
}
