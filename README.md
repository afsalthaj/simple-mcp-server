# MCP flow

```sh

cargo run


```

Note that the flow is simple HTTP with JSON-RPC 2.0, so you can use any HTTP client to interact with the MCP server. 
Below are some example `curl` commands to demonstrate the flow.

SSE or streamable http response can/should be enabled soon. But I also read Claude Desktop for instance is getting rid of SSE

## Initialize
```sh
curl -i -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "initialize",
    "params": {
      "protocolVersion": "2025-06-18",
      "capabilities": {},
      "clientInfo": {
        "name": "curl-client",
        "version": "1.0.0"
      }
    }
  }'



```

## List Capabilities

```sh
curl -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: application/json" \
  -H "Mcp-Session-Id: <SESSION_ID>" \
  -d '{
    "jsonrpc": "2.0",
    "id": 2,
    "method": "tools/list",
    "params": {}
  }'


```

## Actual Tool


```sh

curl -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: application/json" \
  -H "Mcp-Session-Id: <SESSION_ID>" \
  -d '{
    "jsonrpc": "2.0",
    "id": 3,
    "method": "tools/call",
    "params": {
      "name": "hello_agent",
      "arguments": {
        "name": "Alice"
      }
    }
  }'


```

Now to test whether the simple protocol works, use openai's playground. But we need to expose it to the public internet first.
ngrok