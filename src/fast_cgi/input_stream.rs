use bytes::BytesMut;
use core::task::{Context, Poll};
use core::pin::Pin;
use std::marker::Unpin;
use tokio::io::AsyncRead;
use bytes::Buf;
use super::records::Record;
use tokio::stream::{Stream};
use std::sync::Arc;
use tokio::sync::{Mutex,OwnedMutexGuard};
use std::ops::DerefMut;
use std::future::Future;

pub struct RecordInputStream<I>
    where I: AsyncRead + Unpin + Send
{
    input: Arc<Mutex<I>>,
    buffer: BytesMut,
    record: Option<Record>,
    content_left: usize,
    padding_left: usize,
    // The future returned by lock_owned() must be saved so the task
    // will be notified when locking succeeds
    lock_future: Option<Pin<Box<dyn Future<Output = OwnedMutexGuard<I>> + Send>>>
}

impl<I> RecordInputStream<I>
    where I: AsyncRead + Unpin + Send
{
    pub fn new(input: Arc<Mutex<I>>) -> RecordInputStream<I>
    {
        RecordInputStream{input,
                          buffer: BytesMut::new(),
                          record: None,
                          content_left: 0,
                          padding_left: 0,
                          lock_future: None
        }
    }
}

impl<I> Stream for RecordInputStream<I>
    where I: AsyncRead + Unpin + Send + 'static
{
    type Item = Record;
    
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context)
        -> Poll<Option<Self::Item>>
    {
        let mutable = &mut self.get_mut();
        loop {
            //println!("Buf: {:?}", mutable.buffer);
            if mutable.buffer.len() > 0 {
                if let Some(record) = &mut mutable.record {
                    let copy =  mutable.content_left.min(mutable.buffer.len());
                    record.content_data.extend_from_slice(
                        &mutable.buffer.split_to(copy));
                    mutable.content_left -= copy;
                    if mutable.content_left == 0 {
                        return Poll::Ready(mutable.record.take())
                    }
                    continue;
                } else if mutable.padding_left > 0 {
                    let split = mutable.padding_left.min(mutable.buffer.len());
                    mutable.buffer.advance(split);
                    mutable.padding_left -= split;
                    continue;
                } else if  mutable.buffer.len() >= 8 {
                    let mut header = mutable.buffer.split_to(8);
                    let ver = header.get_u8();
                    let rec_type = header.get_u8();
                    let request_id = header.get_u16();
                    mutable.content_left = header.get_u16().into();
                    mutable.padding_left = header.get_u8().into();
                    mutable.record = Some(Record{version:ver,
                                                 rec_type: rec_type,
                                                 request_id: request_id,
                                                 content_data: BytesMut::new()
                    });
                    continue;
                }
            }
            if mutable.lock_future.is_none() {
                mutable.lock_future =
                    Some(Box::pin(mutable.input.clone().lock_owned()));
            }
            let mut guard = 
                match mutable.lock_future.as_mut().unwrap().as_mut().poll(cx) 
            {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(inp) => inp
            };
            mutable.lock_future = None;
            let pinned = Pin::new(guard.deref_mut());
            match pinned.poll_read_buf(cx, &mut mutable.buffer) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(Err(_)) => return Poll::Ready(None),
                Poll::Ready(Ok(0)) => return Poll::Ready(None),
                    Poll::Ready(Ok(_)) => {
                    }
            }
        }
    }
}

#[cfg(test)]
use tokio::runtime::Runtime;
#[cfg(test)]
use bytes::Bytes;
#[cfg(test)]
use tokio::stream::{self,StreamExt};

#[test]
fn test_input_stream()
{
    let mut rt = Runtime::new().unwrap();
    
    rt.block_on(async {
        let blocks = vec![
            Ok(Bytes::from_static(&[1u8,0x02, 0x00,0x03, 0x00])),
            Ok(Bytes::from_static(&[0x05, 0x01, 0x00,
                                    0x01,0x02])),
            Ok(Bytes::from_static(&[0x03,0x04,0x05,
                                    0x00])),
            Ok(Bytes::from_static(&[1u8,0x02, 0x00,0x03, 0x00, 0x05, 0x01, 0x00,
                                    0x01,0x02,0x03,0x04,0x05,
                                    0x00])),
            ];
        let stream = stream::iter(blocks);
        let mut src = tokio::io::stream_reader(stream);
        
        let framer = RecordInputStream::new(Arc::new(Mutex::new(Box::new(src))));
        let records : Vec<Record> = framer.collect().await;
        println!("{:?}", records);
    });
}

#[test]
fn test_input_stream_blocked()
{
      let mut rt = Runtime::new().unwrap();
    
    rt.block_on(async {
        let blocks = vec![
            Ok(Bytes::from_static(&[1u8,0x02, 0x00,0x03, 0x00])),
            Ok(Bytes::from_static(&[0x05, 0x01, 0x00,
                                    0x01,0x02])),
            Ok(Bytes::from_static(&[0x03,0x04,0x05,
                                    0x00])),
            Ok(Bytes::from_static(&[1u8,0x02, 0x00,0x03, 0x00, 0x05, 0x01, 0x00,
                                    0x01,0x02,0x03,0x04,0x05,
                                    0x00])),
        ];
        let stream = stream::iter(blocks);
        let mut src = tokio::io::stream_reader(stream);
        let arc_src = Arc::new(Mutex::new(src));
        let task;
        let local_src = arc_src.clone();
        {
            let locked_src = local_src.lock().await;
            task = tokio::spawn(async move {
                let framer = RecordInputStream::new(arc_src);
                let records : Vec<Record> = framer.collect().await;
                println!("{:?}", records);
            });
            println!("Delay starting");
            tokio::time::delay_for(std::time::Duration::from_secs(1)).await;
            println!("Delay done");
        }
        task.await;
        println!("Exiting");
    });
}
