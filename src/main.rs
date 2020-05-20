use std::net::{IpAddr, SocketAddr};
use tokio::net::TcpStream;
use tokio;
use tokio::prelude::*;
use std::time::Duration;
use std::os::unix::io::FromRawFd;
use tokio::stream::StreamExt;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::sync::Mutex as StdMutex;
use std::fmt;
use std::convert::TryFrom;

extern crate helvar_cgi;
use helvar_cgi::fast_cgi::decoder::Decoder;

#[macro_use]
extern crate async_trait;

pub mod helvar_error;
use helvar_error::HelvarError;
pub mod helvar_defs;
use helvar_defs as cmd;
pub mod dali_state;
use dali_state::RouterState;
use dali_state::SubnetState;
use dali_state::DeviceState;

use helvar_cgi::fast_cgi as fcgi;
use fcgi::input_stream::RecordInputStream;
use fcgi::record_output::RecordOutput;

use fcgi::request::{Request,RequestHandler};
    
struct Router {
    addr: [u8;4],
    stream: TcpStream,
    helvarnet_version: u32
}

impl Router {
    async fn connect(addr: &[u8;4]) -> Result<Router,HelvarError>
    {
        let ip_addr = 
            IpAddr::V4((*addr).into());
        let socket = SocketAddr::new(ip_addr,50000);
        let stream = TcpStream::connect(socket).await?;

        let router = Router{addr: addr.clone(), stream, helvarnet_version: 3};
        Ok(router)
    }

    fn device_arg(&self, subnet: u8, dev: u8) -> String
    {
        format!("@{}.{}.{}.{}.{}.{}",
                self.addr[0],self.addr[1],self.addr[2],self.addr[3], 
                subnet,dev)
    }

    async fn query(&mut self, cmd_str: &str) -> Result<String,HelvarError>
    {
        let cmd_bytes = cmd_str.as_bytes();
        self.stream.write(cmd_bytes).await?;

        let mut buf = [0u8; 256];
        let mut line = Vec::<u8>::new();
        loop {
            let read =  self.stream.read(&mut buf);
            let timeout = tokio::time::timeout(Duration::from_secs(5),
                                               read);
            let n = match timeout.await {
                Err(_) => return Err(HelvarError::Timeout),
                Ok(res) => {
                    match res {
                        Err(e) => 
                            return Err(HelvarError::from_error(Box::new(e))),
                        Ok(n) => n
                    }
                }

            };
            let mut recv:&[u8] = &buf[0..n];
            while !recv.is_empty() {
                if line.len() == 0 {
                    let start = recv.iter()
                        .position(|&b| b == b'?' || b == b'!')
                        .unwrap_or(recv.len());
                    recv = &recv[start..];
                }
                let end_found = recv.iter().position(|&b| b == b'#');
                let end = end_found.map_or(recv.len(), |x| x+1);
                line.extend_from_slice(&recv[..end]);
                recv = &recv[end..];
                if end_found.is_some() {
                    let cmd_end = cmd_bytes.len() - 1; 
                    if cmd_end < line.len() 
                        && line[1..cmd_end] == cmd_bytes[1..cmd_end]
                        && line[cmd_end] == b'='
                    {
                        let reply =
                            std::str::from_utf8(&line[cmd_end+1..line.len()-1])?;
                        if line[0] == b'?' {
                            // println!("{} {}",n, std::str::from_utf8(&line).unwrap());
                            return Ok(reply.to_string());
                        } else if line[0] == b'!' {
                            let err_code = reply.parse::<u32>()?;
                            return Err(HelvarError::from_code(err_code));
                        }
                    }
                    line.clear();
                }
//                println!("recv: {:?}", recv);
            }
        }
        
    }
    
    async fn query_device_type(&mut self, subnet: u8, dev: u8) -> Result<u32,HelvarError>
    {
        let cmd_str = format!("?V:{},C:{},{}#", self.helvarnet_version, 
                              cmd::CMD_QUERY_DEVICE_TYPE,
                              self.device_arg(subnet, dev));
        let reply = self.query(&cmd_str).await?;
        let t = match reply.parse::<u32>() {
            Ok(v) => v,
            Err(e) => return Err(HelvarError::Other(Box::new(e)))
        };
        Ok(t)
    }

      async fn query_device_description(&mut self, subnet: u8, dev: u8) -> Result<String,HelvarError>
    {
        let cmd_str = format!("?V:{},C:{},{}#", self.helvarnet_version, 
                              cmd::CMD_QUERY_DESCRIPTION_DEVICE,
                              self.device_arg(subnet, dev));
        self.query(&cmd_str).await
    }
      async fn query_load_level(&mut self, subnet: u8, dev: u8) -> Result<u32,HelvarError>
    {
        let cmd_str = format!("?V:{},C:{},{}#", self.helvarnet_version, 
                              cmd::CMD_QUERY_LOAD_LEVEL,
                              self.device_arg(subnet, dev));
        let reply = self.query(&cmd_str).await?;
        let t = match reply.parse::<u32>() {
            Ok(v) => v,
            Err(e) => return Err(HelvarError::Other(Box::new(e)))
        };
        Ok(t)
    }
   
}

struct Handler
{
    count: u32
}

struct HandlerError
{
    msg: String
}

impl HandlerError
{
    fn new(msg: &str) -> HandlerError
    {
        HandlerError{msg: msg.to_string()}
    }
}

impl std::error::Error for HandlerError
{
}

impl fmt::Display for HandlerError
{
     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        f.write_str(&self.msg)
    }
}

impl fmt::Debug for HandlerError
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        write!(f,"HandlerError: {}", self.msg)
    }
}

#[async_trait]
impl RequestHandler for Handler 
{
    async fn handle(&mut self, req: &Request) -> Result<String, Box<dyn std::error::Error + Send>>
    {
        //return Err(Box::new(HandlerError::new("Failed")));
        println!("Rec: {:?}", req);
        self.count+= 1;
        let mut reply = "Content-type: text/html\r\n\r\n".to_string();
        reply += "<html><head><title>Response</title></head>\r\n";
        reply += "<body><h1>Response</h1>";
        reply += &format!("<p>Count: {}</p>",self.count);
        if let Some(stdin) = &req.stdin {
            reply += &format!("<p>Stdin: {}</p>",
                              String::from_utf8(stdin.to_vec()).unwrap());
        }
        reply += "</body><html>";
        Ok(reply)
    }
}

async fn connection_handler<S>(stream: Arc<Mutex<Box<S>>>)
    where S: AsyncRead+AsyncWrite+Unpin+Send+'static
{
    let rec_stream = RecordInputStream::new(stream.clone());
    let rec_output = RecordOutput::new(stream);
    let mut decoder = Decoder::new();
    decoder.run(rec_stream,rec_output, &mut Handler{count: 29}).await;
}

#[tokio::main]
async fn main() {

    let router_state = Arc::new(StdMutex::new(RouterState::new()));
    let fcgi = tokio::spawn(async {
        let std_listener = unsafe {
            std::os::unix::net::UnixListener::from_raw_fd(0)
        };
        let mut listener = 
            tokio::net::UnixListener::from_std(std_listener).unwrap();
        let mut incoming = listener.incoming();
        
        println!("Listening");
        while let Some(stream) = incoming.next().await {
            match stream {
                Ok(stream) => {
                
                    let io = Arc::new(Mutex::new(Box::new(stream)));
                tokio::spawn(connection_handler(io));
                },
                Err(e) => {
                    println!("Error: {:?}", e);
                }
            }
        }
        println!("Stopped");
    });

    let helvar = tokio::spawn(async move {
        let mut router = Router::connect(&[172,16,120,98]).await.unwrap();
        for subnet in 1..=2 {
            for a in 1..=64 {
                let mut dev = Box::new(DeviceState::new());
                dev.address = u32::from(a);
                match router.query_device_type(subnet,a).await {
                    Ok(dtype) => {
                        dev.device_type = dtype;
                    },
                    Err(e) => {
                        println!("{} Error: {}", a,e);
                        continue;
                    }
                }
                match router.query_device_description(subnet,a).await {
                    Ok(descr) => {
                        dev.description = descr;
                    },
                    Err(e) => {
                        println!("{} Error: {}", a,e);
                        continue
                    }
                }
                
                match router.query_load_level(subnet,a).await {
                    Ok(level) => {
                        dev.intensity = u8::try_from(level).unwrap_or(0xff);
                    },
                    Err(e) => {
                        println!("{} Error: {}", a,e);
                        continue
                    }
                }

                let mut rs = router_state.lock().unwrap();
                while rs.subnets.len() <= subnet.into() {
                    rs.subnets.push(None);
                }
                let sn = rs.subnets[usize::from(subnet) -1]
                    .get_or_insert_with(
                        || Box::new(SubnetState::new()));
                sn.devices[usize::from(a)-1] = Some(dev);
                
            }
        }
        {
            let rs = router_state.lock();
            println!("{:?}", rs);
        }
        /*    ;
    stream.write(b"?V:3,C:106,@172.16.120.98.1.12#").await.unwrap();
            let n = stream.read(&mut buf).await.unwrap();
            println!("{} {}",n, std::str::from_utf8(&buf[0..n]).unwrap());
         */
    });
    fcgi.await.unwrap();
    helvar.await.unwrap();
}
