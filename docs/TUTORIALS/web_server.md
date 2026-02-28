# Tutorial: Building a Web Server

This tutorial guides you through building an HTTP web server in Knull, from basic request handling to routing and static file serving.

## Overview

By the end of this tutorial, you'll have a web server that:
- Parses HTTP requests
- Routes URLs to handlers
- Serves static files
- Handles form data and JSON
- Supports middleware

## Prerequisites

- Knull installed
- Understanding of HTTP protocol basics
- Basic knowledge of HTML/CSS (for testing)

## Part 1: HTTP Basics

### HTTP Request Format

```
GET /path HTTP/1.1
Host: localhost:8080
User-Agent: Mozilla/5.0
Accept: text/html

```

### HTTP Response Format

```
HTTP/1.1 200 OK
Content-Type: text/html
Content-Length: 123

<html>...</html>
```

## Part 2: Basic Server

### Step 1: Create the Project

```bash
knull new webserver
cd webserver
```

### Step 2: Parse HTTP Requests

Create `src/http.knull`:

```knull
mode expert

use std::collections::HashMap

enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
    HEAD,
    OPTIONS,
}

impl HttpMethod {
    fn from_str(s: &str) -> Option<HttpMethod> {
        match s {
            "GET" => Some(HttpMethod::GET),
            "POST" => Some(HttpMethod::POST),
            "PUT" => Some(HttpMethod::PUT),
            "DELETE" => Some(HttpMethod::DELETE),
            "HEAD" => Some(HttpMethod::HEAD),
            "OPTIONS" => Some(HttpMethod::OPTIONS),
            _ => None,
        }
    }
}

struct HttpRequest {
    method: HttpMethod,
    path: String,
    version: String,
    headers: HashMap<String, String>,
    body: String,
}

impl HttpRequest {
    fn parse(raw: &str) -> Option<HttpRequest> {
        let mut lines = raw.lines()
        
        // Parse request line
        let first_line = lines.next()?
        let parts: Vec<&str> = first_line.split_whitespace().collect()
        
        if parts.len() != 3 {
            return None
        }
        
        let method = HttpMethod::from_str(parts[0])?
        let path = parts[1].to_string()
        let version = parts[2].to_string()
        
        // Parse headers
        let mut headers = HashMap::new()
        for line in &mut lines {
            if line.is_empty() {
                break
            }
            
            if let Some(pos) = line.find(':') {
                let key = line[..pos].trim().to_lowercase()
                let value = line[pos + 1..].trim().to_string()
                headers.insert(key, value)
            }
        }
        
        // Parse body (remaining lines)
        let body = lines.collect::<Vec<_>>().join("\n")
        
        Some(HttpRequest {
            method,
            path,
            version,
            headers,
            body,
        })
    }
}

struct HttpResponse {
    status_code: u16,
    status_text: String,
    headers: HashMap<String, String>,
    body: String,
}

impl HttpResponse {
    fn new(status_code: u16) -> Self {
        let status_text = match status_code {
            200 => "OK",
            404 => "Not Found",
            500 => "Internal Server Error",
            400 => "Bad Request",
            301 => "Moved Permanently",
            302 => "Found",
            _ => "Unknown",
        }.to_string()
        
        let mut headers = HashMap::new()
        headers.insert("Content-Type".to_string(), "text/html".to_string())
        
        HttpResponse {
            status_code,
            status_text,
            headers,
            body: String::new(),
        }
    }
    
    fn with_body(mut self, body: String) -> Self {
        self.body = body
        self.headers.insert(
            "Content-Length".to_string(),
            self.body.len().to_string()
        )
        self
    }
    
    fn with_header(mut self, key: &str, value: &str) -> Self {
        self.headers.insert(key.to_string(), value.to_string())
        self
    }
    
    fn to_string(&self) -> String {
        let mut response = format!(
            "HTTP/1.1 {} {}\r\n",
            self.status_code,
            self.status_text
        )
        
        for (key, value) in &self.headers {
            response.push_str(&format!("{}: {}\r\n", key, value))
        }
        
        response.push_str("\r\n")
        response.push_str(&self.body)
        
        response
    }
}
```

### Step 3: Basic Server

Create `src/server.knull`:

```knull
mode expert

use std::net::{TcpListener, TcpStream}
use std::io::{Read, Write}
use std::thread

mod http

fn handle_request(request: HttpRequest) -> HttpResponse {
    match request.method {
        HttpMethod::GET => handle_get(&request),
        HttpMethod::POST => handle_post(&request),
        _ => HttpResponse::new(405)
            .with_body("Method Not Allowed".to_string()),
    }
}

fn handle_get(request: &HttpRequest) -> HttpResponse {
    match request.path.as_str() {
        "/" => HttpResponse::new(200)
            .with_body("<h1>Welcome to Knull Server!</h1>".to_string()),
        "/about" => HttpResponse::new(200)
            .with_body("<h1>About</h1><p>Built with Knull</p>".to_string()),
        _ => HttpResponse::new(404)
            .with_body("<h1>404 Not Found</h1>".to_string()),
    }
}

fn handle_post(request: &HttpRequest) -> HttpResponse {
    HttpResponse::new(200)
        .with_body(format!("Received: {}", request.body))
}

fn handle_client(mut stream: TcpStream) {
    let mut buffer = [0u8; 1024]
    
    match stream.read(&mut buffer) {
        Ok(n) => {
            let request_str = String::from_utf8_lossy(&buffer[..n])
            
            if let Some(request) = HttpRequest::parse(&request_str) {
                println!("{} {}", 
                    match request.method {
                        HttpMethod::GET => "GET",
                        HttpMethod::POST => "POST",
                        _ => "OTHER",
                    },
                    request.path
                )
                
                let response = handle_request(request)
                let response_str = response.to_string()
                
                stream.write_all(response_str.as_bytes()).unwrap()
            } else {
                let bad_request = HttpResponse::new(400)
                    .with_body("Bad Request".to_string())
                stream.write_all(bad_request.to_string().as_bytes()).unwrap()
            }
        }
        Err(e) => eprintln!("Error reading: {}", e),
    }
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080")
        .expect("Failed to bind")
    
    println!("Server running on http://127.0.0.1:8080")
    
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                thread::spawn(move || {
                    handle_client(stream)
                })
            }
            Err(e) => eprintln!("Error: {}", e),
        }
    }
}
```

## Part 3: Routing System

### Step 4: Add Router

Create `src/router.knull`:

```knull
mode expert

use std::collections::HashMap

type Handler = fn(&HttpRequest) -> HttpResponse

struct Router {
    routes: HashMap<String, Handler>,
    get_routes: HashMap<String, Handler>,
    post_routes: HashMap<String, Handler>,
}

impl Router {
    fn new() -> Self {
        Router {
            routes: HashMap::new(),
            get_routes: HashMap::new(),
            post_routes: HashMap::new(),
        }
    }
    
    fn get(mut self, path: &str, handler: Handler) -> Self {
        self.get_routes.insert(path.to_string(), handler)
        self
    }
    
    fn post(mut self, path: &str, handler: Handler) -> Self {
        self.post_routes.insert(path.to_string(), handler)
        self
    }
    
    fn route(&self, request: &HttpRequest) -> HttpResponse {
        let routes = match request.method {
            HttpMethod::GET => &self.get_routes,
            HttpMethod::POST => &self.post_routes,
            _ => return HttpResponse::new(405)
                .with_body("Method Not Allowed".to_string()),
        }
        
        match routes.get(&request.path) {
            Some(handler) => handler(request),
            None => HttpResponse::new(404)
                .with_body("Not Found".to_string()),
        }
    }
}
```

### Step 5: Update Server with Router

```knull
mode expert

fn home_handler(_request: &HttpRequest) -> HttpResponse {
    let html = r#"
<!DOCTYPE html>
<html>
<head>
    <title>Knull Server</title>
    <style>
        body { font-family: sans-serif; max-width: 800px; margin: 0 auto; padding: 20px; }
        h1 { color: #333; }
    </style>
</head>
<body>
    <h1>Welcome to Knull Web Server</h1>
    <p>Built with the Knull programming language</p>
    <ul>
        <li><a href="/">Home</a></li>
        <li><a href="/about">About</a></li>
        <li><a href="/api/status">API Status</a></li>
    </ul>
</body>
</html>
"#
    HttpResponse::new(200).with_body(html.to_string())
}

fn about_handler(_request: &HttpRequest) -> HttpResponse {
    let html = "<h1>About</h1><p>Built with Knull v1.0</p>"
    HttpResponse::new(200).with_body(html.to_string())
}

fn api_status_handler(_request: &HttpRequest) -> HttpResponse {
    let json = r#"{"status": "ok", "version": "1.0.0"}"#
    HttpResponse::new(200)
        .with_header("Content-Type", "application/json")
        .with_body(json.to_string())
}

fn main() {
    let router = Router::new()
        .get("/", home_handler)
        .get("/about", about_handler)
        .get("/api/status", api_status_handler)
        .post("/api/echo", |request| {
            HttpResponse::new(200)
                .with_header("Content-Type", "application/json")
                .with_body(format!("{{\"echo\": \"{}\"}}", request.body))
        })
    
    let listener = TcpListener::bind("127.0.0.1:8080")
        .expect("Failed to bind")
    
    println!("Server running on http://127.0.0.1:8080")
    
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let router = &router
                thread::spawn(move || {
                    handle_client_with_router(stream, router)
                })
            }
            Err(e) => eprintln!("Error: {}", e),
        }
    }
}

fn handle_client_with_router(mut stream: TcpStream, router: &Router) {
    // ... parse request ...
    let response = router.route(&request)
    // ... send response ...
}
```

## Part 4: Static File Serving

### Step 6: File Server

Create `src/static_files.knull`:

```knull
mode expert

use std::fs
use std::path::Path

struct StaticFileServer {
    root_dir: String,
}

impl StaticFileServer {
    fn new(root: &str) -> Self {
        StaticFileServer {
            root_dir: root.to_string(),
        }
    }
    
    fn serve(&self, path: &str) -> HttpResponse {
        // Sanitize path (prevent directory traversal)
        let sanitized = path.trim_start_matches('/')
            .replace("..", "")
            .replace("//", "/")
        
        let file_path = format!("{}/{}", self.root_dir, sanitized)
        let full_path = Path::new(&file_path)
        
        // Check if file exists and is file (not directory)
        if !full_path.exists() || !full_path.is_file() {
            return HttpResponse::new(404)
                .with_body("File not found".to_string())
        }
        
        // Read file
        match fs::read_to_string(&file_path) {
            Ok(content) => {
                let content_type = Self::guess_content_type(&file_path)
                
                HttpResponse::new(200)
                    .with_header("Content-Type", content_type)
                    .with_body(content)
            }
            Err(e) => {
                HttpResponse::new(500)
                    .with_body(format!("Error reading file: {}", e))
            }
        }
    }
    
    fn guess_content_type(path: &str) -> &str {
        if path.ends_with(".html") || path.ends_with(".htm") {
            "text/html"
        } else if path.ends_with(".css") {
            "text/css"
        } else if path.ends_with(".js") {
            "application/javascript"
        } else if path.ends_with(".json") {
            "application/json"
        } else if path.ends_with(".png") {
            "image/png"
        } else if path.ends_with(".jpg") || path.ends_with(".jpeg") {
            "image/jpeg"
        } else {
            "text/plain"
        }
    }
}
```

### Step 7: Directory Index

```knull
mode expert

impl StaticFileServer {
    fn serve_directory(&self, path: &str) -> HttpResponse {
        let dir_path = format!("{}/{}", self.root_dir, path)
        
        match fs::read_dir(&dir_path) {
            Ok(entries) => {
                let mut html = String::from(
                    "<!DOCTYPE html><html><head><title>Directory</title></head><body>"
                )
                html.push_str(&format!("<h1>Index of /{}</h1><ul>", path))
                
                for entry in entries {
                    if let Ok(entry) = entry {
                        let name = entry.file_name()
                            .to_string_lossy()
                            .to_string()
                        
                        let link = if entry.file_type().unwrap().is_dir() {
                            format!("<li><a href=\"{}/\">{}/</a></li>", name, name)
                        } else {
                            format!("<li><a href=\"{}\">{}</a></li>", name, name)
                        }
                        
                        html.push_str(&link)
                    }
                }
                
                html.push_str("</ul></body></html>")
                
                HttpResponse::new(200).with_body(html)
            }
            Err(_) => HttpResponse::new(500)
                .with_body("Cannot read directory".to_string()),
        }
    }
}
```

## Part 5: Request/Response Handling

### Step 8: Form Data Parsing

```knull
mode expert

fn parse_form_data(body: &str) -> HashMap<String, String> {
    let mut result = HashMap::new()
    
    for pair in body.split('&') {
        if let Some(pos) = pair.find('=') {
            let key = url_decode(&pair[..pos])
            let value = url_decode(&pair[pos + 1..])
            result.insert(key, value)
        }
    }
    
    result
}

fn url_decode(s: &str) -> String {
    // Simple URL decoding (production code should be more robust)
    s.replace("+", " ")
        .replace("%20", " ")
        .replace("%21", "!")
        .replace("%2F", "/")
    // ... more replacements ...
}
```

### Step 9: JSON Handling

```knull
mode expert

// Simple JSON parser (for production, use a library)
struct JsonValue {
    // Implementation details...
}

impl JsonValue {
    fn from_str(s: &str) -> Option<JsonValue> {
        // Parse JSON string...
        None  // Placeholder
    }
    
    fn to_string(&self) -> String {
        // Serialize to JSON...
        String::new()  // Placeholder
    }
}

fn api_handler(request: &HttpRequest) -> HttpResponse {
    if let Some(content_type) = request.headers.get("content-type") {
        if content_type.contains("application/json") {
            // Parse JSON body
            if let Some(json) = JsonValue::from_str(&request.body) {
                // Process JSON...
                return HttpResponse::new(200)
                    .with_header("Content-Type", "application/json")
                    .with_body(r#"{"status": "ok"}"#.to_string())
            }
        }
    }
    
    HttpResponse::new(400)
        .with_body("Expected JSON body".to_string())
}
```

## Part 6: Middleware

### Step 10: Middleware System

```knull
mode expert

type Middleware = fn(&HttpRequest, &HttpResponse) -> HttpResponse

struct MiddlewareChain {
    middlewares: Vec<Middleware>,
}

impl MiddlewareChain {
    fn new() -> Self {
        MiddlewareChain {
            middlewares: vec![],
        }
    }
    
    fn add(&mut self, middleware: Middleware) {
        self.middlewares.push(middleware)
    }
    
    fn execute(&self, request: &HttpRequest, response: HttpResponse) -> HttpResponse {
        let mut result = response
        
        for middleware in &self.middlewares {
            result = middleware(request, &result)
        }
        
        result
    }
}

// Example middleware: Add CORS headers
fn cors_middleware(_request: &HttpRequest, response: &HttpResponse) -> HttpResponse {
    response.clone()
        .with_header("Access-Control-Allow-Origin", "*")
        .with_header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
}

// Example middleware: Logging
fn logging_middleware(request: &HttpRequest, response: &HttpResponse) -> HttpResponse {
    println!("[{}] {} -> {}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        request.path,
        response.status_code
    )
    response.clone()
}
```

## Complete Example

Create `public/index.html`:

```html
<!DOCTYPE html>
<html>
<head>
    <title>Knull Server</title>
    <link rel="stylesheet" href="style.css">
</head>
<body>
    <h1>Welcome to Knull Web Server</h1>
    <p>This page is served by a Knull web server!</p>
    <form action="/submit" method="POST">
        <input type="text" name="message" placeholder="Enter message">
        <button type="submit">Submit</button>
    </form>
    <script src="app.js"></script>
</body>
</html>
```

Final server implementation:

```knull
mode expert

fn main() {
    let static_server = StaticFileServer::new("public")
    
    let mut middleware = MiddlewareChain::new()
    middleware.add(logging_middleware)
    middleware.add(cors_middleware)
    
    let router = Router::new()
        .get("/", |req| static_server.serve("index.html"))
        .get("/api/status", api_status_handler)
        .post("/submit", |req| {
            let form_data = parse_form_data(&req.body)
            let message = form_data.get("message")
                .unwrap_or(&"No message".to_string())
            
            HttpResponse::new(200)
                .with_body(format!("<h1>Received: {}</h1>", message))
        })
    
    // ... server setup ...
}
```

## Testing

```bash
# Start server
knull run src/server.knull

# Test with curl
curl http://localhost:8080/
curl http://localhost:8080/api/status
curl -X POST -d "message=hello" http://localhost:8080/submit

# Load testing
ab -n 10000 -c 100 http://localhost:8080/
```

## Exercises

1. Add template engine support
2. Implement session management with cookies
3. Add WebSocket support
4. Implement HTTP/2
5. Add TLS/HTTPS support

---

*This tutorial demonstrates building production-ready web servers in Knull. For real applications, consider using a framework.*
