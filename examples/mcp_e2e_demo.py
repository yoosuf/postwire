#!/usr/bin/env python3
"""End-to-end demo of the postwire-mcp tools, simulating an agentic e2e test.

Spawns `postwire-mcp` as a subprocess and talks JSON-RPC 2.0 to it over stdio,
exactly like an MCP-aware coding agent (Claude/Copilot/Cursor) would:

  1. initialize                          - handshake
  2. tools/list                          - discover available tools (14 tools)
  --- EMAIL FLOW ---
  3. tools/call send_test_email          - simulate "the app sent an email"
  4. tools/call wait_for_email           - block until email arrives
  5. tools/call extract_signals          - pull OTP code / link from email
  6. tools/call delete_email             - clean up email
  --- SMS FLOW ---
  7. tools/call send_test_sms            - simulate "the app sent an SMS"
  8. tools/call wait_for_sms             - block until SMS arrives
  9. tools/call extract_sms_signals      - pull 2FA OTP code from SMS
 10. tools/call delete_sms               - clean up SMS

Requires a running Postwire server (default http://127.0.0.1:8025) and the
postwire-mcp binary built (`cargo build --release -p postwire-mcp`).

Usage:
    POSTWIRE_URL=http://127.0.0.1:8025 python3 examples/mcp_e2e_demo.py \
        [path/to/postwire-mcp]
"""

import json
import os
import subprocess
import sys
import time

BIN = sys.argv[1] if len(sys.argv) > 1 else "target/release/postwire-mcp"


class McpClient:
    def __init__(self, binary_path: str):
        env = {**os.environ}
        self.proc = subprocess.Popen(
            [binary_path],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            text=True,
            bufsize=1,
            env=env,
        )
        self._next_id = 1

    def call(self, method: str, params: dict | None = None) -> dict:
        request_id = self._next_id
        self._next_id += 1
        request = {"jsonrpc": "2.0", "id": request_id, "method": method, "params": params or {}}
        assert self.proc.stdin and self.proc.stdout
        self.proc.stdin.write(json.dumps(request) + "\n")
        self.proc.stdin.flush()
        line = self.proc.stdout.readline()
        response = json.loads(line)
        if "error" in response:
            raise RuntimeError(f"{method} failed: {response['error']}")
        return response["result"]

    def call_tool(self, name: str, arguments: dict | None = None) -> dict:
        result = self.call("tools/call", {"name": name, "arguments": arguments or {}})
        if result.get("isError"):
            raise RuntimeError(f"tool {name} failed: {result}")
        return json.loads(result["content"][0]["text"])

    def close(self):
        self.proc.stdin.close()
        self.proc.wait(timeout=5)


def main() -> None:
    client = McpClient(BIN)
    try:
        info = client.call("initialize")
        print(f"connected to {info['serverInfo']['name']} v{info['serverInfo']['version']}")

        tools = client.call("tools/list")["tools"]
        print(f"discovered {len(tools)} tools: {', '.join(t['name'] for t in tools)}")

        # --- EMAIL DEMO FLOW ---
        to_address = "agent-test@example.com"
        since_ms = int(time.time() * 1000)

        print(f"\n--- EMAIL FLOW ---")
        print(f"1. simulating app sending email to {to_address} ...")
        client.call_tool("send_test_email", {"to": to_address})

        print("2. waiting for email to arrive via long-polling (wait_for_email) ...")
        message = client.call_tool(
            "wait_for_email",
            {"to": to_address, "subject": "Test email", "since_ms": since_ms, "timeout_ms": 10_000},
        )
        print(f"   received: \"{message['subject']}\" from {message['from']}")

        print("3. extracting signals (OTP codes / magic links) ...")
        signals = client.call_tool("extract_signals", {"id": message["id"]})
        print(f"   codes: {signals['codes']}  links: {signals['links']}")

        print("4. deleting email ...")
        client.call_tool("delete_email", {"id": message["id"]})
        print("   email deleted.")

        # --- SMS DEMO FLOW ---
        to_phone = "+15550100"
        from_phone = "+18005550199"
        sms_body = "Your 2FA authentication code is 940182."
        sms_since_ms = int(time.time() * 1000)

        print(f"\n--- SMS FLOW ---")
        print(f"5. simulating app sending 2FA SMS to {to_phone} ...")
        client.call_tool(
            "send_test_sms",
            {"to": to_phone, "from": from_phone, "body": sms_body},
        )

        print("6. waiting for SMS to arrive via long-polling (wait_for_sms) ...")
        sms_msg = client.call_tool(
            "wait_for_sms",
            {"to": to_phone, "since_ms": sms_since_ms, "timeout_ms": 10_000},
        )
        print(f"   received SMS from {sms_msg['from']}: \"{sms_msg['body']}\"")

        print("7. extracting OTP codes from SMS body ...")
        sms_signals = client.call_tool("extract_sms_signals", {"id": sms_msg["id"]})
        print(f"   extracted SMS codes: {sms_signals['codes']}")

        print("8. deleting SMS ...")
        client.call_tool("delete_sms", {"id": sms_msg["id"]})
        print("   SMS deleted. Demo complete!")

    finally:
        client.close()


if __name__ == "__main__":
    main()
