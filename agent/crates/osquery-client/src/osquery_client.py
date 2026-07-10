from __future__ import annotations
import asyncio
import logging
from pathlib import Path

from thrift.protocol.TBinaryProtocol import TBinaryProtocolAccelerated

from thrift.transport.TTransport import TMemoryBuffer
from thrift.Thrift import TMessageType, TType

from types import QueryResponse, QueryStatus

logger = logging.getLogger(__name__)


class OsqueryClient:

    def __init__(self, socket_path: Path):
        self.socket_path = Path(socket_path)

    @classmethod
    async def connect(cls, socket_path: Path):
        #for connect()
        logger.debug("Connecting to %s", socket_path)
        return cls(socket_path)

    async def query(self, sql: str) -> QueryResponse:
        """
        Execute a SQL query against osquery.
        """

        logger.debug("Executing query: %s", sql)

        request = self._serialize_request(sql)

        reader, writer = await asyncio.open_unix_connection(
            str(self.socket_path)
        )

        writer.write(request)
        await writer.drain()

        response = b"" # for storing binary data

        while True:

            chunk = await reader.read(4096)

            if not chunk:
                raise RuntimeError(
                    "Connection closed before response completed."
                )

            response += chunk

            try:
                result = self._parse_response(response)

                writer.close()
                await writer.wait_closed()

                return result

            except EOFError:
                continue

    def _serialize_request(self, sql: str) -> bytes:
        """
        Build the binary thrift request.
        """

        transport = TMemoryBuffer()

        protocol = TBinaryProtocolAccelerated(transport)

        protocol.writeMessageBegin("query",TMessageType.CALL,1)
        protocol.writeStructBegin("query_args")
        protocol.writeFieldBegin("sql",TType.STRING,1)
        protocol.writeString(sql)
        protocol.writeFieldEnd()
        protocol.writeFieldStop()
        protocol.writeStructEnd()
        protocol.writeMessageEnd()

        return transport.getvalue()

    def _parse_response(self, data: bytes) -> QueryResponse:
        """
        Parse binary thrift response.
        """

        transport = TMemoryBuffer(data)

        protocol = TBinaryProtocolAccelerated(transport)

        #
        # read_message_begin()
        # read_struct_begin()
        # read_field_begin()
        #
        # ...
        #

        return QueryResponse(
            status=QueryStatus(
                code=0,
                message="OK",
            ),
            rows=[],
        )

    async def live_query(self, sql: str):
        return await self.query(sql)

    async def ping(self):
        return

    async def reconnect(self):
        return
