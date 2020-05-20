use std::marker::Unpin;
use tokio::io::AsyncWrite;
use tokio::io::AsyncWriteExt;
use super::records::Record;
use bytes::BytesMut;
use bytes::BufMut;
use std::convert::TryFrom;
use tokio::io::Error;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct RecordOutput<O>
    where O: AsyncWrite + Send + Unpin
{
    output: Arc<Mutex<O>>
}

impl<O> RecordOutput<O>
    where O: AsyncWrite + Send + Unpin
{
    pub fn new(output:Arc<Mutex<O>>) -> RecordOutput<O>
    {
        RecordOutput{output}
    }

    pub async fn write(&mut self, rec: &Record) -> Result<(), Error> {
        const PADDING: [u8;7] = [0u8;7];
        let content_len = rec.content_data.len();
        let padding_len = (content_len.wrapping_neg() & 7) as u8;
        let mut header = BytesMut::new();
        header.put_u8(rec.version);
        header.put_u8(rec.rec_type);
        header.put_u16(rec.request_id);
        header.put_u16(u16::try_from(content_len).unwrap());
        header.put_u8(padding_len);
        header.put_u8(0);

        let mut output = self.output.lock().await;
        output.write(&header).await?;
        output.write(&rec.content_data).await?;
        output.write(&PADDING[..usize::from(padding_len)]).await?;

        Ok(())
    }
}

#[cfg(test)]
use tokio::runtime::Runtime;


#[test]
fn test_output_stream()
{
    let mut rt = Runtime::new().unwrap();
    rt.block_on(async {
        let buffer = Arc::new(Mutex::new(Vec::new()));
        
        let mut output = RecordOutput::new(buffer.clone());
        let content_data = BytesMut::from([9u8,7,8].as_ref());
        output.write(&Record{version: 1,
                             rec_type: 3,
                             request_id: 0x1733,
                             content_data}).await.unwrap();

        let res = buffer.lock().await;
        assert_eq!(*res,
                   BytesMut::from([1u8, 3, 0x17, 0x33, 0,3, 5, 0,
                                   9,7,8, 0,0,0,0,0].as_ref()));
    });
}
