#!/usr/bin/env python3
"""Call one Ghidra MCP tool; store large results in the sibling research repo."""
import argparse
import asyncio
import json
import os
from pathlib import Path
from mcp import ClientSession
from mcp.client.streamable_http import streamable_http_client

async def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('tool', help='tool name, or --list')
    parser.add_argument('arguments', nargs='?', default='{}', help='JSON object')
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    async with streamable_http_client(os.environ.get('C6_MCP_URL', 'http://127.0.0.1:8765/mcp')) as streams:
        async with ClientSession(*streams, read_timeout_seconds=300) as client:
            await client.initialize()
            result = await client.list_tools() if args.tool == 'list-tools' else await client.call_tool(args.tool, json.loads(args.arguments))
            data = result.model_dump(mode='json', exclude_none=True)
            rendered = json.dumps(data, indent=2)
            if args.output:
                args.output.parent.mkdir(parents=True, exist_ok=True)
                args.output.write_text(rendered + '\n')
                print(json.dumps({'output':str(args.output), 'bytes':len(rendered), 'isError':data.get('is_error', data.get('isError', False))}))
            else:
                print(rendered)
            if data.get('is_error', data.get('isError', False)):
                raise SystemExit(1)

if __name__ == '__main__':
    asyncio.run(main())
