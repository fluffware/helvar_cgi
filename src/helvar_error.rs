use std::error::Error;
use std::fmt;
use std::convert::From;

#[derive(Debug)]
pub enum HelvarError
{
    Ok,
    InvalidGroupIndex,
    InvalidCluster,
    InvalidRouter,
    InvalidSubnet,
    InvalidDevice,
    InvalidSubdevice,
    InvalidBlock,
    InvalidScene,
    NoSuchCluster,
    NoSuchRouter,
    NoSuchDevice,
    NoSuchProperty,
    InvalidRawMessageSize,
    InvalidMessagesType,
    InvalidMessageCommand,
    Timeout,
    UnknownErrorCode(u32),
    Other(Box<dyn Error + Send + 'static>)
}

impl HelvarError
{
    pub fn from_code(code: u32) -> HelvarError
    {
        match code {
            0 => HelvarError::Ok,
            1 => HelvarError::InvalidGroupIndex,
            2 => HelvarError::InvalidCluster,
            3 => HelvarError::InvalidRouter,
            4 => HelvarError::InvalidSubnet,
            5 => HelvarError::InvalidDevice,
            6 => HelvarError::InvalidSubdevice,
            7 => HelvarError::InvalidBlock,
            8 => HelvarError::InvalidScene,
            9 => HelvarError::NoSuchCluster,
            10 => HelvarError::NoSuchRouter,
            11 => HelvarError::NoSuchDevice,
            12 => HelvarError::NoSuchProperty,
            13 => HelvarError::InvalidRawMessageSize,
            14 => HelvarError::InvalidMessagesType,
            15 => HelvarError::InvalidMessageCommand,
            _ => HelvarError::UnknownErrorCode(code)
        }
    }

    pub fn from_error(err: Box<dyn Error + Send + 'static>) -> HelvarError
    {
        HelvarError::Other(err)
    }
}

impl Error for HelvarError
{
    fn source(&self) -> Option<&(dyn Error + 'static)>
    {
        match self {
            HelvarError::Other(err) => Some(err.as_ref()),
            _ => None
        }
    }
            
}

/*
impl<E: Error + Send + 'static> From<E> for HelvarError
{
    fn from(error: E) ->HelvarError
        where E: Error + Send + 'static
    {
        HelvarError::from_error(Box::new(error))
    }
}
*/

impl From<std::io::Error> for HelvarError
{
    fn from(error: std::io::Error) -> HelvarError
    {
        HelvarError::from_error(Box::new(error))
    }
}

impl From<std::str::Utf8Error> for HelvarError
{
    fn from(error: std::str::Utf8Error) -> HelvarError
    {
        HelvarError::from_error(Box::new(error))
    }
}

impl From<std::num::ParseIntError> for HelvarError
{
    fn from(error: std::num::ParseIntError) -> HelvarError
    {
        HelvarError::from_error(Box::new(error))
    }
}

impl fmt::Display for HelvarError
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        if let HelvarError::UnknownErrorCode(code) = self {
            write!(f,"Unknown error code {}", code)
        } else if let HelvarError::Other(err) = self {
            err.fmt(f)
        } else {
            let msg = 
                match self {
                    HelvarError::Ok => "Success",
                    HelvarError::InvalidGroupIndex => 
                        "Invalid group index parameter",
                    HelvarError::InvalidCluster => "Invalid cluster parameter",
                    HelvarError::InvalidRouter => "Invalid router parameter",
                    HelvarError::InvalidSubnet => "Invalid subnet parameter",
                    HelvarError::InvalidDevice => "Invalid device parameter",
                    HelvarError::InvalidSubdevice => 
                        "Invalid subdevice parameter",
                    HelvarError::InvalidBlock => "Invalid block parameter",
                    HelvarError::InvalidScene => "Invalid scene parameter",
                    HelvarError::NoSuchCluster => "Cluster does not exist",
                    HelvarError::NoSuchRouter => "Router does not exist",
                    HelvarError::NoSuchDevice => "Device does not exist",
                    HelvarError::NoSuchProperty => "Device does not exist",
                    HelvarError::InvalidRawMessageSize =>
                        "Invalid raw message size",
                    HelvarError::InvalidMessagesType => "Invalid messages type",
                    HelvarError::InvalidMessageCommand =>
                        "Invalid message command",
                    HelvarError::Timeout => "Timeout",
                    _ => "?" 
                };
            f.write_str(msg)
        }
    }
}

#[test]
fn test_helvar_error()
{
    let e = HelvarError::from_code(8);
    println!("{}", e);
    let e2 = HelvarError::from_error(Box::new(e));
    println!("{}", e2);
    
    let e = HelvarError::from_code(18);
    println!("{}", e);
    let e2 = HelvarError::from_error(Box::new(e));
    println!("{}", e2);
    println!("{}", e2.source().unwrap());
    
    
}
