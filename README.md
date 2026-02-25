# MCP Server POC

See https://github.com/afsalthaj/simple-mcp-server/blob/master/src/rmcp_streamable_http.rs

```sh

# manual handshake
cargo run --example manual_server

# rust sdk

cargo run --example golem_server_rmcp
# or
golem_server_flattened_rmcp
```


```sh

# This is important why because, none of the clients can connect to a local
# MCP servers through http (all of them use this STDIO which is of no use to us)
cloudflared tunnel --url http://127.0.0.1:8000

```

Then, copy the URL and add a tool `MCP Server` in https://platform.openai.com/chat/edit?models=gpt-4.1-mini or
clients of your choice. I will be surprised if this works in Claude Desktop (as I couldn't manage to get that going).

Apparently this play-ground will be our go-to manual-testing of golem servers too


## Manul server handshake details (the bare minimum that has to be there in GOLEM regardless of using SDK or not)

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

## Initialized notitifcation


```
curl -i -X POST https://telecom-exam-chances-developer.trycloudflare.com/my-agent/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: application/json" \
  -H "Mcp-Session-Id: 59fcd8cf-0399-481a-97ed-24284327b880" \
  -d '{
    "jsonrpc": "2.0",
    "method": "notifications/initialized",
    "params": {}
  }'

```

Response has SessionId in header. 


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
      "name": "counter",
      "arguments": {
        "number": 1
      }
    }
  }'


```

Now to test whether the simple protocol works, use openai's playground. But we need to expose it to the public internet first.
ngrok
