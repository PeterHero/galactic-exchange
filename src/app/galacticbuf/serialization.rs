use std::{collections::HashMap, fmt::Debug};

use crate::app::galacticbuf::{
    FieldName, FieldValue, Header, INTEGER_T, LIST_T, List, Message, OBJECT_T, Object, STRING_T,
    StringValue, VERSION1,
};

#[derive(Debug)]
pub struct SerializeError(String);

impl From<String> for SerializeError {
    fn from(value: String) -> Self {
        Self(value)
    }
}

trait Serializable {
    fn serialize(&self) -> Result<Vec<u8>, SerializeError>;
}

/// [Integer - 8 bytes]
impl Serializable for i64 {
    fn serialize(&self) -> Result<Vec<u8>, SerializeError> {
        Ok(self.to_be_bytes().to_vec())
    }
}

/// [UTF-8 Data]
impl Serializable for String {
    fn serialize(&self) -> Result<Vec<u8>, SerializeError> {
        Ok(self.as_bytes().to_vec())
    }
}

/// [Element 1][Element 2]...[Element N]
impl<T: Serializable> Serializable for Vec<T> {
    fn serialize(&self) -> Result<Vec<u8>, SerializeError> {
        let mut buffer = vec![];

        for (i, el) in self.iter().enumerate() {
            buffer.push(
                el.serialize()
                    .map_err(|SerializeError(err)| format!("at [{}]: {}", i, err))?,
            )
        }

        Ok(buffer.concat())
    }
}

/// [Value U][Value V]
impl<U: Serializable + Debug, V: Serializable> Serializable for (U, V) {
    fn serialize(&self) -> Result<Vec<u8>, SerializeError> {
        Ok([
            self.0
                .serialize()
                .map_err(|SerializeError(e)| format!("at (u, _): {}", e))?,
            self.1
                .serialize()
                .map_err(|SerializeError(e)| format!("at `{:?}`: {}", self.0, e))?,
        ]
        .concat())
    }
}

/// [Key 1][Value 1][Key 2][Value 2]...[Key N][Value N]
impl<K: Serializable + Clone + Debug, V: Serializable + Clone> Serializable for HashMap<K, V> {
    fn serialize(&self) -> Result<Vec<u8>, SerializeError> {
        self.clone()
            .into_iter()
            .collect::<Vec<(K, V)>>()
            .serialize()
    }
}

/// [Element Type (1 byte)][Element Count (2 bytes)][Elements...]
/// Element is one of Integer/String/Object
impl Serializable for List {
    fn serialize(&self) -> Result<Vec<u8>, SerializeError> {
        let (element_type, count, elements) = match self {
            List::Integers(integers) => (INTEGER_T, integers.len(), integers.serialize()),
            List::Strings(strings) => (STRING_T, strings.len(), strings.serialize()),
            List::Objects(objects) => (OBJECT_T, objects.len(), objects.serialize()),
        };

        if count <= u16::MAX as usize {
            return Err(SerializeError(String::from(
                "Maximum list elements: 65,535 is supported",
            )));
        }
        Ok([
            vec![element_type],
            (count as u16).to_be_bytes().to_vec(),
            elements?,
        ]
        .concat())
    }
}

/// [Length (2 byte)][UTF-8 Data]
impl Serializable for StringValue {
    fn serialize(&self) -> Result<Vec<u8>, SerializeError> {
        if self.0.len() > u16::MAX as usize {
            return Err(SerializeError(String::from(
                "Maximum String value length: 65,535 bytes is supported",
            )));
        }
        let length = (self.0.len() as u16).to_be_bytes().to_vec();
        let string = self.0.serialize()?;
        Ok([length, string].concat())
    }
}

/// [Length (1 byte)][UTF-8 Data]
impl Serializable for FieldName {
    fn serialize(&self) -> Result<Vec<u8>, SerializeError> {
        if self.0.len() > u8::MAX as usize {
            return Err(SerializeError(String::from(
                "Maximum Field name value length: 255 bytes is supported",
            )));
        }
        let length = vec![self.0.len() as u8];
        let string = self.0.serialize()?;
        Ok([length, string].concat())
    }
}

/// [Type (1 byte)][Integer/String/List/Object]
impl Serializable for FieldValue {
    fn serialize(&self) -> Result<Vec<u8>, SerializeError> {
        let (type_indicator, value) = match self {
            Self::Integer(i) => (INTEGER_T, i.serialize()?),
            Self::String(s) => (STRING_T, s.serialize()?),
            Self::List(l) => (LIST_T, l.serialize()?),
            Self::Object(o) => (OBJECT_T, o.serialize()?),
        };
        Ok([vec![type_indicator], value].concat())
    }
}

/// [Field Count (1 byte)][Field 1][Field 2]...[Field N]
impl Serializable for Object {
    fn serialize(&self) -> Result<Vec<u8>, SerializeError> {
        if self.0.len() > u8::MAX as usize {
            return Err(SerializeError(String::from(
                "Maximum fields per object: 255 is supported",
            )));
        }
        let count = self.0.len() as u8;
        let fields = self.0.serialize()?;
        Ok([vec![count], fields].concat())
    }
}

/// Byte 0: Protocol Version (0x01)
/// Byte 1: Field Count (0-255)
/// Bytes 2-3: Total Message Length (big-endian, includes header)
impl Serializable for Header {
    fn serialize(&self) -> Result<Vec<u8>, SerializeError> {
        let length = self.length.to_be_bytes();
        Ok(vec![self.version, self.field_count, length[0], length[1]])
    }
}

/// [Header][Field 1][Field 2]...[Field N]
impl Serializable for Message {
    fn serialize(&self) -> Result<Vec<u8>, SerializeError> {
        let header = self.header.serialize()?;
        let body = self.body.serialize()?;
        Ok([header, body].concat())
    }
}

pub fn serialize_message(body: HashMap<FieldName, FieldValue>) -> Result<Vec<u8>, SerializeError> {
    let mut message = Message {
        header: Header {
            version: VERSION1,
            field_count: body
                .len()
                .try_into()
                .map_err(|_| format!("Maximum field count is 255"))?,
            length: 0,
        },
        body,
    };

    message.header.length = message
        .serialize()?
        .len()
        .try_into()
        .map_err(|_| format!("Maximum message length is 65535"))?;

    message.serialize()
}
