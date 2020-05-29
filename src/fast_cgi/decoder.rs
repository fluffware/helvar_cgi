use std::collections::HashMap;
use std::collections::BTreeMap;
use tokio::io::AsyncRead;
use tokio::io::AsyncWrite;
use tokio::stream::StreamExt;
use super::records::ServerRecord;
use super::records::AppRecord;
use super::records::EndRequest;
use super::defs;
use bytes::{Bytes,BytesMut,BufMut};
use super::request::{Request, RequestHandler};

use super::input_stream::RecordInputStream;
use super::record_output::RecordOutput;


pub struct Decoder
{
    requests: HashMap<u16, Request>
}


impl Decoder
{
    pub fn new() -> Decoder
    {
        Decoder{requests: HashMap::<u16,Request>::new()}
    }

    async fn write_error<O>(output: &mut RecordOutput<O>, msg: &str)
        where O: AsyncWrite + Unpin + Send + 'static,
    {
        let out = AppRecord::StdErr(
            Bytes::from(msg.to_string())
        );
        output.write(&out.encode(0).unwrap()).await.unwrap();
    }

    async fn error_reply<O>(output: &mut RecordOutput<O>, 
                      err: Box<dyn std::error::Error + Send>, 
                      request_id: u16)
        where O: AsyncWrite + Unpin + Send + 'static,
    {
        let out = AppRecord::StdOut({
            let reply = "Status: 500 Internal error\r\n\r\n";
            Bytes::from(reply)
        });
        output.write(&out.encode(request_id).unwrap()).await.unwrap();

        Self::write_error(output,&format!("App failed with error: {}",err)).await;

        let reply = AppRecord::EndRequest(
            EndRequest{
                app_status: 0,
                protocol_status: defs::FCGI_REQUEST_COMPLETE
            });
        output.write(&reply.encode(request_id).unwrap()).await.unwrap();
        
    }
    
    pub async fn run<I,O>(&mut self,
                     mut input_stream: RecordInputStream<I>,
                     mut output: RecordOutput<O>,
                     handler: &mut dyn RequestHandler
    ) where I: AsyncRead + Unpin + Send + 'static, 
            O: AsyncWrite + Unpin + Send + 'static
    {
        while let Some(rec) = input_stream.next().await {
            if rec.request_id == 0 {
                let unknown = AppRecord::UnknownType(rec.rec_type);
                output.write(&unknown.encode(rec.request_id).unwrap()).await.unwrap_or(());
            } else {
                match ServerRecord::decode(&rec) {
                    Ok(ServerRecord::BeginRequest(begin)) => {
                        //println!("Begin: {:?}", begin);
                        let params = BTreeMap::new();
                        self.requests.insert(rec.request_id, 
                                             Request{
                                                 params,
                                                 stdin: None,
                                                 input_left: 0,
                                                 request_done: false
                                             });
                    },
                    Ok(ServerRecord::Params(pairs)) => {
                        if let Some(request) = 
                            self.requests.get_mut(&rec.request_id) 
                        {
                            if pairs.is_empty() {
                                if let Some(len) = 
                                    request.params.get("CONTENT_LENGTH")
                                    .and_then(|s| { 
                                        usize::from_str_radix(s, 10)
                                            .map_or(None , |l| {
                                                if l > 0 {Some(l)} else {None}
                                            })
                                    }) {
                                    request.input_left = len;
                                    request.stdin = Some(BytesMut::new());
                                } else {
                                    request.request_done = true;
                                }
                            } else {
                                for p in pairs {
                                    request.params.insert(p.name, p.value);
                                }
                            }
                        }
                    },
                    Ok(ServerRecord::StdIn(data)) => {
                        if let Some(request) = 
                            self.requests.get_mut(&rec.request_id) 
                        {
                            let len = data.len();
                            if len > 0 {
                                if let Some(stdin) = &mut request.stdin {
                                    stdin.put(data);
                                }
                                if request.input_left > len {
                                    request.input_left -= len;
                                } else {
                                    request.input_left = 0;
                                    request.request_done = true;
                                }
                            } else {
                                request.request_done = true;
                            }
                        }   
                    },
                    Ok(ServerRecord::Abort) => {
                        let reply = AppRecord::EndRequest(
                            EndRequest{
                                app_status: 0,
                                protocol_status: defs::FCGI_REQUEST_COMPLETE
                            });
                        output.write(&reply.encode(rec.request_id).unwrap()).await.unwrap();
                    },
                    Ok(r) => {
                        println!("Other: {:?}", r);
                        let unknown = AppRecord::UnknownType(rec.rec_type);
                        output.write(&unknown.encode(rec.request_id).unwrap()).await.unwrap_or(());

                    }
                    Err(e) => {
                        println!("Failed to decode record: {}", e);
                    }
                    
                }
                match self.requests.get_mut(&rec.request_id) {
                    Some(req) if req.request_done =>
                    {
                        match handler.handle(req).await {
                            Ok(reply) => {
                                let out = AppRecord::StdOut(
                                    Bytes::from(reply)
                                );
                                output.write(&out.encode(rec.request_id).unwrap()).await.unwrap();
                                let reply = AppRecord::EndRequest(
                                    EndRequest{
                                        app_status: 0,
                                        protocol_status: defs::FCGI_REQUEST_COMPLETE
                                    });
                                output.write(&reply.encode(rec.request_id).unwrap()).await.unwrap_or(());
                                //println!("Request done");
                            },
                            Err(e) => {
                                Self::error_reply(&mut output, e, rec.request_id).await;
                            }
                        }
                    },
                    _ => {}
                }
            }
        }
        //println!("Connection closed");
    }
}

