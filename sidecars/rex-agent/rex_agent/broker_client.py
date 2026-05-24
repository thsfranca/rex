"""gRPC client for rex.v1 broker RPCs over the daemon UDS."""

from __future__ import annotations

import grpc

from rex.v1 import rex_pb2, rex_pb2_grpc


class RexBrokerClient:
    def __init__(self, socket_path: str) -> None:
        self._channel = grpc.aio.insecure_channel(f"unix://{socket_path}")
        self._stub = rex_pb2_grpc.RexServiceStub(self._channel)

    async def close(self) -> None:
        await self._channel.close()

    async def read_file(self, path: str) -> str:
        response = await self._stub.BrokerReadFile(rex_pb2.BrokerReadFileRequest(path=path))
        if not response.ok:
            raise RuntimeError(response.error or "read_file failed")
        return response.content

    async def list_dir(self, path: str = ".") -> list[str]:
        response = await self._stub.BrokerListDir(rex_pb2.BrokerListDirRequest(path=path))
        if not response.ok:
            raise RuntimeError(response.error or "list_dir failed")
        return list(response.entries)

    async def write_file(self, path: str, content: str) -> None:
        response = await self._stub.BrokerWriteFile(
            rex_pb2.BrokerWriteFileRequest(path=path, content=content)
        )
        if not response.ok:
            raise RuntimeError(response.error or "write_file failed")

    async def exec_shell(self, command: str) -> str:
        response = await self._stub.BrokerExecShell(rex_pb2.BrokerExecShellRequest(command=command))
        if not response.ok:
            raise RuntimeError(response.error or "exec_shell failed")
        parts = []
        if response.stdout:
            parts.append(f"stdout:\n{response.stdout}")
        if response.stderr:
            parts.append(f"stderr:\n{response.stderr}")
        return "\n".join(parts) if parts else "(no output)"

    async def inference(self, prompt: str, mode: str, model: str) -> str:
        response = await self._stub.BrokerInference(
            rex_pb2.BrokerInferenceRequest(prompt=prompt, mode=mode, model=model)
        )
        if not response.ok:
            raise RuntimeError(response.error or "inference failed")
        return response.text
