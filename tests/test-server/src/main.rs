use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

const PORT: u16 = 4221;

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind(format!("127.0.0.1:{}", PORT))
        .await
        .expect("failed to bind");

    println!("test-server listening on port {}", PORT);

    loop {
        let (mut socket, addr) = listener.accept().await.expect("failed to accept");
        println!("connection from {}", addr);

        tokio::spawn(async move {
            let mut buf = [0u8; 1024];
            let n = socket.read(&mut buf).await.unwrap_or(0);

            if n == 0 {
                return;
            }

            let request = String::from_utf8_lossy(&buf[..n]);
            let first_line = request.lines().next().unwrap_or("");
            println!("request: {}", first_line);

            // parse method and path from request line: "GET /path HTTP/1.1"
            let parts: Vec<&str> = first_line.split_whitespace().collect();
            let path = if parts.len() >= 2 { parts[1] } else { "/" };

            let response = match path {
                "/" => {
                    let body = "Hello, World!";
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                        body.len(),
                        body
                    )
                }
                _ => {
                    "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n".to_string()
                }
            };

            let _ = socket.write_all(response.as_bytes()).await;
        });
    }
}
