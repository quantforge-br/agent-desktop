"""Gives the nested compositor's seat a pointer for as long as this process runs.

A real desktop always has a mouse. The nested Hyprland has none: its host offers no input
devices, so the seat carries a pointer only while a product command holds its virtual
pointer, and every command would reach the clients as a brand-new pointer that enters their
surfaces without having left them. This process holds one zwlr_virtual_pointer_v1 the whole
session, which is what a plugged-in mouse looks like to the compositor and its clients.

It is also the hand on that mouse: a line "move X Y WIDTH HEIGHT" written to the FIFO named by
the first argument moves it to (X, Y) of a WIDTH x HEIGHT layout box, the way a person moving
the mouse does, so clients see the motion. A compositor warp moves the cursor without telling
any client, which leaves the surface under it believing the pointer never left.

It speaks the Wayland wire protocol directly, so it needs no client library.
"""

import os
import selectors
import socket
import struct
import sys
import time

DISPLAY_ID, REGISTRY_ID, CALLBACK_ID, MANAGER_ID, POINTER_ID = 1, 2, 3, 4, 5
MANAGER = "zwlr_virtual_pointer_manager_v1"
MOTION_ABSOLUTE, FRAME = 1, 4
PREFIX = "private_hyprland_seat_pointer:"


def message(object_id: int, opcode: int, payload: bytes = b"") -> bytes:
    return struct.pack("<II", object_id, ((8 + len(payload)) << 16) | opcode) + payload


def string(text: str) -> bytes:
    data = text.encode() + b"\0"
    return struct.pack("<I", len(data)) + data + b"\0" * (-len(data) % 4)


def read_string(payload: bytes, offset: int) -> tuple[str, int]:
    (length,) = struct.unpack_from("<I", payload, offset)
    start = offset + 4
    return payload[start:start + length - 1].decode(), start + length + (-length % 4)


class Connection:
    def __init__(self, path: str):
        self.socket = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.socket.connect(path)
        self.buffer = b""

    def send(self, data: bytes) -> None:
        self.socket.sendall(data)

    def receive(self):
        """Reads once and yields every whole message it completed; None at end of stream."""
        chunk = self.socket.recv(4096)
        if not chunk:
            return None
        self.buffer += chunk
        messages = []
        while len(self.buffer) >= 8:
            object_id, header = struct.unpack_from("<II", self.buffer)
            size = header >> 16
            if len(self.buffer) < size:
                break
            messages.append((object_id, header & 0xFFFF, self.buffer[8:size]))
            self.buffer = self.buffer[size:]
        return messages


def protocol_error(object_id: int, opcode: int, payload: bytes) -> bool:
    if object_id == DISPLAY_ID and opcode == 0:
        print(f"{PREFIX} protocol error {payload!r}", file=sys.stderr)
        return True
    return False


def bind_manager(connection: Connection) -> bool:
    connection.send(message(DISPLAY_ID, 1, struct.pack("<I", REGISTRY_ID))
                    + message(DISPLAY_ID, 0, struct.pack("<I", CALLBACK_ID)))
    manager = None
    while True:
        messages = connection.receive()
        if messages is None:
            return False
        for object_id, opcode, payload in messages:
            if protocol_error(object_id, opcode, payload):
                return False
            if object_id == REGISTRY_ID and opcode == 0:
                (name,) = struct.unpack_from("<I", payload)
                interface, offset = read_string(payload, 4)
                if interface == MANAGER:
                    manager = (name, min(struct.unpack_from("<I", payload, offset)[0], 2))
            if object_id == CALLBACK_ID and opcode == 0:
                if manager is None:
                    print(f"{PREFIX} the compositor offers no {MANAGER}", file=sys.stderr)
                    return False
                name, version = manager
                connection.send(
                    message(REGISTRY_ID, 0, struct.pack("<I", name) + string(MANAGER)
                            + struct.pack("<II", version, MANAGER_ID))
                    + message(MANAGER_ID, 0, struct.pack("<II", 0, POINTER_ID)))
                return True


def move(connection: Connection, line: str) -> None:
    words = line.split()
    try:
        if len(words) != 5 or words[0] != "move":
            raise ValueError("not a move")
        x, y, width, height = (int(word) for word in words[1:])
    except ValueError:
        print(f"{PREFIX} ignored {line!r}", file=sys.stderr, flush=True)
        return
    stamp = int(time.monotonic() * 1000) & 0xFFFFFFFF
    connection.send(message(POINTER_ID, MOTION_ABSOLUTE, struct.pack("<IIIII", stamp, x, y, width, height))
                    + message(POINTER_ID, FRAME))


def main() -> int:
    if len(sys.argv) != 2:
        print(f"usage: {sys.argv[0]} FIFO", file=sys.stderr)
        return 2
    fifo_path = sys.argv[1]
    os.mkfifo(fifo_path, 0o600)
    fifo = os.open(fifo_path, os.O_RDWR | os.O_NONBLOCK)
    connection = Connection(os.path.join(os.environ["XDG_RUNTIME_DIR"], os.environ["WAYLAND_DISPLAY"]))
    if not bind_manager(connection):
        return 1
    print(f"{PREFIX} ready", flush=True)
    selector = selectors.DefaultSelector()
    selector.register(connection.socket, selectors.EVENT_READ, "wayland")
    selector.register(fifo, selectors.EVENT_READ, "fifo")
    pending = b""
    while True:
        for key, _ in selector.select():
            if key.data == "wayland":
                messages = connection.receive()
                if messages is None:
                    return 0
                if any(protocol_error(*each) for each in messages):
                    return 1
                continue
            pending += os.read(fifo, 4096)
            *lines, pending = pending.split(b"\n")
            for line in lines:
                move(connection, line.decode())


if __name__ == "__main__":
    sys.exit(main())
