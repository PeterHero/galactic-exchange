id: auth
name: Authentication
summary: Implement user registration and login endpoints
points: 20
prerequisites: [health]
is_hidden: false
---

> NOTE: All request and response bodies need to be encoded using the GalacticBuf protocol. Follow the [link to the specification](./galacticbuf-v1.md)

Unfortunately, for now, we will have to abandon DNA-based logins and quantum curve cryptography, and fall back to a simple, proven, token-based approach.

Since the tokens are issued and checked by your exchange, their format and content are completely in your control. Clients will just send 
whatever was provided by the login endpoint back. Keeping their size reasonable and following modern security practices is *highly* recommended.

There are no restrictions on the password format, users are free to chose whatever password they want.

Your exchange should provide the following endpoints:

## POST /register

Creates a new user account.

**Request** (GalacticBuf binary format):
- `username` (String): User's chosen username
- `password` (String): User's password

**Response:**
- Success: `204 No Content` (empty response body)

### Status Codes

| HTTP Status     | Scenario                                    |
|-----------------|---------------------------------------------|
| 400 Bad Request | Invalid input (empty username/password)     |
| 409 Conflict    | Username already exists                     |
| 204 No Content  | User account created successfully           |

## POST /login

Authenticates a user and returns an authentication token.

**Request** (GalacticBuf binary format):
- `username` (String): User's username
- `password` (String): User's password

**Response:**
- Success: `200 OK` with GalacticBuf-encoded response containing:
  - `token` (String): Authentication token to use for protected endpoints

### Status Codes

| HTTP Status      | Scenario                                 |
|------------------|------------------------------------------|
| 401 Unauthorized | Invalid credentials or user doesn't exist|
| 200 OK           | Login successful, token returned         |

## Using Authentication Tokens

For protected endpoints, include the token in the `Authorization` header:
```
Authorization: Bearer <token>
```

All requests and responses use the GalacticBuf binary protocol with:
```
Content-Type: application/x-galacticbuf
```
