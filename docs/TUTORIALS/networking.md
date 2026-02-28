# Tutorial: Building a TCP Chat Server

This tutorial walks you through building a multi-client TCP chat server in Knull.

## Overview

By the end of this tutorial, you'll have a chat server that:
- Accepts multiple simultaneous connections
- Broadcasts messages to all connected clients
- Handles client join/leave notifications
- Supports private messages

## Prerequisites

- Knull installed and configured
- Basic understanding of Knull syntax
- Familiarity with networking concepts

## Part 1: Basic TCP Server

### Step 1: Create the Project

```bash
knull new chat_server
cd chat_server
```

### Step 2: Single-Client Server

Create `src/server.knull`:

```knull
mode expert

use std::net::{TcpListener, TcpStream}
use std::io::{Read, Write, BufRead, BufReader}
use std::thread

fn handle_client(mut stream: TcpStream) {
    let addr = stream.peer_addr().unwrap()
    println!("New connection from: {}", addr)
    
    let mut reader = BufReader::new(&stream)
    let mut line = String::new()
    
    loop {
        line.clear()
        match reader.read_line(&mut line) {
            Ok(0) => {
                println!("Client {} disconnected", addr)
                break
            }
            Ok(_) => {
                let msg = line.trim()
                println!("Received from {}: {}", addr, msg)
                
                // Echo back
                let response = format!("Echo: {}\n", msg)
                stream.write_all(response.as_bytes()).unwrap()
                stream.flush().unwrap()
            }
            Err(e) => {
                eprintln!("Error reading from {}: {}", addr, e)
                break
            }
        }
    }
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080")
        .expect("Failed to bind to address")
    
    println!("Chat server listening on 127.0.0.1:8080")
    
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                thread::spawn(move || {
                    handle_client(stream)
                })
            }
            Err(e) => {
                eprintln!("Error accepting connection: {}", e)
            }
        }
    }
}
```

### Step 3: Test the Server

Run the server:
```bash
knull run src/server.knull
```

Test with netcat:
```bash
nc localhost 8080
Hello
```

## Part 2: Multi-Client Support

### Step 4: Shared State with Arc and Mutex

Update `src/server.knull`:

```knull
mode expert

use std::net::{TcpListener, TcpStream}
use std::io::{Read, Write, BufRead, BufReader}
use std::thread
use std::sync::{Arc, Mutex}
use std::collections::HashMap

struct ChatServer {
    clients: Arc<Mutex<HashMap<String, TcpStream>>>,
}

impl ChatServer {
    fn new() -> Self {
        ChatServer {
            clients: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    fn broadcast(&self, sender: &str, message: &str) {
        let clients = self.clients.lock().unwrap()
        let msg = format!("[{}] {}\n", sender, message)
        
        for (addr, stream) in clients.iter() {
            if addr != sender {
                let _ = stream.write_all(msg.as_bytes())
            }
        }
    }
    
    fn send_to(&self, recipient: &str, message: &str) -> Result<(), String> {
        let clients = self.clients.lock().unwrap()
        
        match clients.get(recipient) {
            Some(stream) => {
                stream.write_all(message.as_bytes())
                    .map_err(|e| e.to_string())
            }
            None => Err(format!("User {} not found", recipient)),
        }
    }
    
    fn add_client(&self, addr: String, stream: TcpStream) {
        let mut clients = self.clients.lock().unwrap()
        clients.insert(addr, stream.try_clone().unwrap())
    }
    
    fn remove_client(&self, addr: &str) {
        let mut clients = self.clients.lock().unwrap()
        clients.remove(addr)
    }
}

fn handle_client(
    server: Arc<ChatServer>,
    stream: TcpStream,
    addr: String
) {
    server.add_client(addr.clone(), stream.try_clone().unwrap())
    
    // Notify others
    server.broadcast(&addr, "has joined the chat")
    
    let mut reader = BufReader::new(&stream)
    let mut line = String::new()
    
    loop {
        line.clear()
        match reader.read_line(&mut line) {
            Ok(0) => {
                println!("Client {} disconnected", addr)
                break
            }
            Ok(_) => {
                let msg = line.trim()
                if msg.is_empty() {
                    continue
                }
                
                // Check for commands
                if msg.starts_with("/") {
                    handle_command(server.clone(), &addr, msg, &stream)
                } else {
                    // Broadcast to all
                    server.broadcast(&addr, msg)
                    println!("[{}] {}", addr, msg)
                }
            }
            Err(e) => {
                eprintln!("Error: {}", e)
                break
            }
        }
    }
    
    // Cleanup
    server.remove_client(&addr)
    server.broadcast(&addr, "has left the chat")
}

fn handle_command(
    server: Arc<ChatServer>,
    sender: &str,
    cmd: &str,
    stream: &TcpStream
) {
    let parts: Vec<&str> = cmd.split_whitespace().collect()
    
    match parts.get(0) {
        Some(&"/help") => {
            let help = "Commands:\n  /help - Show this message\n  /whisper <user> <msg> - Private message\n  /users - List connected users\n"
            let _ = stream.write_all(help.as_bytes())
        }
        Some(&"/whisper") if parts.len() >= 3 => {
            let recipient = parts[1]
            let msg = parts[2..].join(" ")
            let pm = format!("[PM from {}] {}\n", sender, msg)
            
            match server.send_to(recipient, &pm) {
                Ok(_) => {
                    let confirm = format!("[PM to {}] {}\n", recipient, msg)
                    let _ = stream.write_all(confirm.as_bytes())
                }
                Err(e) => {
                    let err = format!("Error: {}\n", e)
                    let _ = stream.write_all(err.as_bytes())
                }
            }
        }
        Some(&"/users") => {
            // Would list users here
            let _ = stream.write_all(b"User list feature coming soon\n")
        }
        _ => {
            let _ = stream.write_all(b"Unknown command. Type /help\n")
        }
    }
}

fn main() {
    let server = Arc::new(ChatServer::new())
    let listener = TcpListener::bind("127.0.0.1:8080")
        .expect("Failed to bind")
    
    println!("Chat server on 127.0.0.1:8080")
    println!("Clients can connect with: nc localhost 8080")
    
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let addr = stream.peer_addr()
                    .unwrap()
                    .to_string()
                
                let server_clone = Arc::clone(&server)
                thread::spawn(move || {
                    handle_client(server_clone, stream, addr)
                })
            }
            Err(e) => eprintln!("Error: {}", e),
        }
    }
}
```

## Part 3: Chat Client

### Step 5: Create a Client

Create `src/client.knull`:

```knull
mode expert

use std::net::TcpStream
use std::io::{self, Read, Write, BufRead, BufReader}
use std::thread

fn main() {
    println!("Connecting to server...")
    
    let stream = TcpStream::connect("127.0.0.1:8080")
        .expect("Failed to connect to server")
    
    println!("Connected! Type messages and press Enter")
    println!("Type /quit to exit\n")
    
    let mut stream_clone = stream.try_clone().unwrap()
    
    // Spawn thread to read from server
    thread::spawn(move || {
        let mut reader = BufReader::new(&stream_clone)
        let mut buffer = [0u8; 1024]
        
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => {
                    println!("\nServer closed connection")
                    break
                }
                Ok(n) => {
                    let msg = String::from_utf8_lossy(&buffer[..n])
                    print!("{}", msg)
                    io::stdout().flush().unwrap()
                }
                Err(_) => break,
            }
        }
    })
    
    // Read from stdin and send to server
    let stdin = io::stdin()
    let mut stdin_reader = stdin.lock()
    let mut line = String::new()
    
    loop {
        line.clear()
        stdin_reader.read_line(&mut line).unwrap()
        
        let trimmed = line.trim()
        if trimmed == "/quit" {
            break
        }
        
        stream.write_all(line.as_bytes()).unwrap()
        stream.flush().unwrap()
    }
    
    println!("Goodbye!")
}
```

## Part 4: Protocol Design

### Step 6: Structured Messages

Create `src/protocol.knull`:

```knull
mode expert

use std::collections::HashMap

enum Message {
    Chat { sender: String, content: String },
    Private { sender: String, recipient: String, content: String },
    Join { username: String },
    Leave { username: String },
    Command { sender: String, cmd: String, args: Vec<String> },
}

impl Message {
    fn serialize(&self) -> String {
        match self {
            Message::Chat { sender, content } => {
                format!("CHAT|{}|{}", sender, content)
            }
            Message::Private { sender, recipient, content } => {
                format!("PM|{}|{}|{}", sender, recipient, content)
            }
            Message::Join { username } => {
                format!("JOIN|{}", username)
            }
            Message::Leave { username } => {
                format!("LEAVE|{}", username)
            }
            Message::Command { sender, cmd, args } => {
                let args_str = args.join(",")
                format!("CMD|{}|{}|{}", sender, cmd, args_str)
            }
        }
    }
    
    fn deserialize(data: &str) -> Option<Message> {
        let parts: Vec<&str> = data.split('|').collect()
        
        match parts.get(0)? {
            "CHAT" if parts.len() >= 3 => Some(Message::Chat {
                sender: parts[1].to_string(),
                content: parts[2].to_string(),
            }),
            "PM" if parts.len() >= 4 => Some(Message::Private {
                sender: parts[1].to_string(),
                recipient: parts[2].to_string(),
                content: parts[3].to_string(),
            }),
            "JOIN" if parts.len() >= 2 => Some(Message::Join {
                username: parts[1].to_string(),
            }),
            "LEAVE" if parts.len() >= 2 => Some(Message::Leave {
                username: parts[1].to_string(),
            }),
            "CMD" if parts.len() >= 3 => {
                let args = if parts.len() > 3 {
                    parts[3].split(',').map(|s| s.to_string()).collect()
                } else {
                    vec![]
                }
                Some(Message::Command {
                    sender: parts[1].to_string(),
                    cmd: parts[2].to_string(),
                    args,
                })
            }
            _ => None,
        }
    }
}
```

## Part 5: Advanced Features

### Step 7: Add Nicknames

Update the server to support nicknames:

```knull
mode expert

use std::collections::HashMap
use std::net::TcpStream
use std::sync::{Arc, Mutex}

struct ClientInfo {
    stream: TcpStream,
    nickname: String,
}

struct ChatServer {
    clients: Arc<Mutex<HashMap<String, ClientInfo>>>,
}

impl ChatServer {
    fn set_nickname(&self, addr: &str, nickname: String) -> Result<(), String> {
        let mut clients = self.clients.lock().unwrap()
        
        // Check if nickname is taken
        for (_, info) in clients.iter() {
            if info.nickname == nickname {
                return Err("Nickname already in use".to_string())
            }
        }
        
        if let Some(info) = clients.get_mut(addr) {
            info.nickname = nickname
            Ok(())
        } else {
            Err("Client not found".to_string())
        }
    }
    
    fn get_nickname(&self, addr: &str) -> Option<String> {
        let clients = self.clients.lock().unwrap()
        clients.get(addr).map(|info| info.nickname.clone())
    }
    
    fn broadcast(&self, sender: &str, message: &str) {
        let clients = self.clients.lock().unwrap()
        
        let sender_name = self.get_nickname(sender)
            .unwrap_or_else(|| sender.to_string())
        
        let msg = format!("[{}] {}\n", sender_name, message)
        
        for (addr, info) in clients.iter() {
            if addr != sender {
                let _ = info.stream.write_all(msg.as_bytes())
            }
        }
    }
}
```

### Step 8: Rate Limiting

```knull
mode expert

use std::time::{Duration, Instant}
use std::collections::HashMap

struct RateLimiter {
    messages: HashMap<String, Vec<Instant>>,
    max_messages: usize,
    window: Duration,
}

impl RateLimiter {
    fn new(max_messages: usize, window_secs: u64) -> Self {
        RateLimiter {
            messages: HashMap::new(),
            max_messages,
            window: Duration::from_secs(window_secs),
        }
    }
    
    fn check(&mut self, client: &str) -> bool {
        let now = Instant::now()
        let window_start = now - self.window
        
        let timestamps = self.messages.entry(client.to_string())
            .or_insert_with(Vec::new)
        
        // Remove old timestamps
        timestamps.retain(|&t| t > window_start)
        
        // Check if under limit
        if timestamps.len() >= self.max_messages {
            return false
        }
        
        // Record this message
        timestamps.push(now)
        true
    }
}
```

## Testing

### Unit Tests

Create `tests/server_test.knull`:

```knull
mode expert

#[test]
fn test_message_serialize() {
    let msg = Message::Chat {
        sender: "Alice".to_string(),
        content: "Hello!".to_string(),
    }
    
    let serialized = msg.serialize()
    assert_eq!(serialized, "CHAT|Alice|Hello!")
    
    let deserialized = Message::deserialize(&serialized).unwrap()
    match deserialized {
        Message::Chat { sender, content } => {
            assert_eq!(sender, "Alice")
            assert_eq!(content, "Hello!")
        }
        _ => panic!("Wrong message type"),
    }
}

#[test]
fn test_rate_limiter() {
    let mut limiter = RateLimiter::new(3, 60)
    
    assert!(limiter.check("user1"))  // 1st message - OK
    assert!(limiter.check("user1"))  // 2nd message - OK
    assert!(limiter.check("user1"))  // 3rd message - OK
    assert!(!limiter.check("user1")) // 4th message - BLOCKED
}
```

## Running the Server

### Development Mode

```bash
knull run src/server.knull
```

### Production Mode

```bash
knull build --release src/server.knull
./chat_server
```

## Exercises

1. **Add Message History**: Store last 100 messages and send to new clients
2. **Add Rooms**: Support multiple chat channels/rooms
3. **Add Authentication**: Simple password or token-based auth
4. **Add File Sharing**: Allow sending small files between clients
5. **Add Logging**: Log all messages to file

## Common Issues

### Connection Refused
- Check server is running
- Verify port number matches
- Check firewall settings

### Port Already in Use
```bash
# Find process using port
lsof -i :8080
# Kill process
kill <PID>
```

### Thread Panic
- Check mutex isn't poisoned
- Ensure proper error handling
- Verify all unwrap() calls

## Next Steps

- WebSocket support for browser clients
- TLS/SSL encryption
- Database persistence
- Admin commands and moderation
- Bot framework

---

*This tutorial covers the fundamentals of network programming in Knull. The complete source code is available in the examples directory.*
