//! Knull Standard Library - Real Networking Module
//!
//! Provides actual TCP/UDP socket operations for Knull programs.

use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream, UdpSocket};
use std::thread;
use std::time::Duration;

/// TCP connection handle
pub struct KnullTcpStream {
    stream: TcpStream,
}

impl KnullTcpStream {
    pub fn connect(addr: &str) -> Result<Self, String> {
        let stream = TcpStream::connect(addr)
            .map_err(|e| format!("Failed to connect to {}: {}", addr, e))?;

        stream
            .set_nonblocking(false)
            .map_err(|e| format!("Failed to set blocking mode: {}", e))?;

        Ok(KnullTcpStream { stream })
    }

    pub fn set_timeout(&mut self, millis: u64) -> Result<(), String> {
        let duration = Duration::from_millis(millis);
        self.stream
            .set_read_timeout(Some(duration))
            .map_err(|e| format!("Failed to set read timeout: {}", e))?;
        self.stream
            .set_write_timeout(Some(duration))
            .map_err(|e| format!("Failed to set write timeout: {}", e))?;
        Ok(())
    }

    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, String> {
        self.stream
            .read(buf)
            .map_err(|e| format!("Read error: {}", e))
    }

    pub fn read_to_string(&mut self) -> Result<String, String> {
        let mut buf = String::new();
        self.stream
            .read_to_string(&mut buf)
            .map_err(|e| format!("Read error: {}", e))?;
        Ok(buf)
    }

    pub fn write(&mut self, data: &[u8]) -> Result<usize, String> {
        self.stream
            .write(data)
            .map_err(|e| format!("Write error: {}", e))
    }

    pub fn write_all(&mut self, data: &[u8]) -> Result<(), String> {
        self.stream
            .write_all(data)
            .map_err(|e| format!("Write error: {}", e))
    }

    pub fn flush(&mut self) -> Result<(), String> {
        self.stream
            .flush()
            .map_err(|e| format!("Flush error: {}", e))
    }

    pub fn shutdown(&self, how: Shutdown) -> Result<(), String> {
        self.stream
            .shutdown(how)
            .map_err(|e| format!("Shutdown error: {}", e))
    }

    pub fn local_addr(&self) -> Result<String, String> {
        self.stream
            .local_addr()
            .map(|a| a.to_string())
            .map_err(|e| format!("Failed to get local address: {}", e))
    }

    pub fn peer_addr(&self) -> Result<String, String> {
        self.stream
            .peer_addr()
            .map(|a| a.to_string())
            .map_err(|e| format!("Failed to get peer address: {}", e))
    }
}

/// TCP server handle
pub struct KnullTcpListener {
    listener: TcpListener,
}

impl KnullTcpListener {
    pub fn bind(addr: &str) -> Result<Self, String> {
        let listener =
            TcpListener::bind(addr).map_err(|e| format!("Failed to bind to {}: {}", addr, e))?;

        listener
            .set_nonblocking(false)
            .map_err(|e| format!("Failed to set blocking mode: {}", e))?;

        Ok(KnullTcpListener { listener })
    }

    pub fn accept(&self) -> Result<(KnullTcpStream, String), String> {
        let (stream, addr) = self
            .listener
            .accept()
            .map_err(|e| format!("Accept error: {}", e))?;

        Ok((KnullTcpStream { stream }, addr.to_string()))
    }

    pub fn local_addr(&self) -> Result<String, String> {
        self.listener
            .local_addr()
            .map(|a| a.to_string())
            .map_err(|e| format!("Failed to get local address: {}", e))
    }

    pub fn incoming(&self) -> IncomingConnections {
        IncomingConnections { listener: self }
    }
}

pub struct IncomingConnections<'a> {
    listener: &'a KnullTcpListener,
}

impl<'a> Iterator for IncomingConnections<'a> {
    type Item = Result<(KnullTcpStream, String), String>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.listener.accept())
    }
}

/// UDP socket handle
pub struct KnullUdpSocket {
    socket: UdpSocket,
}

impl KnullUdpSocket {
    pub fn bind(addr: &str) -> Result<Self, String> {
        let socket =
            UdpSocket::bind(addr).map_err(|e| format!("Failed to bind to {}: {}", addr, e))?;
        Ok(KnullUdpSocket { socket })
    }

    pub fn connect(&self, addr: &str) -> Result<(), String> {
        self.socket
            .connect(addr)
            .map_err(|e| format!("Failed to connect to {}: {}", addr, e))
    }

    pub fn send(&self, data: &[u8]) -> Result<usize, String> {
        self.socket
            .send(data)
            .map_err(|e| format!("Send error: {}", e))
    }

    pub fn send_to(&self, data: &[u8], addr: &str) -> Result<usize, String> {
        self.socket
            .send_to(data, addr)
            .map_err(|e| format!("Send error: {}", e))
    }

    pub fn recv(&self, buf: &mut [u8]) -> Result<usize, String> {
        self.socket
            .recv(buf)
            .map_err(|e| format!("Recv error: {}", e))
    }

    pub fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, String), String> {
        let (size, addr) = self
            .socket
            .recv_from(buf)
            .map_err(|e| format!("Recv error: {}", e))?;
        Ok((size, addr.to_string()))
    }

    pub fn set_timeout(&self, millis: u64) -> Result<(), String> {
        let duration = Duration::from_millis(millis);
        self.socket
            .set_read_timeout(Some(duration))
            .map_err(|e| format!("Failed to set read timeout: {}", e))?;
        self.socket
            .set_write_timeout(Some(duration))
            .map_err(|e| format!("Failed to set write timeout: {}", e))?;
        Ok(())
    }

    pub fn local_addr(&self) -> Result<String, String> {
        self.socket
            .local_addr()
            .map(|a| a.to_string())
            .map_err(|e| format!("Failed to get local address: {}", e))
    }
}

/// HTTP client for simple requests
pub fn http_get(url: &str) -> Result<String, String> {
    // Parse simple URL (http://host:port/path)
    let url = url.trim_start_matches("http://");
    let parts: Vec<&str> = url.splitn(2, '/').collect();
    let host_port = parts[0];
    let path = if parts.len() > 1 { parts[1] } else { "" };

    let mut stream = KnullTcpStream::connect(host_port)?;

    let request = format!(
        "GET /{} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
        path, host_port
    );

    stream.write_all(request.as_bytes())?;
    stream.flush()?;

    let mut response = String::new();
    let mut buf = [0u8; 4096];

    loop {
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                response.push_str(&String::from_utf8_lossy(&buf[..n]));
            }
            Err(_) => break,
        }
    }

    Ok(response)
}

/// Simple HTTP server
pub fn start_http_server(addr: &str, handler: fn(&str) -> String) -> Result<(), String> {
    let listener = KnullTcpListener::bind(addr)?;
    println!("HTTP server listening on {}", addr);

    for conn in listener.incoming() {
        match conn {
            Ok((mut stream, peer)) => {
                println!("Connection from {}", peer);

                // Read request
                let mut buf = [0u8; 4096];
                if let Ok(n) = stream.read(&mut buf) {
                    let request = String::from_utf8_lossy(&buf[..n]);
                    let path = parse_http_path(&request);

                    // Generate response
                    let body = handler(&path);
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(), body
                    );

                    let _ = stream.write_all(response.as_bytes());
                    let _ = stream.flush();
                }
            }
            Err(e) => {
                eprintln!("Connection error: {}", e);
            }
        }
    }

    Ok(())
}

fn parse_http_path(request: &str) -> String {
    let lines: Vec<&str> = request.lines().collect();
    if let Some(first) = lines.first() {
        let parts: Vec<&str> = first.split_whitespace().collect();
        if parts.len() >= 2 {
            return parts[1].to_string();
        }
    }
    "/".to_string()
}

/// Network utilities
pub fn resolve_hostname(hostname: &str) -> Result<Vec<String>, String> {
    use std::net::ToSocketAddrs;

    let addrs: Vec<_> = (hostname, 0)
        .to_socket_addrs()
        .map_err(|e| format!("Failed to resolve {}: {}", hostname, e))?
        .map(|a| a.ip().to_string())
        .collect();

    Ok(addrs)
}

pub fn get_local_ip() -> Result<String, String> {
    // Connect to a public DNS server to determine local IP
    let socket =
        UdpSocket::bind("0.0.0.0:0").map_err(|e| format!("Failed to create socket: {}", e))?;

    socket
        .connect("8.8.8.8:80")
        .map_err(|e| format!("Failed to connect: {}", e))?;

    let local_addr = socket
        .local_addr()
        .map_err(|e| format!("Failed to get local address: {}", e))?;

    Ok(local_addr.ip().to_string())
}

/// Spawn a thread to handle connection
pub fn spawn_connection_handler<F>(handler: F) -> thread::JoinHandle<()>
where
    F: FnOnce() + Send + 'static,
{
    thread::spawn(handler)
}
