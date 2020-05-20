use bytes::Bytes;
use bytes::Buf;
use std::convert::TryInto;

/// Read name-value pair
/// Returns (name,value,remaining)
pub fn decode_name_value_pair(mut block: Bytes)
                              -> (Bytes, Bytes, Bytes)
{
    let name_length: usize =
        if (block[0] & 0x80) == 0 {
            block.get_u8().into()
        } else {
            (block.get_u32() & 0x7fffffff).try_into().unwrap() 
        };
    let value_length: usize =
        if (block[0] & 0x80) == 0 {
            block.get_u8().into()
        } else {
            (block.get_u32() & 0x7fffffff).try_into().unwrap()
        };
    let name = block.split_to(name_length);
    let value = block.split_to(value_length);
    (name, value, block)
}

#[test]
fn test_decode_name_value_pair_11()
{
    let mut block = Bytes::from_static(&[2u8,3,1,2,6,5,4]);
    let (name, value, b) = decode_name_value_pair(block);
    assert_eq!(name, Bytes::from_static(&[1,2]));
    assert_eq!(value, Bytes::from_static(&[6,5,4]));
}

#[test]
fn test_decode_name_value_pair_41()
{
    let mut block = Bytes::from_static(&[0x80u8,0,0,3, 3, 1,2,3, 6,5,4]);
    let (name, value, b) = decode_name_value_pair(block);
    assert_eq!(name, Bytes::from_static(&[1,2,3]));
    assert_eq!(value, Bytes::from_static(&[6,5,4]));
}

#[test]
fn test_decode_name_value_pair_14()
{
    let mut block = Bytes::from_static(&[3u8,0x80, 0,0,3, 1,2,3, 6,5,4]);
    let (name, value, b) = decode_name_value_pair(block);
    assert_eq!(name, Bytes::from_static(&[1,2,3]));
    assert_eq!(value, Bytes::from_static(&[6,5,4]));
}

#[test]
fn test_decode_name_value_pair_44()
{
    let mut block = Bytes::from_static(&[0x80u8, 0,0, 3,0x80, 0,0,3, 1,2,3,
                                         6,5,4]);
    let (name, value, b) = decode_name_value_pair(block);
    assert_eq!(name, Bytes::from_static(&[1,2,3]));
    assert_eq!(value, Bytes::from_static(&[6,5,4]));
}
