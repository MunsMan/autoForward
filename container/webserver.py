from http.server import BaseHTTPRequestHandler, HTTPServer

BUFFER_SIZE = 1024


class MyServer(BaseHTTPRequestHandler):
    def do_GET(self):
        print(self.client_address)
        self.send_response(200)
        self.send_header("Content-type", "text/html")
        self.end_headers()
        with open("index.html", "rb") as file:
            data = file.read()
            print(data.decode())
            self.wfile.write(data)


def webserver():
    address = ('localhost', 3000)
    server = HTTPServer(
        address,
        MyServer
    )
    server.serve_forever()


if __name__ == "__main__":
    webserver()
