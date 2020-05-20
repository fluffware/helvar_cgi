use bytes::BufMut;
use std::convert::TryFrom;

/// Write name-value pair
pub fn encode_name_value_pair(buf: &mut dyn BufMut, name: & [u8], value: & [u8])
{
    if name.len() > 127 {
        buf.put_u32(u32::try_from(name.len()).unwrap() | 0x8000_0000u32);
    } else {
        buf.put_u8(u8::try_from(name.len()).unwrap());
    }
    if value.len() > 127 {
        buf.put_u32(u32::try_from(value.len()).unwrap() | 0x8000_0000u32);
    } else {
        buf.put_u8(u8::try_from(value.len()).unwrap());
    }
    
    buf.put_slice(name);
    buf.put_slice(value);
}
