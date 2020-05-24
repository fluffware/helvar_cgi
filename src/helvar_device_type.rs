use std::convert::From;

pub struct HelvarDeviceType(u32);

impl From<u32> for HelvarDeviceType
{
    fn from(t: u32) -> HelvarDeviceType
    {
        HelvarDeviceType{0:t}
    }
}    
    
impl HelvarDeviceType
{
    pub fn is_load(&self) -> bool
    {
        match self.0 & 0xff {
            0x01 => { // DALI
                let dali_type = self.0>>8;
                dali_type <= 8
            },
            0x08 => {
                let dmx_type = self.0>>8;
                dmx_type == 0x02
            },
            _ => false
        }
    }
}
