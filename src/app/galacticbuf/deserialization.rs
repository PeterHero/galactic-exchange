use std::{collections::HashMap, fmt::Debug, hash::Hash};

use crate::app::galacticbuf::{
    FieldName, FieldValue, Header, INTEGER_T, LIST_T, List, Message, OBJECT_T, Object, STRING_T,
    StringValue, VERSION1,
};

#[derive(Debug)]
pub struct DeserializeError(String);

impl From<String> for DeserializeError {
    fn from(value: String) -> Self {
        Self(value)
    }
}

pub trait Deserializable: Sized {
    fn deserialize(bytes: &[u8], count: Option<usize>) -> Result<(Self, &[u8]), DeserializeError>;
}

/// [Integer - 8 bytes]
impl Deserializable for i64 {
    fn deserialize(bytes: &[u8], _: Option<usize>) -> Result<(Self, &[u8]), DeserializeError> {
        let Some(integer) = bytes
            .get(..std::mem::size_of::<i64>())
            .and_then(|b| b.try_into().ok())
            .map(i64::from_be_bytes)
        else {
            return Err(DeserializeError(format!("expected i64, end of buffer!")));
        };

        let bytes = match bytes.get(std::mem::size_of::<i64>()..) {
            Some(slice) => slice,
            None => &[],
        };
        Ok((integer, bytes))
    }
}

/// [UTF-8 Data]
impl Deserializable for String {
    fn deserialize(bytes: &[u8], count: Option<usize>) -> Result<(Self, &[u8]), DeserializeError> {
        let count = count.unwrap_or(0);

        if count == 0 {
            return Ok((String::new(), bytes));
        }

        let name = bytes.get(..count).ok_or(format!(
            "expected string of length {}, end of buffer!",
            count
        ))?;
        let name = std::str::from_utf8(name).map_err(|e| format!("invalid utf-8 string: {}", e))?;
        let bytes = match bytes.get(count..) {
            Some(slice) => slice,
            None => &[],
        };
        Ok((String::from(name), bytes))
    }
}

/// [Element 1][Element 2]...[Element N]
impl<T: Deserializable> Deserializable for Vec<T> {
    fn deserialize(
        mut bytes: &[u8],
        count: Option<usize>,
    ) -> Result<(Self, &[u8]), DeserializeError> {
        let count = count.unwrap_or(0);

        let mut list = vec![];
        for i in 0..count {
            let (element, next_bytes) = T::deserialize(bytes, None)
                .map_err(|DeserializeError(e)| format!("at [{}]: {}", i, e))?;
            list.push(element);
            bytes = next_bytes;
        }

        Ok((list, bytes))
    }
}

/// [Value U][Value V]
impl<U: Deserializable + Debug, V: Deserializable> Deserializable for (U, V) {
    fn deserialize(bytes: &[u8], _: Option<usize>) -> Result<(Self, &[u8]), DeserializeError> {
        let (u, bytes) = U::deserialize(bytes, None)
            .map_err(|DeserializeError(e)| format!("at (u, _): {}", e))?;
        let (v, bytes) = V::deserialize(bytes, None)
            .map_err(|DeserializeError(e)| format!("at `{:?}`: {}", u, e))?;
        Ok(((u, v), bytes))
    }
}

/// [Key 1][Value 1][Key 2][Value 2]...[Key N][Value N]
impl<K: Deserializable + Eq + Hash + Debug, V: Deserializable> Deserializable for HashMap<K, V> {
    fn deserialize(bytes: &[u8], count: Option<usize>) -> Result<(Self, &[u8]), DeserializeError> {
        let (list, bytes) = Vec::<(K, V)>::deserialize(bytes, count)?;
        let map = list.into_iter().collect();
        Ok((map, bytes))
    }
}

/// [Element Type (1 byte)][Element Count (2 bytes)][Elements...]
/// Element is one of Integer/String/Object
impl Deserializable for List {
    fn deserialize(bytes: &[u8], _: Option<usize>) -> Result<(Self, &[u8]), DeserializeError> {
        let element_type = *bytes
            .get(0)
            .ok_or(format!("expected u8 (element type), end of buffer!"))?;
        let bytes = match bytes.get(std::mem::size_of::<u8>()..) {
            Some(slice) => slice,
            None => &[],
        };

        let count = bytes
            .get(..std::mem::size_of::<u16>())
            .and_then(|b| b.try_into().ok())
            .map(u16::from_be_bytes)
            .ok_or(format!("expected u16 (count), end of buffer!"))? as usize;

        let bytes = match bytes.get(std::mem::size_of::<u16>()..) {
            Some(slice) => slice,
            None => &[],
        };
        let (elements, bytes) = match element_type {
            INTEGER_T => {
                let (integers, bytes) = Vec::<i64>::deserialize(bytes, Some(count))?;
                (List::Integers(integers), bytes)
            }
            STRING_T => {
                let (strings, bytes) = Vec::<StringValue>::deserialize(bytes, Some(count))?;
                (List::Strings(strings), bytes)
            }
            OBJECT_T => {
                let (objects, bytes) = Vec::<Object>::deserialize(bytes, Some(count))?;
                (List::Objects(objects), bytes)
            }
            t => {
                return Err(DeserializeError(format!(
                    "Unsupported type {}, expected one of {} = Integer, {} = String, {} = Object",
                    t, INTEGER_T, STRING_T, OBJECT_T
                )));
            }
        };
        Ok((elements, bytes))
    }
}

/// [Length (2 byte)][UTF-8 Data]
impl Deserializable for StringValue {
    fn deserialize(bytes: &[u8], _: Option<usize>) -> Result<(Self, &[u8]), DeserializeError> {
        let length = bytes
            .get(..std::mem::size_of::<u16>())
            .and_then(|b| b.try_into().ok())
            .map(u16::from_be_bytes)
            .ok_or(format!("expected u16 (length), end of buffer!"))? as usize;

        let bytes = match bytes.get(std::mem::size_of::<u16>()..) {
            Some(slice) => slice,
            None => &[],
        };
        let (string, bytes) = String::deserialize(bytes, Some(length))?;
        Ok((StringValue(string), bytes))
    }
}

/// [Length (1 byte)][UTF-8 Data]
impl Deserializable for FieldName {
    fn deserialize(bytes: &[u8], _: Option<usize>) -> Result<(Self, &[u8]), DeserializeError> {
        let length = *bytes
            .get(0)
            .ok_or(format!("expected u8 (element type), end of buffer!"))?
            as usize;
        let bytes = match bytes.get(std::mem::size_of::<u8>()..) {
            Some(slice) => slice,
            None => &[],
        };
        let (string, bytes) = String::deserialize(bytes, Some(length))?;
        Ok((FieldName(string), bytes))
    }
}

/// [Type (1 byte)][Integer/String/List/Object]
impl Deserializable for FieldValue {
    fn deserialize(bytes: &[u8], _: Option<usize>) -> Result<(Self, &[u8]), DeserializeError> {
        let type_indicator = *bytes
            .get(0)
            .ok_or(format!("expected u8 (type indicator), end of buffer!"))?;
        let bytes = match bytes.get(std::mem::size_of::<u8>()..) {
            Some(slice) => slice,
            None => &[],
        };
        let (value, bytes) = match type_indicator {
            INTEGER_T => {
                let (integer, bytes) = i64::deserialize(bytes, None)?;
                (FieldValue::Integer(integer), bytes)
            }
            STRING_T => {
                let (string, bytes) = StringValue::deserialize(bytes, None)?;
                (FieldValue::String(string), bytes)
            }
            LIST_T => {
                let (list, bytes) = List::deserialize(bytes, None)?;
                (FieldValue::List(list), bytes)
            }
            OBJECT_T => {
                let (object, bytes) = Object::deserialize(bytes, None)?;
                (FieldValue::Object(object), bytes)
            }
            t => {
                return Err(DeserializeError(format!(
                    "Unsupported type {}, expected one of {} = Integer, {} = String, {} = List, {} = Object",
                    t, INTEGER_T, STRING_T, LIST_T, OBJECT_T
                )));
            }
        };
        Ok((value, bytes))
    }
}

/// [Field Count (1 byte)][Field 1][Field 2]...[Field N]
impl Deserializable for Object {
    fn deserialize(bytes: &[u8], _: Option<usize>) -> Result<(Self, &[u8]), DeserializeError> {
        let count = *bytes
            .get(0)
            .ok_or(format!("expected u8 (count), end of buffer!"))? as usize;
        let bytes = match bytes.get(std::mem::size_of::<u8>()..) {
            Some(slice) => slice,
            None => &[],
        };
        let (object, bytes) = HashMap::<FieldName, FieldValue>::deserialize(bytes, Some(count))?;
        Ok((Object(object), bytes))
    }
}

/// Byte 0: Protocol Version (0x01)
/// Byte 1: Field Count (0-255)
/// Bytes 2-3: Total Message Length (big-endian, includes header)
impl Deserializable for Header {
    fn deserialize(bytes: &[u8], _: Option<usize>) -> Result<(Self, &[u8]), DeserializeError> {
        bytes
            .get(..4)
            .ok_or(format!("expected 4 byte header, end of buffer!"))?;
        let header = Header {
            version: bytes[0],
            field_count: bytes[1],
            length: u16::from_be_bytes([bytes[2], bytes[3]]),
        };
        let bytes = match bytes.get(4..) {
            Some(slice) => slice,
            None => &[],
        };
        Ok((header, bytes))
    }
}

/// [Header][Field 1][Field 2]...[Field N]
impl Deserializable for Message {
    fn deserialize(bytes: &[u8], _: Option<usize>) -> Result<(Self, &[u8]), DeserializeError> {
        let old_bytes = bytes;
        let (header, bytes) = Header::deserialize(bytes, None)?;

        if header.version != VERSION1 {
            return Err(DeserializeError(format!(
                "expected version: {}, found: {}",
                VERSION1, header.version
            )));
        }

        if header.length as usize > bytes.len() + 4 {
            return Err(DeserializeError(format!(
                "buffer: {} is shorter than the message length: {}!",
                bytes.len() + 4,
                header.length
            )));
        }

        let (body, bytes) = HashMap::<FieldName, FieldValue>::deserialize(
            bytes,
            Some(header.field_count as usize),
        )?;

        let message_length = old_bytes.len() - bytes.len();
        if message_length != header.length as usize {
            return Err(DeserializeError(format!(
                "message length: {} does not match the length in header: {}",
                message_length, header.length
            )));
        }

        Ok((Message { header, body }, bytes))
    }
}
