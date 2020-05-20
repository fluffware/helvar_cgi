use std::fmt;
use std::mem::{self, MaybeUninit};

#[derive(Debug,Clone)]
pub struct DeviceState {
    pub address: u32,
    pub device_type: u32,
    pub intensity: u8,
    pub description: String
}

impl DeviceState
{
    pub fn new() ->DeviceState
    {
        DeviceState{
            address: 0,
            device_type: 0,
            intensity: 0,
            description: String::new()
        }
    }
}

pub struct SubnetState
{
    pub devices: [Option<Box<DeviceState>>;64]
}

impl SubnetState {
    pub fn new() -> SubnetState
    {
        let mut devices: [MaybeUninit<Option<Box<DeviceState>>>;64] = unsafe {
            MaybeUninit::uninit().assume_init()
        };
        for d in &mut devices[..] {
            *d = MaybeUninit::new(None);
        }
        SubnetState{devices:
                    unsafe {
                        mem::transmute::<_, [Option<Box<DeviceState>>;64]>(
                            devices)}}
    }
}
impl fmt::Debug for SubnetState
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        write!(f,"[")?;
        for dev in self.devices.iter() {
            (dev as &dyn fmt::Debug).fmt(f)?;
        }
        write!(f,"]")
    }
}


#[derive(Debug)]
pub struct RouterState {
    pub subnets: Vec<Option<Box<SubnetState>>>
}

impl RouterState
{
    pub fn new() ->RouterState
    {
        RouterState{subnets: Vec::new()}
    }
}
