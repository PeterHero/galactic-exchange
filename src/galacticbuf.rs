#![allow(dead_code)]

use std::{collections::HashMap, fmt::Debug, hash::Hash};

const VERSION1: u8 = 0x01;
const INTEGER_T: u8 = 0x01;
const STRING_T: u8 = 0x02;
const LIST_T: u8 = 0x03;
const OBJECT_T: u8 = 0x04;

#[derive(Debug, PartialEq)]
struct Header {
    version: u8,
    field_count: u8,
    length: u16,
}

#[derive(Debug, PartialEq)]
struct Message {
    header: Header,
    body: HashMap<FieldName, FieldValue>,
}

#[derive(Clone, Debug, PartialEq)]
struct StringValue(String);
#[derive(Clone, Debug, PartialEq)]
enum List {
    Integers(Vec<i64>),
    Strings(Vec<StringValue>),
    Objects(Vec<Object>),
}
#[derive(Clone, Debug, PartialEq)]
struct Object(HashMap<FieldName, FieldValue>);

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
struct FieldName(String);

#[derive(Clone, Debug, PartialEq)]
enum FieldValue {
    Integer(i64),
    String(StringValue),
    List(List),
    Object(Object),
}

trait Serializable {
    fn serialize(&self) -> Vec<u8>;
}

#[derive(Debug)]
struct DeserializeError(String);

impl From<String> for DeserializeError {
    fn from(value: String) -> Self {
        Self(value)
    }
}

trait Deserializable: Sized {
    fn deserialize(bytes: &[u8], count: Option<usize>) -> Result<(Self, &[u8]), DeserializeError>;
}

/// [Integer - 8 bytes]
impl Serializable for i64 {
    fn serialize(&self) -> Vec<u8> {
        self.to_be_bytes().to_vec()
    }
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
impl Serializable for String {
    fn serialize(&self) -> Vec<u8> {
        self.as_bytes().to_vec()
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
impl<T: Serializable> Serializable for Vec<T> {
    fn serialize(&self) -> Vec<u8> {
        self.iter().map(|el| el.serialize()).flatten().collect()
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
impl<U: Serializable, V: Serializable> Serializable for (U, V) {
    fn serialize(&self) -> Vec<u8> {
        [self.0.serialize(), self.1.serialize()].concat()
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
impl<K: Serializable + Clone, V: Serializable + Clone> Serializable for HashMap<K, V> {
    fn serialize(&self) -> Vec<u8> {
        self.clone()
            .into_iter()
            .collect::<Vec<(K, V)>>()
            .serialize()
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
impl Serializable for List {
    fn serialize(&self) -> Vec<u8> {
        let (element_type, count, elements) = match self {
            List::Integers(integers) => (INTEGER_T, integers.len(), integers.serialize()),
            List::Strings(strings) => (STRING_T, strings.len(), strings.serialize()),
            List::Objects(objects) => (OBJECT_T, objects.len(), objects.serialize()),
        };
        assert!(
            count <= u16::MAX as usize,
            "Maximum list elements: 65,535 is supported"
        );
        [
            vec![element_type],
            (count as u16).to_be_bytes().to_vec(),
            elements,
        ]
        .concat()
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
impl Serializable for StringValue {
    fn serialize(&self) -> Vec<u8> {
        assert!(
            self.0.len() <= u16::MAX as usize,
            "Maximum string value length: 65,535 bytes is supported"
        );
        let length = (self.0.len() as u16).to_be_bytes();
        let string = self.0.as_bytes();
        [&length, string].concat()
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
impl Serializable for FieldName {
    fn serialize(&self) -> Vec<u8> {
        assert!(
            self.0.len() <= u8::MAX as usize,
            "Maximum field name length: 255 bytes is supported"
        );
        let length = [self.0.len() as u8];
        let string = self.0.as_bytes();
        [&length, string].concat()
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
impl Serializable for FieldValue {
    fn serialize(&self) -> Vec<u8> {
        let (type_indicator, value) = match self {
            Self::Integer(i) => (INTEGER_T, i.serialize()),
            Self::String(s) => (STRING_T, s.serialize()),
            Self::List(l) => (LIST_T, l.serialize()),
            Self::Object(o) => (OBJECT_T, o.serialize()),
        };
        [vec![type_indicator], value].concat()
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
impl Serializable for Object {
    fn serialize(&self) -> Vec<u8> {
        assert!(
            self.0.len() <= u8::MAX as usize,
            "Maximum fields per object: 255 is supported"
        );
        let count = self.0.len() as u8;
        let fields = self.0.serialize();
        [vec![count], fields].concat()
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
impl Serializable for Header {
    fn serialize(&self) -> Vec<u8> {
        let length = self.length.to_be_bytes();
        vec![self.version, self.field_count, length[0], length[1]]
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
impl Serializable for Message {
    fn serialize(&self) -> Vec<u8> {
        let header = self.header.serialize();
        let body = self.body.serialize();
        [header, body].concat()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_message() {
        // Message: `user_id=1001, name="Alice", scores=[100, 200, 300]`
        let message = Message {
            header: Header {
                version: VERSION1,
                field_count: 3,
                length: 69,
            },
            body: [
                (
                    FieldName(String::from("user_id")),
                    FieldValue::Integer(1001),
                ),
                (
                    FieldName(String::from("name")),
                    FieldValue::String(StringValue(String::from("Alice"))),
                ),
                (
                    FieldName(String::from("scores")),
                    FieldValue::List(List::Integers(vec![100, 200, 300])),
                ),
            ]
            .into(),
        };
        let binary_message: [u8; 69] = [
            // Header (4 bytes):
            0x01, //      - Protocol version
            0x03, //      - 3 fields
            0x00, 0x45, //  - Total length: 69 bytes
            // Field 1 - user_id (integer):
            0x07, //          - Name length: 7
            0x75, 0x73, 0x65, 0x72, 0x5F, 0x69, 0x64, // - "user_id" in UTF-8
            0x01, //          - Type: Integer
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0xE9, // - Value: 1001 (64-bit)
            // Field 2 - name (string):
            0x04, //                  - Name length: 4
            0x6E, 0x61, 0x6D, 0x65, //  - "name" in UTF-8
            0x02, //                  - Type: String
            0x00, 0x05, //              - String length: 5
            0x41, 0x6C, 0x69, 0x63, 0x65, // - "Alice" in UTF-8
            //Field 3 - scores (list of integers):
            0x06, //              - Name length: 6
            0x73, 0x63, 0x6F, 0x72, 0x65, 0x73, // - "scores" in UTF-8
            0x03, //              - Type: List
            0x01, //              - Element type: Integer
            0x00, 0x03, //          - Element count: 3
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x64, //      - 10x00
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xC8, //    - 20x00
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x2C, //    -30x00,
        ];
        let (deserialized_message, bytes) = Message::deserialize(&binary_message, None).unwrap();
        assert_eq!(bytes.len(), 0);
        assert_eq!(message, deserialized_message)
    }

    #[test]
    fn list_of_objects() {
        // ### Message with List of Objects: `timestamp=1698765432, trades=[{id:1, price:100}, {id:2, price:200}]`
        let message = Message {
            header: Header {
                version: VERSION1,
                field_count: 2,
                length: 90,
            },
            body: [
                (
                    FieldName(String::from("timestamp")),
                    FieldValue::Integer(1698765432),
                ),
                (
                    FieldName(String::from("trades")),
                    FieldValue::List(List::Objects(vec![
                        Object(
                            [
                                (FieldName(String::from("id")), FieldValue::Integer(1)),
                                (FieldName(String::from("price")), FieldValue::Integer(100)),
                            ]
                            .into(),
                        ),
                        Object(
                            [
                                (FieldName(String::from("id")), FieldValue::Integer(2)),
                                (FieldName(String::from("price")), FieldValue::Integer(200)),
                            ]
                            .into(),
                        ),
                    ])),
                ),
            ]
            .into(),
        };
        let binary_message: [u8; 90] = [
            // Header (4 bytes):
            0x01, //        - Protocol version
            0x02, //        - 2 fields
            0x00, 0x5a, //  - Total length: 90 bytes
            // Field 1 - timestamp (integer):
            0x09, //        - Name length: 9
            0x74, 0x69, 0x6D, 0x65, 0x73, 0x74, 0x61, 0x6D, 0x70, //    - "timestamp" in UTF-8
            0x01, //        - Type: Integer
            0x00, 0x00, 0x00, 0x00, 0x65, 0x41, 0x1A, 0x78, //  - Value: 1698765432
            // Field 2 - trades (list of objects):
            0x06, //        - Name length: 6
            0x74, 0x72, 0x61, 0x64, 0x65, 0x73, //  - "trades" in UTF-8
            0x03, //        - Type: List
            0x04, //        - Element type: Object
            0x00, 0x02, //  - Element count: 2
            // Object 1:
            0x02, //        - Field count: 2
            // Field: id
            0x02, //        - Name length: 2
            0x69, 0x64, //  - "id" in UTF-8
            0x01, //        - Type: Integer
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, //  - Value: 1
            // Field: price
            0x05, //        - Name length: 5
            0x70, 0x72, 0x69, 0x63, 0x65, //    - "price" in UTF-8
            0x01, //       - Type: Integer
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x64, //  - Value: 100
            // Object 2:
            0x02, //        - Field count: 2
            // Field: id
            0x02, //        - Name length: 2
            0x69, 0x64, //  - "id" in UTF-8
            0x01, //        - Type: Integer
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, //  - Value: 2
            // Field: price
            0x05, //        - Name length: 5
            0x70, 0x72, 0x69, 0x63, 0x65, //    - "price" in UTF-8
            0x01, //        - Type: Integer
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xC8, //  - Value: 200
        ];
        let (deserialized_message, bytes) = Message::deserialize(&binary_message, None).unwrap();
        assert_eq!(bytes.len(), 0);
        assert_eq!(message, deserialized_message)
    }
}
