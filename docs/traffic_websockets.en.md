# Traffic WebSocket Integration Guide (Centrifugo)

This document describes how client applications (Mobile, Desktop, Web) integrate with the Centrifugo WebSocket server to receive real-time traffic limit updates (total and remaining traffic).

## Integration Flow

Since history and message recovery are disabled for the personal namespace, the client MUST follow this sequence:

```mermaid
sequenceDiagram
    participant Client
    participant Hub API
    participant Centrifugo WS

    Client->>Hub API: GET /traffic/me (Load initial traffic state)
    Hub API-->>Client: 200 OK (traffic_total_bytes, remaining_bytes, updated_at_ms)
    Note over Client: Store last_applied_ms = updated_at_ms
    
    Client->>Hub API: GET /traffic/ws-tokens (Request tokens)
    Hub API-->>Client: 200 OK (connection_token, subscription_token, channel)
    
    Client->>Centrifugo WS: Connect (ws://<host>:38000/connection/websocket)
    Centrifugo WS-->>Client: WS Established
    
    Client->>Centrifugo WS: Send Connect Frame (connection_token)
    Centrifugo WS-->>Client: Receive Connect Reply (id: 1)
    
    Client->>Centrifugo WS: Send Subscribe Frame (channel, subscription_token)
    Centrifugo WS-->>Client: Receive Subscribe Reply (id: 2)
    
    Note over Client, Centrifugo WS: Listening to real-time events...
    
    alt Traffic Changes (Add/Consume)
        Centrifugo WS->>Client: Push Notification Frame (updated traffic stats + updated_at_ms)
        Note over Client: Apply only if push.updated_at_ms > last_applied_ms
    end
    
    alt Connection Dropped
        Centrifugo WS--xClient: Disconnected
        Client->>Hub API: GET /traffic/me (Fetch fresh state / sync values)
        Note over Client: Update last_applied_ms = response.updated_at_ms
        Client->>Hub API: GET /traffic/ws-tokens (Get fresh tokens)
        Client->>Centrifugo WS: Re-establish connection & Re-subscribe
    end
```

---

## 1. Requesting WebSocket Tokens

Before connecting to the WebSocket server, the client must obtain connection and subscription tokens from the Hub API.

* **Endpoint:** `GET /traffic/ws-tokens`
* **Headers:** `Authorization: Bearer <jwt_access_token>`
* **Response DTO (`CentrifugeTokenDto`):**
  ```json
  {
    "status": "success",
    "data": {
      "connection_token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...",
      "subscription_token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...",
      "channel": "personal:5c841783-8f96-4fef-b268-3839f6c6baf0"
    }
  }
  ```

---

## 2. Connecting to WebSocket

Connect your WebSocket client to the Centrifugo endpoint:
* **Local Development:** `ws://localhost:38000/connection/websocket`
* **Production:** `wss://<your-domain>/connection/websocket` (or configured custom port)

---

## 3. Centrifugo Protocol Handshake

Centrifugo utilizes a simple frame framing protocol. Frames are sent as text messages.

### Step A: Send Connect Command
Send a JSON payload to authenticate the connection immediately after the WebSocket is opened:
```json
{
  "connect": {
    "token": "<connection_token>"
  },
  "id": 1
}
```

**Centrifugo Connect Reply:**
```json
{
  "id": 1,
  "connect": {
    "client": "6c39a659-eb2c-40a3-9655-b07e945daca9",
    "version": "5.2.2",
    "expires": true,
    "ttl": 86400,
    "ping": 25,
    "pong": true
  }
}
```

### Step B: Send Subscribe Command
Subscribe to the personal channel returned in the `CentrifugeTokenDto` (`channel` field):
```json
{
  "subscribe": {
    "channel": "personal:5c841783-8f96-4fef-b268-3839f6c6baf0",
    "token": "<subscription_token>"
  },
  "id": 2
}
```

**Centrifugo Subscribe Reply:**
```json
{
  "id": 2,
  "subscribe": {
    "expires": true,
    "ttl": 7200
  }
}
```

---

## 4. Handling Traffic Update Pushes

Whenever user traffic is consumed by a VPN node or new traffic is added via the API, Centrifugo pushes an update frame to the WebSocket:

```json
{
  "push": {
    "channel": "personal:5c841783-8f96-4fef-b268-3839f6c6baf0",
    "pub": {
      "data": {
        "traffic_total_bytes": 26843545600,
        "traffic_remaining_bytes": 24696061952,
        "updated_at_ms": 1753192800000
      }
    }
  }
}
```

* **`traffic_total_bytes`**: The sum of all active, non-expired packets' limits.
* **`traffic_remaining_bytes`**: The sum of all active packets' remaining traffic.
* **`updated_at_ms`**: Unix timestamp in milliseconds of the last DB write. Use as a monotonic cursor (see section below).

---

## 5. Solving the REST vs WebSocket Race Condition

A **race condition** exists between the initial REST fetch and the WebSocket stream:

```
Client            Server
  |                  |
  |--GET /traffic/me>|    # request snapshot
  |                  |--push (updated_at_ms=T2)-->  # node drained traffic
  |<--200 (T1)-------|    # stale snapshot arrives AFTER ws push
  # ❌ Client overwrites T2 with T1
```

### Solution: `updated_at_ms` monotonic cursor

Both `GET /traffic/me` and every WS push include `updated_at_ms` — the `MAX(modified_at)` of all active packets, expressed as Unix milliseconds.

**Client pseudocode:**
```swift
var lastAppliedMs: Int64 = 0

func applyTrafficState(totalBytes: Int64, remainingBytes: Int64, updatedAtMs: Int64) {
    guard updatedAtMs > lastAppliedMs else { return }  // discard stale
    lastAppliedMs = updatedAtMs
    // update UI
}

// On app launch:
let snapshot = await GET("/traffic/me")          // { ..., updated_at_ms: T1 }
applyTrafficState(...snapshot, updatedAtMs: T1)
subscribeWebSocket()  // may receive pushes with T0..T1..T2

// On each WS push:
let push = receivedPush.data                     // { ..., updated_at_ms: T2 }
applyTrafficState(...push, updatedAtMs: T2)       // T2 > T1 → applied; T0 → discarded
```

> [!TIP]
> The order of REST and WS subscription **does not matter**. Subscribe first or fetch first — `updated_at_ms` always wins.

---

## 5. Reconnection & Recovery Policy

> [!IMPORTANT]
> **No History/Recovery:** The `personal` channel namespace has history disabled.
>
> When the client disconnects and reconnects (e.g. due to internet handovers, sleep mode, or network dropouts), it **must not** attempt recovery. Instead, it must make a standard HTTP request to `GET /traffic/me` to synchronize the absolute total/remaining traffic bytes, fetch fresh tokens from `GET /traffic/ws-tokens`, and perform the handshake sequence again.
