#!/usr/bin/env python3
"""HermesOS Brain WebSocket Server (prototype)

ADR-008/ADR-009 uyumlu pull-based worker communication.
Worker'lar brain'e baglanir, hello gonderir, brain task dagitir.

Kullanim:
    .venv/bin/python scripts/brain-server.py [--port 8765]
"""

import argparse
import asyncio
import json
import secrets
import signal
import sys
import time
from pathlib import Path

try:
    import websockets
    from websockets.asyncio.server import serve
except ImportError:
    sys.exit("websockets kutuphanesi yok: .venv/bin/pip install websockets")

PROTOCOL_VERSION = 1
REGISTRY_PATH = Path(__file__).parent.parent / "config" / "worker_registry.json"


class WorkerConnection:
    def __init__(self, websocket):
        self.ws = websocket
        self.worker_id = None
        self.role = None
        self.capabilities = []
        self.last_heartbeat = 0.0
        self.pending_tasks: asyncio.Queue = asyncio.Queue()

    def info(self):
        return {
            "worker_id": self.worker_id,
            "role": self.role,
            "capabilities": self.capabilities,
            "last_heartbeat": self.last_heartbeat,
            "remote": str(getattr(self.ws, "remote_address", "?")),
        }


class Brain:
    def __init__(self, port):
        self.port = port
        self.workers: dict[str, WorkerConnection] = {}
        self.registry = self._load_registry()

    def _load_registry(self) -> dict:
        """Kalici worker kayitlari (ADR-009 enrollment kayitlari)."""
        if REGISTRY_PATH.exists():
            return json.loads(REGISTRY_PATH.read_text())
        return {"workers": {}}

    def _save_registry(self):
        REGISTRY_PATH.parent.mkdir(parents=True, exist_ok=True)
        REGISTRY_PATH.write_text(json.dumps(self.registry, indent=2))

    async def handle(self, websocket):
        conn = WorkerConnection(websocket)
        peer = getattr(websocket, "remote_address", ("?", 0))
        print(f"[brain] baglanti: {peer[0]}:{peer[1]}")

        try:
            async for raw in websocket:
                try:
                    msg = json.loads(raw)
                except json.JSONDecodeError:
                    await self._send(conn, {"type": "error", "message": "invalid json"})
                    continue

                mtype = msg.get("type")

                if mtype == "hello":
                    if msg.get("protocol_version") != PROTOCOL_VERSION:
                        await self._send(conn, {
                            "type": "reject",
                            "reason": f"desteklenmeyen protokol: {msg.get('protocol_version')}",
                        })
                        await websocket.close()
                        return

                    worker_id = msg.get("worker_id", "")
                    if not worker_id:
                        await self._send(conn, {"type": "reject", "reason": "worker_id bos"})
                        await websocket.close()
                        return

                    # ADR-009: kayitli worker mi? (ilk asamada open enrollment + log)
                    known = self.registry["workers"].get(worker_id)
                    if known is None:
                        token = secrets.token_urlsafe(24)
                        self.registry["workers"][worker_id] = {
                            "role": msg.get("role"),
                            "capabilities": msg.get("capabilities", []),
                            "enrolled_at": time.time(),
                            "enrollment_token": token,
                        }
                        self._save_registry()
                        print(f"[brain] YENI KAYIT: {worker_id} -> {known is None}")

                    conn.worker_id = worker_id
                    conn.role = msg.get("role")
                    conn.capabilities = msg.get("capabilities", [])
                    conn.last_heartbeat = time.time()
                    self.workers[worker_id] = conn

                    await self._send(conn, {
                        "type": "welcome",
                        "protocol_version": PROTOCOL_VERSION,
                        "brain_time": time.time(),
                        "tasks_pending": conn.pending_tasks.qsize(),
                    })
                    print(f"[brain] hello alindi: {worker_id} rol={conn.role} caps={conn.capabilities}")

                elif mtype == "heartbeat":
                    conn.last_heartbeat = time.time()
                    await self._send(conn, {"type": "heartbeat_ack", "brain_time": time.time()})

                elif mtype == "task_request":
                    # pull-based: worker is talep ediyor
                    capability = msg.get("capability")
                    task = {
                        "type": "task",
                        "task_id": f"task-{secrets.token_hex(4)}",
                        "capability": capability,
                        "input": msg.get("input", {}),
                    }
                    await self._send(conn, task)
                    print(f"[brain] task atandi: {task['task_id']} -> {conn.worker_id} ({capability})")

                elif mtype == "task_result":
                    ok = msg.get("success")
                    print(f"[brain] sonuc: {msg.get('task_id')} success={ok}")
                    print(f"         output: {json.dumps(msg.get('output'), ensure_ascii=False)[:200]}")
                    await self._send(conn, {"type": "task_ack", "task_id": msg.get("task_id")})

                elif mtype == "status_request":
                    await self._send(conn, {
                        "type": "status",
                        "workers": {w: c.info() for w, c in self.workers.items()},
                        "uptime": time.time(),
                    })

                else:
                    await self._send(conn, {"type": "error", "message": f"bilinmeyen tip: {mtype}"})

        except websockets.ConnectionClosed as e:
            print(f"[brain] kopdu: {conn.worker_id or peer} ({e})")
        finally:
            if conn.worker_id and self.workers.get(conn.worker_id) is conn:
                del self.workers[conn.worker_id]
                print(f"[brain] kayit silindi: {conn.worker_id}")

    @staticmethod
    async def _send(conn: WorkerConnection, payload: dict):
        await conn.ws.send(json.dumps(payload, ensure_ascii=False))

    async def run(self):
        async with serve(self.handle, "0.0.0.0", self.port):
            print(f"[brain] HermesOS Brain dinliyor: 0.0.0.0:{self.port}")
            print(f"[brain] registry: {REGISTRY_PATH}")
            await asyncio.Future()  # sonsuza dek


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--port", type=int, default=8765)
    args = ap.parse_args()
    brain = Brain(args.port)
    asyncio.run(brain.run())


if __name__ == "__main__":
    main()
