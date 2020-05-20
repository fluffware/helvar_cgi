use std::collections::BTreeMap;
use bytes::BytesMut;

#[derive(Debug)]
pub struct Request
{
    pub params: BTreeMap<String,String>,
    pub stdin: Option<BytesMut>,
    pub input_left: usize,
    pub request_done: bool
}

#[async_trait]
pub trait RequestHandler: Send
{
    async fn handle(&mut self, req: &Request) -> Result<String, Box<dyn std::error::Error + Send>>;
}
