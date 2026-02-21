id: galacticbuf-v1
name: GalacticBuf serialization
summary: Implement the serialization protocol used by the Galactic Energy Council
prerequisites: [intro]
is_hidden: false
mission_type: intel
---

The GalacticBuf protocol has been the official data format of the Galactic Energy Council since before anyone can remember.
Its origins are shrouded in mystery - some say it was discovered in the ruins of an ancient trading outpost, others claim it was handed down by the First Traders themselves.
Regardless of its origins, one thing is certain: the Council requires all exchange communications to use this protocol. No exceptions. No questions asked.

Implement the following serialization protocol, and use it for all request and response bodies for subsequent tasks.

## Overview
A compact binary protocol for transmitting structured data with named fields supporting integers and strings.

## Message Structure

### Message Format
```
[Header][Field 1][Field 2]...[Field N]
```

### Header (4 bytes)
```
Byte 0: Protocol Version (0x01)
Byte 1: Field Count (0-255)
Bytes 2-3: Total Message Length (big-endian, includes header)
```

### Field Format
```
[Field Name Length][Field Name][Type][Value]
```

## Field Components

### Field Name Length (1 byte)
- Length of field name in bytes (1-255)

### Field Name (variable)
- UTF-8 encoded string
- Length specified by previous byte

### Type Indicator (1 byte)
- `0x01` - Integer (64-bit signed)
- `0x02` - String (variable length)
- `0x03` - List (variable length, homogeneous type)
- `0x04` - Object (nested structure)

### Value (variable)

#### Integer (`0x01`) - 8 bytes
- 64-bit signed integer
- Big-endian byte order
- Range: -9,223,372,036,854,775,808 to 9,223,372,036,854,775,807

#### String (`0x02`) - variable
```
[Length (2 bytes)][UTF-8 Data]
```
- First 2 bytes: string length in bytes (big-endian, 0-65535)
- Followed by UTF-8 encoded string data

#### List (`0x03`) - variable
```
[Element Type (1 byte)][Element Count (2 bytes)][Elements...]
```
- Element Type: `0x01` for integers, `0x02` for strings, `0x04` for objects
- Element Count: number of elements (big-endian, 0-65535)
- Elements: each element encoded according to its type
    - Integers: 8 bytes each (big-endian)
    - Strings: 2-byte length + UTF-8 data for each string
    - Objects: nested GalacticBuf message for each object (without outer header)

#### Object (`0x04`) - variable
```
[Field Count (1 byte)][Field 1][Field 2]...[Field N]
```
- Nested structure containing fields in the same format as top-level message
- Does not include the 4-byte protocol header (version/total length)
- Only includes field count followed by fields
- Each field follows the standard field format: [name length][name][type][value]

## Complete Example

### Message: `user_id=1001, name="Alice", scores=[100, 200, 300]`

```
Header (4 bytes):
  01           - Protocol version
  03           - 3 fields
  00 45        - Total length: 69 bytes

Field 1 - user_id (integer):
  07           - Name length: 7
  75 73 65 72 5F 69 64  - "user_id" in UTF-8
  01           - Type: Integer
  00 00 00 00 00 00 03 E9  - Value: 1001 (64-bit)

Field 2 - name (string):
  04           - Name length: 4
  6E 61 6D 65  - "name" in UTF-8
  02           - Type: String
  00 05        - String length: 5
  41 6C 69 63 65  - "Alice" in UTF-8

Field 3 - scores (list of integers):
  06           - Name length: 6
  73 63 6F 72 65 73  - "scores" in UTF-8
  03           - Type: List
  01           - Element type: Integer
  00 03        - Element count: 3
  00 00 00 00 00 00 00 64  - 100
  00 00 00 00 00 00 00 C8  - 200
  00 00 00 00 00 00 01 2C  - 300
```

**Total: 69 bytes**

### Message with List of Objects: `timestamp=1698765432, trades=[{id:1, price:100}, {id:2, price:200}]`

```
Header (4 bytes):
  01           - Protocol version
  02           - 2 fields
  00 5a        - Total length: 90 bytes

Field 1 - timestamp (integer):
  09           - Name length: 9
  74 69 6D 65 73 74 61 6D 70  - "timestamp" in UTF-8
  01           - Type: Integer
  00 00 00 00 65 41 1A 78  - Value: 1698765432

Field 2 - trades (list of objects):
  06           - Name length: 6
  74 72 61 64 65 73  - "trades" in UTF-8
  03           - Type: List
  04           - Element type: Object
  00 02        - Element count: 2

  Object 1:
    02         - Field count: 2

    Field: id
      02       - Name length: 2
      69 64    - "id" in UTF-8
      01       - Type: Integer
      00 00 00 00 00 00 00 01  - Value: 1

    Field: price
      05       - Name length: 5
      70 72 69 63 65  - "price" in UTF-8
      01       - Type: Integer
      00 00 00 00 00 00 00 64  - Value: 100

  Object 2:
    02         - Field count: 2

    Field: id
      02       - Name length: 2
      69 64    - "id" in UTF-8
      01       - Type: Integer
      00 00 00 00 00 00 00 02  - Value: 2

    Field: price
      05       - Name length: 5
      70 72 69 63 65  - "price" in UTF-8
      01       - Type: Integer
      00 00 00 00 00 00 00 C8  - Value: 200
```

**Total: 90 bytes**

## Parsing Algorithm

```
1. Read 4-byte header
   - Verify version = 0x01
   - Read field count
   - Read total message length

2. For each field (field_count times):
   a. Read 1 byte: field name length (N)
   b. Read N bytes: field name (UTF-8)
   c. Read 1 byte: type indicator
   d. Based on type:
      - If 0x01: Read 8 bytes as big-endian int64
      - If 0x02:
        * Read 2 bytes as big-endian uint16 (length L)
        * Read L bytes as UTF-8 string
      - If 0x03:
        * Read 1 byte: element type (0x01, 0x02, or 0x04)
        * Read 2 bytes as big-endian uint16 (element count N)
        * For each element (N times):
          - If element type 0x01: Read 8 bytes as int64
          - If element type 0x02: Read 2-byte length + UTF-8 string
          - If element type 0x04: Read nested object (field count + fields)
      - If 0x04:
        * Read 1 byte: field count
        * Read fields using same parsing rules as step 2
```

## Limits
- Maximum fields per message: 255
- Maximum message size: 65,535 bytes
- Maximum field name length: 255 bytes
- Maximum string value length: 65,535 bytes
- Maximum list elements: 65,535
- Integer range: -2⁶³ to 2⁶³-1
- Lists are homogeneous (all elements same type)
