use anyhow::anyhow;
use embedded_websocket::WebSocketClient;
use embedded_websocket::WebSocketOptions;
use rand::rngs::ThreadRng;
use std::net::TcpStream;

pub fn convert_connect_error(
    err: embedded_websocket::framer::FramerError<std::io::Error>,
) -> anyhow::Error {
    let err_anyhow = match err {
        embedded_websocket::framer::FramerError::Io(e) => anyhow!("{:?}", e),
        embedded_websocket::framer::FramerError::Utf8(e) => anyhow::Error::from(e),
        embedded_websocket::framer::FramerError::WebSocket(e) => anyhow!("{:?}", e),
        embedded_websocket::framer::FramerError::FrameTooLarge(n) => {
            anyhow::Error::msg(format!("Frame to large: {}", n))
        }
        embedded_websocket::framer::FramerError::HttpHeader(e) => anyhow!("{:?}", e),
    };
    return err_anyhow;
}
pub fn create_tcp_conn_and_client(
    addr: &str,
) -> anyhow::Result<(TcpStream, WebSocketOptions, WebSocketClient<ThreadRng>)> {
    let stream = TcpStream::connect(addr)?;
    let client = WebSocketClient::new_client(rand::thread_rng());
    let websocket_options = WebSocketOptions {
        path: "/subscribe",
        host: "",
        origin: "",
        sub_protocols: None,
        additional_headers: None,
    };
    return Ok((stream, websocket_options, client));
}
