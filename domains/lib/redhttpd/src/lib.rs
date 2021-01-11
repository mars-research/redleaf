// redhttpd
//
// A very naive HTTP 1.1 server that supports serving multiple clients at
// once.

#![no_std]

pub mod usrnet;

extern crate alloc;

#[cfg(not(target_os = "linux"))]
use console::println;

#[cfg(not(target_os = "linux"))]
use alloc::vec;

use core::fmt;
use core::fmt::Write;

use arrayvec::{ArrayString, ArrayVec};

use smoltcp::socket::{Socket, SocketRef, TcpSocket, TcpSocketBuffer};
use smoltcp::socket::{SocketHandle, SocketSet};

#[macro_use]
use core::include_bytes;

// All sessions are pre-allocated in advance
const MAX_SESSIONS: usize = 50;

/// Our dummy backing storage :)
fn read_file(url: &[u8]) -> Option<&'static [u8]> {
    match url {
        b"/index.html" | b"/" | b"" => Some(include_bytes!("htdocs/index.html")),
        b"/style.css" => Some(include_bytes!("htdocs/style.css")),
        b"/404.html" => Some(include_bytes!("htdocs/404.html")),
        b"/1K.bin" => Some(include_bytes!("htdocs/1K.bin")),
        b"/10K.bin" => Some(include_bytes!("htdocs/10K.bin")),
        b"/100K.bin" => Some(include_bytes!("htdocs/100K.bin")),
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

/*
    Sample HTTP/1.1 requests:

    GET /rustc-meowing.jpg HTTP/1.1[cr][lf]
    Host: redleaf.cat[cr][lf]
    User-Agent: Whatever[cr][lf]
    X-Moar-Headers: Yes[cr][lf]
    [cr][lf]

    POST /new-cat-pic HTTP/1.1[cr][lf]
    Content-Length: 1234[cr][lf]
    Connection: Keep-Alive[cr][lf]
    X-Need-to-Parse-Number: Annoying[cr][lf]
    [cr][lf]
    [1234 bytes of cat pic]
*/

/// The HTTP state.
enum HttpState {
    /// Reading request
    ReadRequest,

    /// Sending header + response
    SendResponse(bool, usize, usize),
}

#[non_exhaustive]
enum HttpStatus {
    Success,
    NotFound,
    BadRequest,
}

struct HttpResponse {
    status: HttpStatus,
    body: &'static [u8],
}

/// An HTTP session.
struct HttpSession {
    handle: SocketHandle,
    response: Option<HttpResponse>,
    state: HttpState,
    request_buf: ArrayVec<[u8; 1024]>,
    was_alive: bool,
}

impl HttpSession {
    pub fn new<'a, 'b, 'c>(sockets: &mut SocketSet<'a, 'b, 'c>) -> Self {
        let socket = TcpSocket::new(
            TcpSocketBuffer::new(vec![0; 2048]),
            TcpSocketBuffer::new(vec![0; 2048]),
        );
        let socket: Socket<'b, 'c> = socket.into();

        let handle = sockets.add(socket);

        let r = HttpSession {
            handle,
            response: None,
            state: HttpState::ReadRequest,
            request_buf: ArrayVec::new(),
            was_alive: true,
        };

        r
    }

    fn buffer_request(&mut self, buf: &[u8]) -> bool {
        /*
        for chr in &buf[..] {
            self.request_buf.push(*chr);
        }
        */
        self.request_buf
            .try_extend_from_slice(buf)
            .expect("Failed to write to request buffer");
        let buflen = self.request_buf.len();
        if self.request_buf.len() > 5 && &self.request_buf[buflen - 4..] == "\r\n\r\n".as_bytes() {
            return true;
        }
        return false;
    }

    pub fn handle(&mut self, sockets: &mut SocketSet) {
        let mut socket = sockets.get::<TcpSocket>(self.handle);

        if !socket.is_active() || !socket.is_open() {
            self.state = HttpState::ReadRequest;
            self.was_alive = false;

            if self.request_buf.len() != 0 {
                self.request_buf.clear();
            }

            if !socket.is_listening() {
                socket.listen(80).unwrap();
            }
            return;
        }

        // Connection active
        match self.state {
            HttpState::ReadRequest => {
                let id = self as *const HttpSession;
                if socket.may_recv() {
                    let send_res = socket
                        .recv(|pbuf| {
                            if pbuf.len() == 0 {
                                return (0, false);
                            }

                            /*
                            if self.request_buf.len() == 0 {
                                if self.was_alive {
                                    println!("{:X?}: Keep-Alive request", id);
                                } else {
                                    println!("{:X?}: New connection", id);
                                }
                            }
                            */

                            let consumed = pbuf.len();

                            // ok let's do request buffering...
                            let buf = {
                                if self.request_buf.len() == 0
                                    && pbuf.len() > 5
                                    && &pbuf[pbuf.len() - 4..] == "\r\n\r\n".as_bytes()
                                {
                                    // Cool, everything in one single packet
                                    &pbuf
                                } else if self.buffer_request(pbuf) {
                                    // Request complete!
                                    &*self.request_buf
                                } else {
                                    // Wait for next packet
                                    return (consumed, false);
                                }
                            };

                            let mut urlend: usize = 4;
                            for i in 4..buf.len() {
                                if buf[i] == ' ' as u8 {
                                    urlend = i;
                                    break;
                                }
                            }
                            if urlend == 4 {
                                // No path
                                self.response = Self::emit_error(HttpStatus::BadRequest);
                                panic!("Should not return 400 at all");
                                return (consumed, true);
                            }

                            let url = &buf[4..urlend];

                            /*
                            let url = if let Ok(url) = core::str::from_utf8(&buf[4..urlend]) {
                                url
                            } else {
                                // Invalid UTF-8
                                self.response = Self::emit_error(HttpStatus::BadRequest);
                                return (consumed, true);
                            };
                            */

                            match read_file(url) {
                                Some(body) => {
                                    self.response = Some(HttpResponse {
                                        status: HttpStatus::Success,
                                        body,
                                    });
                                }
                                None => {
                                    // 404
                                    panic!("Should not return 404 at all: {:?}", url);
                                    self.response = Self::emit_error(HttpStatus::NotFound);
                                }
                            }

                            return (consumed, true);
                        })
                        .expect("Failed to receive");

                    if send_res {
                        // println!("{:X?}: Transition to sendresponse", id);
                        self.state = HttpState::SendResponse(false, 0, 0);
                        self.send_response(&mut socket);
                    }
                }
            }
            HttpState::SendResponse(_, _, _) => {
                // Continue sending response
                self.send_response(&mut socket);
            }
        }
    }

    fn send_response(&mut self, socket: &mut SocketRef<TcpSocket>) {
        if !socket.may_send() {
            return;
        }

        if let HttpState::SendResponse(header_complete, header_sent, body_sent) = self.state {
            // We must have content-length to do keep-alive
            let response = self.response.as_ref().unwrap();

            if !header_complete {
                // let mut header = "HTTP/1.1 200 R\r\n\r\n".as_bytes();

                let mut header = ResponseHeader::new();
                let header = header.emit(response.body.len());

                let remaining_slice = &header[header_sent..];
                let sent = socket.send_slice(remaining_slice).expect("Failed to send");
                let new_cursor = header_sent + sent;

                if new_cursor == header.len() {
                    // Continue to send body
                    self.state = HttpState::SendResponse(true, 0, 0);
                    self.send_response(socket);
                } else {
                    self.state = HttpState::SendResponse(false, new_cursor, 0);
                }
            } else {
                // Send body
                let body = response.body;
                // let id = self as *const HttpSession;

                let remaining_slice = &body[body_sent..];

                let sent = socket.send_slice(remaining_slice).expect("Failed to send");
                let new_cursor = body_sent + sent;

                if new_cursor == body.len() {
                    // We are done! Keep alive?
                    // println!("keep-alive");
                    self.state = HttpState::ReadRequest;
                    self.was_alive = true;
                    self.request_buf.clear();
                } else {
                    self.state = HttpState::SendResponse(true, 0, new_cursor);
                }
            }
        } else {
            panic!("Invalid state");
        }
    }

    fn emit_error(error: HttpStatus) -> Option<HttpResponse> {
        match error {
            HttpStatus::NotFound => Some(HttpResponse {
                status: HttpStatus::NotFound,
                body: read_file(b"/404.html").unwrap_or("Not Found".as_bytes()),
            }),
            HttpStatus::BadRequest => Some(HttpResponse {
                status: HttpStatus::BadRequest,
                body: read_file(b"/400.html").unwrap_or("Bad Request".as_bytes()),
            }),
            _ => unimplemented!(),
        }
    }
}

struct ResponseHeader {
    buffer: [u8; 128],
    len: usize,
}

impl ResponseHeader {
    fn new() -> Self {
        // Might be heavy???
        let mut buffer = [0u8; 128];
        let th = "HTTP/1.1 200 R\r\nContent-Length: ".as_bytes();
        let len = th.len();
        buffer[..th.len()].copy_from_slice(th);
        Self { buffer, len }
    }

    fn emit(&mut self, content_length: usize) -> &[u8] {
        write!(self, "{}\r\n\r\n", content_length);
        &self.buffer[..self.len]
    }
}

impl Write for ResponseHeader {
    fn write_str(&mut self, s: &str) -> Result<(), fmt::Error> {
        // FIXME: Bound (can panic here)
        self.buffer[self.len..self.len + s.len()].copy_from_slice(s.as_bytes());
        self.len += s.len();
        Ok(())
    }
}
