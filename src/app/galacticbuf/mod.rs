pub mod deserialization;
pub mod serialization;

use std::{collections::HashMap, fmt::Debug, hash::Hash};

// Note: if serialization becomes bottleneck it can be quite improved - the current implementation
// is not optimized for performance

const VERSION1: u8 = 0x01;
const INTEGER_T: u8 = 0x01;
const STRING_T: u8 = 0x02;
const LIST_T: u8 = 0x03;
const OBJECT_T: u8 = 0x04;

#[derive(Debug, PartialEq)]
pub struct Header {
    version: u8,
    pub field_count: u8,
    length: u16,
}

#[derive(Debug, PartialEq)]
pub struct Message {
    pub header: Header,
    pub body: HashMap<FieldName, FieldValue>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StringValue(pub String);

#[derive(Clone, Debug, PartialEq)]
pub enum List {
    Integers(Vec<i64>),
    Strings(Vec<StringValue>),
    Objects(Vec<Object>),
}
#[derive(Clone, Debug, PartialEq)]
pub struct Object(HashMap<FieldName, FieldValue>);

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct FieldName(pub String);

#[derive(Clone, Debug, PartialEq)]
pub enum FieldValue {
    Integer(i64),
    String(StringValue),
    List(List),
    Object(Object),
}

#[derive(Debug)]
pub enum TransformationError {
    Invalid,
}

impl From<&str> for FieldName {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl From<i64> for FieldValue {
    fn from(value: i64) -> Self {
        FieldValue::Integer(value)
    }
}

impl From<String> for FieldValue {
    fn from(value: String) -> Self {
        FieldValue::String(StringValue(value))
    }
}

impl TryFrom<FieldValue> for i64 {
    type Error = TransformationError;

    fn try_from(value: FieldValue) -> Result<Self, Self::Error> {
        match value {
            FieldValue::Integer(n) => Ok(n),
            _ => Err(TransformationError::Invalid),
        }
    }
}

impl TryFrom<FieldValue> for String {
    type Error = TransformationError;

    fn try_from(value: FieldValue) -> Result<Self, Self::Error> {
        match value {
            FieldValue::String(StringValue(s)) => Ok(s),
            _ => Err(TransformationError::Invalid),
        }
    }
}

pub trait Extractable {
    fn get_value<T>(&mut self, key: &'static str) -> Option<T>
    where
        T: TryFrom<FieldValue>;
}

impl Extractable for HashMap<FieldName, FieldValue> {
    fn get_value<T>(&mut self, key: &'static str) -> Option<T>
    where
        T: TryFrom<FieldValue>,
    {
        self.remove(&FieldName(key.to_string()))?.try_into().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::deserialization::Deserializable;
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
