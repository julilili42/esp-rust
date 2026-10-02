- Protocol which provides full-duplex communication over a single TCP connection.
- WS are persistent (connection is established once) and bidirectional.
- Real-time communication with low latency.
- Starts with HTTP handshake, upgrades to WS protocol.
- WS connection remain open indefinitely until they are closed.

Comparison to HTTP

- HTTP follows a request-response pattern therefore Client must always initiate requests. Single TCP connection can hold multiple HTTP requests. For WS Protocol we do not have request-response pattern. Each party can send messages at any time.
- WS has little overhead.
- WS has low latency.
- WS is stateful. HTTP is stateless, horizontal scaling is easier.
