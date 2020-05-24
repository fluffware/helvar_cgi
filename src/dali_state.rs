use std::fmt;
use std::mem::{self, MaybeUninit};
use std::convert::TryFrom;

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
    pub index: u32,
    pub devices: [Option<Box<DeviceState>>;64]
}

impl SubnetState {
    pub fn new(index: u32) -> SubnetState
    {
        let mut devices: [MaybeUninit<Option<Box<DeviceState>>>;64] = unsafe {
            MaybeUninit::uninit().assume_init()
        };
        for d in &mut devices[..] {
            *d = MaybeUninit::new(None);
        }
        SubnetState{index,
                    devices:
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
        let mut iter = self.devices.iter().peekable();
        while let Some(dev) = &mut iter.next()  {
            (dev as &dyn fmt::Debug).fmt(f)?;
            if iter.peek().is_some() {
                write!(f,", ")?;
            }
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

    pub fn get_subnet<'a>(&'a self, subnet: u32) -> Option<&'a SubnetState>
    {
        let subnet: usize = usize::try_from(subnet).ok()?;
        self.subnets.get(subnet - 1).and_then(|x| x.as_deref())
    }
    
    pub fn get_subnet_mut<'a>(&'a mut self, subnet: u32)
                              -> Option<&'a mut  SubnetState>
    {
        let subnet: usize = usize::try_from(subnet).ok()?;
        self.subnets.get_mut(subnet - 1).and_then(|x| x.as_deref_mut())
    }

    pub fn get_device<'a>(&'a self, subnet: u32, addr: u32)
                          -> Option<&'a DeviceState>
    {
        let sn = self.get_subnet(subnet)?;
        let addr: usize = usize::try_from(addr).ok()?;
        sn.devices.get(addr - 1).and_then(|x| x.as_deref())
    }
    
    pub fn get_device_mut<'a>(&'a mut self, subnet: u32, addr: u32)
                          -> Option<&'a mut DeviceState>
    {
        let sn = self.get_subnet_mut(subnet)?;
        let addr: usize = usize::try_from(addr).ok()?;
        sn.devices.get_mut(addr - 1).and_then(|x| x.as_deref_mut())
    }

    
}
