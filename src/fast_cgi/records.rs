use bytes::{BytesMut,Bytes,Buf,BufMut};
use std::vec::Vec;
use super::defs;
use super::decode;
use super::encode;
use std::fmt;

pub struct Error
{
    pub description: String
}

impl Error {
    pub fn new(description: &str) -> Error
    {
        Error{description: description.to_string()}
    }
}

impl std::error::Error for Error
{
}

impl fmt::Display for Error
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        f.write_str(&self.description)
    }
}

impl fmt::Debug for Error
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        write!(f,"Record error: {}", self.description)
    }
}

#[derive(Debug)]
pub struct Record
{
    pub version: u8,
    pub rec_type: u8,
    pub request_id: u16,
    pub content_data: BytesMut,
}

#[derive(Debug)]
pub struct BeginRequest
{
    pub role: u16,
    pub flags: u8
}

#[derive(Debug)]
pub struct EndRequest
{
    pub app_status: u32,
    pub protocol_status: u8
}


#[derive(Debug)]
pub struct NameValuePair
{
    pub name: String,
    pub value: String
}

impl NameValuePair {
    pub fn new(name: String, value: String) -> NameValuePair
    {
        NameValuePair{name, value}
    }
}

#[derive(Debug)]
pub enum ServerRecord
{
    GetValues(Vec<NameValuePair>),
    BeginRequest(BeginRequest),
    Params(Vec<NameValuePair>),
    StdIn(Bytes),
    Data(Bytes),
    Abort,
}

impl ServerRecord {
    pub fn decode(rec: &Record) -> Result<ServerRecord,Error>
    {
        match rec.rec_type {
            defs::FCGI_BEGIN_REQUEST => {
                let mut block = rec.content_data.clone();
                let role = block.get_u16();
                let flags = block.get_u8();
                Ok(ServerRecord::BeginRequest(BeginRequest{role, flags}))
            },
            defs::FCGI_ABORT_REQUEST => {
              Ok(ServerRecord::Abort)  
            },
            defs::FCGI_PARAMS => {
                let mut block: Bytes = rec.content_data.clone().into();
                let mut params = Vec::new();
                while block.len() >= 2 {
                    let (name,value,rest) = decode::decode_name_value_pair(block);
                    let name_str = String::from_utf8(name.to_vec()).unwrap();
                    let value_str = String::from_utf8(value.to_vec()).unwrap();
                    params.push(NameValuePair::new(name_str, value_str));
                        
                    block = rest;
                }
                Ok(ServerRecord::Params(params))
            },
            defs::FCGI_STDIN => {
                Ok(ServerRecord::StdIn(rec.content_data.clone().into()))
            },
            defs::FCGI_DATA => {
                Ok(ServerRecord::Data(rec.content_data.clone().into()))
            },
            defs::FCGI_GET_VALUES => {
                let mut block: Bytes = rec.content_data.clone().into();
                let mut params = Vec::new();
                while block.len() >= 2 {
                    let (name,value,rest) = decode::decode_name_value_pair(block);
                    let name_str = String::from_utf8(name.to_vec()).unwrap();
                    let value_str = String::from_utf8(value.to_vec()).unwrap();
                    params.push(NameValuePair::new(name_str, value_str));
                    block = rest;
                }
                Ok(ServerRecord::GetValues(params))
            },
            
            _ => Err(Error::new("Unrecognized record type"))
        }
            
    }
}

pub enum AppRecord {
    GetValuesResult(Vec<NameValuePair>),
    UnknownType(u8),
    EndRequest(EndRequest),
    StdOut(Bytes),
    StdErr(Bytes),
}

impl AppRecord {
    pub fn encode(self: &AppRecord, request_id: u16) -> Result<Record,Error>
    {
        let mut rec = Record{request_id: request_id,
                         version: defs::FCGI_VERSION_1,
                         rec_type: 0,
                         content_data: BytesMut::new()};
        match self {
            AppRecord::EndRequest(end) => {
                rec.rec_type = defs::FCGI_END_REQUEST;
                rec.content_data.put_u32(end.app_status);
                rec.content_data.put_u8(end.protocol_status);
                rec.content_data.put_slice(&[0u8;3]);
            },
            AppRecord::StdOut(data) => {
                rec.rec_type = defs::FCGI_STDOUT;
                rec.content_data.put(&mut data.clone());
            },
            AppRecord::StdErr(data) => {
                rec.rec_type = defs::FCGI_STDERR;
                rec.content_data.put(&mut data.clone());
            },
            AppRecord::GetValuesResult(values) => {
                rec.rec_type = defs::FCGI_GET_VALUES_RESULT;
                let buf = &mut rec.content_data;
                for p in values {
                    encode::encode_name_value_pair(buf, 
                                                   p.name.as_bytes(),
                                                   p.name.as_bytes());
                }
            },
             AppRecord::UnknownType(t) => {
                rec.rec_type = defs::FCGI_UNKNOWN_TYPE;
                 rec.content_data.put_u8(*t);
                 rec.content_data.put_slice(&[0u8;7]);
            },
        }
        Ok(rec)
    }
}
