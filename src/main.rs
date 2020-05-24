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
use std::convert::TryInto;
use std::str::FromStr;

extern crate helvar_cgi;
use helvar_cgi::fast_cgi::decoder::Decoder;

#[macro_use]
extern crate async_trait;

#[macro_use]
extern crate serde_json;
use serde_json as json;

pub mod helvar_error;
use helvar_error::HelvarError;
pub mod helvar_defs;
use helvar_defs as cmd;
pub mod dali_state;
use dali_state::RouterState;
use dali_state::SubnetState;
use dali_state::DeviceState;

pub mod helvar_device_type;
use helvar_device_type::HelvarDeviceType;

pub mod wrapper_error;
use wrapper_error::WrapperError;

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
    
    async fn command(&mut self, cmd_str: &str) -> Result<(),HelvarError>
    {
        let cmd_bytes = cmd_str.as_bytes();
        match self.stream.write(cmd_bytes).await {
            Ok(_) => Ok(()),
            Err(e) => Err(e.into())
        }
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

    async fn set_direct_level_device(&mut self, subnet: u8, dev: u8,
                                     level: u32, fade: u32) 
                                     -> Result<(),HelvarError>
    {
        let cmd_str = format!("?V:{},C:{},L:{},F:{},{}#",
                              self.helvarnet_version, 
                              cmd::CMD_DIRECT_LEVEL_DEVICE,
                              level, fade,
                              self.device_arg(subnet, dev));
        self.command(&cmd_str).await
    }
}

struct Handler
{
    router_state: RouterStateArc,
    router_control: RouterArc
}

struct HandlerError
{
    msg: String,
    src: Option<Box<dyn std::error::Error + Send + 'static>>
}

impl HandlerError
{
    fn new(msg: &str) -> HandlerError
    {
        HandlerError{msg: msg.to_string(),
                     src: None}
    }

    fn from_error<E>(err: E, msg: &str) -> HandlerError
        where E: std::error::Error + Send + 'static
    {
        HandlerError{msg: msg.to_string(),
                     src: Some(Box::new(err))}
    }
}

impl std::error::Error for HandlerError
{
}

impl fmt::Display for HandlerError
{
     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        if let Some(src) = &self.src {
            write!(f,"{}: {}", &self.msg, src)
        } else {
            f.write_str(&self.msg)
        }
    }
}

impl fmt::Debug for HandlerError
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        if let Some(src) = &self.src {
            write!(f,"HandlerError: {}: {}", &self.msg, src)
        } else {
             write!(f,"HandlerError: {}", self.msg)
        }
    }
}

fn device_to_json(dev: &DeviceState) -> json::Value
{
    json::json!({"description": dev.description,
                 "address": dev.address,
                 "level": dev.intensity})
}

fn subnet_to_json(sn: &SubnetState) -> json::Value
{
    let mut dev_map = json::map::Map::new();
    let mut dev_iter = (1..64).map(|x| &sn.devices[x-1])
        .filter_map(|x| x.as_ref())
        .peekable();
    while let Some(dev) = dev_iter.next() {
        dev_map.insert(dev.address.to_string(), 
                       device_to_json(dev));
    }
    
    json::json!({"devices": json!(dev_map),
                 "index": json!(sn.index)})
}


#[async_trait]
impl RequestHandler for Handler 
{
    async fn handle(&mut self, req: &Request) -> Result<String, Box<dyn std::error::Error + Send>>
    {
        let mut subnet_arg = None::<u32>;
        let mut address_arg = None::<u32>;
        if let Some(path) = req.params.get("PATH_INFO") {
            let mut parts = path.split("/");
            let subnet_str =
                parts.next()
                .and_then(|p| if p.is_empty() {None} else {Some(p)})
                .or_else(|| parts.next());
            if let Some(subnet_str) = subnet_str {
                subnet_arg = match u32::from_str(subnet_str)
                {
                    Ok(i) => Some(i),
                    Err(e) => return Err(Box::new(HandlerError::from_error(
                        e, "Failed to parse subnet index")))
                };
                let address_str = parts.next();
                if let Some(address_str) = address_str {
                    address_arg = match u32::from_str(address_str)
                    {
                        Ok(i) => Some(i),
                        Err(e) => return Err(Box::new(HandlerError::from_error(
                            e,"Failed to parse address")))
                    };
                }
            }
                    
        }
        println!("{:?}.{:?}", subnet_arg, address_arg);

        if let Some(query_str) = req.params.get("QUERY_STRING") {
            if query_str.starts_with("level=") {
                match u8::from_str(&query_str[6..]) {
                    Ok(level) => {
                        if let (Some(sn_index), Some(addr)) =
                            (subnet_arg, address_arg)
                        {
                            println!("Set level {}.{}: {}",sn_index, addr, level);
                            let mut router = self.router_control.lock().await;
                            match router.set_direct_level_device(
                                sn_index.try_into().unwrap(),
                                addr.try_into().unwrap(),
                                level.into(), 70).await {
                                Ok(_) => {},
                                Err(e) => {
                                    return Err(Box::new(
                                        HandlerError::from_error(
                                            e,"Failed to set device level")))
                                }
                            }
                            let mut rs = self.router_state.lock().unwrap();
                            if let Some(dev) = rs.get_device_mut(sn_index, addr) {
                                dev.intensity = level;
                            }

                        }
                    }
                    Err(e) => return Err(Box::new(
                        HandlerError::from_error(
                            e,"Failed to parse level")))
                }
            }
        }
        let rs = self.router_state.lock().unwrap();
        let mut reply = "Content-type: application/json\r\n\r\n".to_string();

        let top_obj = match (subnet_arg, address_arg) {
            (Some(subnet), Some(addr)) => {
                if let Some(dev) = rs.get_device(subnet, addr) {
                    device_to_json(dev)
                } else {
                    json::Value::Null
                }
            },
            (Some(subnet), None) => {
                if let Some(sn) = rs.get_subnet(subnet) {
                    subnet_to_json(&sn)
                } else {
                    json::Value::Null
                } 
            },
            (None, _) => {
                let mut subnet_map = serde_json::map::Map::new();
                
                let mut sn_iter = 
                    rs.subnets.iter()
                    .filter_map(|x| x.as_ref())
                    .peekable();
                while let Some(sn) = sn_iter.next() {
                    
            
                    subnet_map.insert(sn.index.to_string(), 
                                      subnet_to_json(sn));
                    
                }
                json!({"subnets": json!(subnet_map)})
            }
        };
        reply += &serde_json::to_string_pretty(&top_obj).unwrap();
        Ok(reply)
    }
}

type RouterStateArc = Arc<StdMutex<RouterState>>;
type RouterArc = Arc<Mutex<Router>>;

async fn connection_handler<S>(stream: Arc<Mutex<Box<S>>>, 
                               router_state: RouterStateArc,
                               router_control: RouterArc)
    where S: AsyncRead+AsyncWrite+Unpin+Send+'static
{
    let rec_stream = RecordInputStream::new(stream.clone());
    let rec_output = RecordOutput::new(stream);
    let mut decoder = Decoder::new();
    decoder.run(rec_stream,rec_output, 
                &mut Handler{router_state, router_control}).await;
}

async fn query_device(router: &mut Router, router_state: &RouterStateArc,
                      subnet: u8, addr: u8, priority: u32)
                      -> Result<(), Box<dyn std::error::Error>>
{
    let mut dev = {
        let rs = router_state.lock().unwrap();
        rs.subnets.get(usize::from(subnet) -1).and_then({
            |sn| match sn {
                Some(sn) => sn.devices[usize::from(addr)-1].clone(),
                None => None
            }
        }).unwrap_or_else(|| Box::new(DeviceState::new()))
    };
    dev.address = u32::from(addr);
    'done: loop {
        match router.query_device_type(subnet,addr).await {
            Ok(dtype) => {
                dev.device_type = dtype;
            },
            Err(HelvarError::NoSuchDevice) => return Ok(()),
            Err(e) => {
                return Err(WrapperError::new("Failed to query device type",
                                             Box::new(e)).into());
            }
        }
        if HelvarDeviceType::from(dev.device_type).is_load() { 
            match router.query_load_level(subnet,addr).await {
                Ok(level) => {
                    dev.intensity = u8::try_from(level).unwrap_or(0xff);
                },
                Err(HelvarError::NoSuchDevice) => return Ok(()),
                Err(e) => {
                    return Err(WrapperError::new("Failed to query load level",
                                                 Box::new(e)).into());
                }
            }
        }
        if priority > 1 {break 'done}
        match router.query_device_description(subnet,addr).await {
            Ok(descr) => {
                dev.description = descr;
            },
            Err(e) => {
                return Err(WrapperError::new("Failed to query device description",
                                             Box::new(e)).into());
            }
        }
        break
    }
    
    let mut rs = router_state.lock().unwrap();
    while rs.subnets.len() <= subnet.into() {
        rs.subnets.push(None);
    }
    let sn = rs.subnets[usize::from(subnet) -1]
        .get_or_insert_with(
            || Box::new(SubnetState::new(u32::from(subnet))));
    sn.devices[usize::from(addr)-1] = Some(dev);
    
    Ok(())
}

async fn fcgi_task(router: RouterArc, router_state:RouterStateArc)
{
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
                tokio::spawn(connection_handler(io,
                                                router_state.clone(),
                                                router.clone()));
            },
            Err(e) => {
                println!("Error: {:?}", e);
            }
        }
    }
    println!("Stopped");
}

async fn router_poll_task(router: RouterArc, router_state:RouterStateArc)
{
      
        for subnet in 1..=2 {
            for a in 1..=64 {
                let mut router = router.lock().await;
                match query_device(&mut router, &router_state,
                                   subnet, a, 0).await
                {
                    Ok(()) => {},
                    Err(e) => eprintln!("Initial query failed: {}",e)
                }
            }
        }
        {
            let rs = router_state.lock();
            println!("{:?}", rs);
        }
        loop {
            for subnet in 1..=2 {
                for a in 1..=64 {
                    {
                        let mut router = router.lock().await;
                        match query_device(&mut router, &router_state,
                                           subnet, a, 0).await
                        {
                            Ok(()) => {},
                            Err(e) => eprintln!("Update query failed: {}",e)
                        }
                    }
                    tokio::time::delay_for(Duration::from_secs(1)).await;
                println!("Updating {}",a);
                }
            }
        }
}

#[tokio::main]
async fn main() {

    let router_state = Arc::new(StdMutex::new(RouterState::new()));
    let router = Router::connect(&[172,16,120,98]).await.unwrap();
    let router = Arc::new(tokio::sync::Mutex::new(router));
    
    let fcgi = tokio::spawn(fcgi_task(router.clone(),
                                      router_state.clone()));
    
    let helvar = tokio::spawn(router_poll_task(router.clone(),
                                               router_state.clone()));
                              
    fcgi.await.unwrap();
    helvar.await.unwrap();
}
