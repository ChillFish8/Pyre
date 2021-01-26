import pyre_test


def cb(send, *args):
    send(
        False,
        b"HTTP/1.1 200 OK\r\n"
        b"Content-Length: 13\r\n"
        b"Server: Pyre\r\n"
        b"\r\n"
        b"Hello, World!"
    )
    print("wew")


pyre_test.create_server("127.0.0.1", 5050, cb, 5)