import sys
import socket

BUFFER_SIZE = 1024


def client(unix_socket, port):
    HOST = 'localhost'

    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as client:
        client.connect(unix_socket)
        try:
            with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as server:
                server.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
                server.bind((HOST, port))
                server.listen()
                while True:
                    conn, addr = server.accept()
                    with conn:
                        while True:
                            data = conn.recv(BUFFER_SIZE)
                            if not data:
                                break
                            client.sendall(data)
                            response = client.recv(BUFFER_SIZE)
                            conn.sendall(response)
                            conn.close()
                            break
        finally:
            client.shutdown(socket.SHUT_RDWR)
            client.close()


def parse_socket(socket_name: str):
    name = socket_name.split("/")[-1].split(".")[0]
    port = name.split("-")
    return name, port


def main():
    if len(sys.argv) == 1:
        print("ERROR: PATH to socket needed.")
        exit(1)
    unix_socket = sys.argv[1]
    name, port = parse_socket(unix_socket)
    client(unix_socket, port)


if __name__ == '__main__':
    main()
