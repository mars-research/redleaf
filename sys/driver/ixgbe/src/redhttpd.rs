// redhttpd
//
// A very naive HTTP 1.1 server that supports serving multiple clients at
// once.

#[cfg(not(target_os = "linux"))]
use console::println;

#[cfg(not(target_os = "linux"))]
use alloc::vec;

extern crate arrayvec;
use arrayvec::ArrayVec;

use smoltcp::socket::{SocketSet, SocketHandle};
use smoltcp::socket::{Socket, TcpSocket, TcpSocketBuffer};
use httparse::BrokenRequest as HttpRequest;

#[macro_use]
use core::include_bytes;

// All sessions are pre-allocated in advance
const MAX_SESSIONS: usize = 2048;

/// Our dummy backing storage :)
fn read_file(url: &str) -> Option<&'static [u8]> {
    match url {
        "/index.html" | "/" | "" => Some(include_bytes!("htdocs/index.html")),
        "/404.html" => Some(include_bytes!("htdocs/404.html")),
        "/style.css" => Some(include_bytes!("htdocs/style.css")),
        _ => None,
    }
}


// Currently the design of smoltcp requires us to have multiple
// listening sockets to handle simultaneous connections. In other words,
// at least one HttpSession must be in the Listen state at any given time
// (unless we have already saturated MAX_SESSIONS) otherwise new clients
// will get refused.
//
// For now, let's simply create all the sockets up to MAX_SESSIONS
// at the start (we can do some fancy scaling later).

pub struct Httpd {
    sessions: ArrayVec<[HttpSession; MAX_SESSIONS]>,
}

impl Httpd {
    pub fn new() -> Self {
        Self {
            sessions: ArrayVec::new(),
        }
    }

    pub fn handle(&mut self, sockets: &mut SocketSet) {
        self.autoscale(sockets);

        for session in self.sessions.iter_mut() {
            session.handle(sockets);
        }
    }

    /// Auto-scales the HttpSessions
    fn autoscale(&mut self, sockets: &mut SocketSet) {
        // As mentioned above, let's just create all the sockets up
        // to MAX_SESSIONS :)

        while self.sessions.len() < self.sessions.capacity() {
            self.sessions.push(HttpSession::new(sockets));
        }
    }
}

/// An HTTP session.
///
/// A session is a state machine with 4 states:
/// [Idle] -> [Listen] -> [ReadRequest] -> [SendResponse] -
///   ^---------------------------------------------------|
///
/// A session holds a socket, and can be reused. After it's initialized,
/// it does not perform any dynamic allocations.
struct HttpSession {
    handle: SocketHandle,
    state: HttpState,
    response: Option<HttpResponse>,
}

struct HttpResponse {
    code: &'static [u8; 3],
    body: &'static [u8],
}

#[derive(PartialEq)]
enum HttpState {
    Idle,
    Listen,
    ReadRequest,
    SendResponseHeaders,
    SendResponseBody(usize),
    ToClose,
    Closing,
}

impl HttpSession {
    pub fn new<'a, 'b, 'c>(sockets: &mut SocketSet<'a, 'b, 'c>) -> Self {
        let socket = TcpSocket::new(
            TcpSocketBuffer::new(vec![0; 1024]),
            TcpSocketBuffer::new(vec![0; 1024]),
        );
        let socket: Socket<'b, 'c> = socket.into();

        let handle = sockets.add(socket);

        Self {
            handle,
            state: HttpState::Idle,
            response: None,
        }
    }

    /// Returns true if the connection has just established
    pub fn handle(&mut self, sockets: &mut SocketSet) {
        let mut socket = sockets.get::<TcpSocket>(self.handle);

        if !socket.is_active() && HttpState::Closing == self.state {
            self.state = HttpState::Idle;
            self.reset_state();
        }
        
        if !socket.is_active() && !socket.is_listening() {
            // It's possible that we were serving someone and the connection
            // closed. Anyways, let's reset the state machine and go home.

            socket.listen(80).unwrap();
            self.state = HttpState::Listen;
        }

        if socket.is_active() {
            // We have an established connection, but it's possible that we can't recv/send yet

            if let HttpState::Listen = self.state {
                self.state = HttpState::ReadRequest;
            }

            match self.state {
                HttpState::Closing => {},
                HttpState::ReadRequest => {
                    if socket.can_recv() {
                        socket.recv(|buf| {
                            let mut req = HttpRequest::new();
                            match req.parse(buf) {
                                Ok(_) => {
                                    if let Some(path) = req.path {
                                        if path.len() <= 250 {
                                            self.prepare_response(&path);
                                        } else {
                                            println!("Path too long: {}", &path);
                                            self.prepare_error(b"400");
                                        }
                                    }
                                },
                                Err(_) => {
                                    // Bad request
                                    self.prepare_error(b"400");
                                }
                            }
                            
                            (buf.len(), buf)
                        }).expect("Failed to receive");

                        self.state = HttpState::SendResponseHeaders;
                    }
                },
                HttpState::SendResponseHeaders => {
                    if socket.can_send() {
                        match &self.response {
                            Some(response) => {
                                socket.send(|mut buf| {
                                    // we can send at most buf.len() bytes
                                    let mut cursor = 0;
                                    let http_version = "HTTP/1.1 ".as_bytes();
                                    let ending = b" Kinda Worked?\r\n\r\n";
                                    
                                    buf[cursor..cursor + http_version.len()].copy_from_slice(http_version);
                                    cursor += http_version.len();

                                    buf[cursor..cursor + 3].copy_from_slice(response.code);
                                    cursor += 3;

                                    buf[cursor..cursor + ending.len()].copy_from_slice(ending);
                                    cursor += ending.len();

                                    (cursor, buf)
                                }).expect("Failed to send");

                                self.state = HttpState::SendResponseBody(0);
                            },
                            None => {
                                // Nothing to send!
                                self.state = HttpState::ToClose;
                            },
                        }
                    } else {
                        // Cannot send!
                        self.state = HttpState::ToClose;
                    }
                },
                HttpState::SendResponseBody(offset) => {
                    if socket.can_send() {
                        match &self.response {
                            Some(response) => {
                                let new_offset = socket.send(|mut buf| {
                                    // we can send at most buf.len() bytes
                                    let remaining = response.body.len() - offset;
                                    let to_send = core::cmp::min(buf.len(), remaining);

                                    buf[0..to_send].clone_from_slice(&response.body[offset..offset + to_send]);

                                    (to_send, offset + to_send)
                                }).expect("Failed to send");
                                
                                self.state = HttpState::SendResponseBody(new_offset);

                                if new_offset >= response.body.len() {
                                    // All done, going home
                                    self.state = HttpState::ToClose;
                                }
                            },
                            None => {
                                // Nothing to send!
                                self.state = HttpState::ToClose;
                            },
                        }
                    } else {
                        // Cannot send!
                        self.state = HttpState::ToClose;
                    }
                },
                _ => {
                    self.state = HttpState::ToClose;
                },
            }

            if let HttpState::ToClose = self.state {
                // Bye
                socket.close();
                self.state = HttpState::Closing;
            }
        } 
    }

    fn prepare_response(&mut self, path: &str) {
        if let Some(body) = read_file(path) {
            self.response = Some(HttpResponse {
                code: b"200",
                body,
            })
        } else if let Some(body) = read_file("/404.html") {
            self.response = Some(HttpResponse {
                code: b"404",
                body: body,
            })
        } else {
            self.response = Some(HttpResponse {
                code: b"404",
                body: "Meow?".as_bytes(),
            })
        }
    }

    fn prepare_error(&mut self, code: &'static [u8; 3]) {
        self.response = Some(HttpResponse {
            code,
            body: "Some kind of error occurred :(".as_bytes(),
        })
    }

    fn reset_state(&mut self) {
        self.response = None;
    }
}