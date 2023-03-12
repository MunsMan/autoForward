import os
import sys
import socket

BUFFER_SIZE = 1024


def recv_all(socket):
    buffer = b''
    while True:
        data = socket.recv(BUFFER_SIZE)
        if not data:
            break
        buffer += data
    return buffer


def forward_TCP_request(port: int, request: bytes) -> bytes:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as client:
        client.connect(('localhost', port))
        client.sendall(request)
        return recv_all(client)


def server(unix_socket: str, port: int):
    try:
        os.unlink(unix_socket)
    except OSError:
        if os.path.exists(unix_socket):
            raise
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as server:
        server.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        server.bind(unix_socket)
        server.listen(0)
        while True:
            connection, client_address = server.accept()
            print(connection, client_address)
            try:
                while True:
                    request: bytes = connection.recv(BUFFER_SIZE)
                    if not request:
                        break
                    response = forward_TCP_request(port, request)
                    connection.sendall(response)
            finally:
                connection.shutdown(socket.SHUT_RDWR)
                connection.close()
    os.remove(unix_socket)
    print("SOCKET CLOSED")


def parse_socket(socket_name: str):
    name = socket_name.split("/")[-1].split(".")[0]
    port = int(name.split("-")[-1])
    return name, port


def main():
    if len(sys.argv) == 1:
        print("ERROR: PATH to socket needed.")
        exit(1)
    unix_socket = sys.argv[1]
    name, port = parse_socket(unix_socket)
    server(unix_socket, port)


if __name__ == "__main__":
    main()
