// redhttpd for usrnet
//
// A quick hack to support usrnet in redhttpd
//
// TODO: Figure out a cleanway to support both
// vanilla smoltcp and redleaf usrnet semantics.

#![no_std]
extern crate alloc;

#[cfg(not(target_os = "linux"))]
use console::println;

#[cfg(not(target_os = "linux"))]
use alloc::vec;

use core::fmt;
use core::fmt::Write;

use arrayvec::{ArrayVec, ArrayString};

use rref::RRefVec;
use usr::usrnet::UsrNet;

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

    pub fn handle(&mut self, net: &dyn UsrNet) {
        self.autoscale(net);

        for session in self.sessions.iter_mut() {
            session.handle(net);
        }
    }

    /// Auto-scales the HttpSessions
    fn autoscale(&mut self, net: &dyn UsrNet) {
        // As mentioned above, let's just create all the sockets up
        // to MAX_SESSIONS :)

        while self.sessions.len() < self.sessions.capacity() {
            self.sessions.push(HttpSession::new(net));
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

type SocketHandle = usize;

/// An HTTP session.
struct HttpSession {
    handle: SocketHandle,
    response: Option<HttpResponse>,
    state: HttpState,
    request_buf: ArrayVec<[u8; 1024]>,
    buf: Option<RRefVec<u8>>,
}


impl HttpSession {
    pub fn new(net: &dyn UsrNet) -> Self {
        // FIXME: Better error handling
        let handle = net.create().unwrap().unwrap();

        let r = HttpSession {
            handle,
            response: None,
            state: HttpState::ReadRequest,
            request_buf: ArrayVec::new(),
            buf: Some(RRefVec::new(0, 1024)),
        };

        r
    }

    fn buffer_request(&mut self, buf: &[u8]) -> bool {
        /*
        for chr in &buf[..] {
            self.request_buf.push(*chr);
        }
        */
        self.request_buf.try_extend_from_slice(buf).expect("Failed to write to request buffer");
        let buflen = self.request_buf.len();
        if self.request_buf.len() > 5 && &self.request_buf[buflen - 4..] == "\r\n\r\n".as_bytes() {
            return true;
        }
        return false;
    }

    fn read_request(&mut self, pbuf: &[u8]) -> (usize, bool) {
        if pbuf.len() == 0 {
            return (0, false);
        }

        let consumed = pbuf.len();

        // ok let's do request buffering...
        let buf = {
            if self.request_buf.len() == 0 && pbuf.len() > 5 && &pbuf[pbuf.len() - 4..] == "\r\n\r\n".as_bytes() {
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
                self.response = Self::emit_error(HttpStatus::NotFound);
            }
        }

        return (consumed, true);
    }

    pub fn handle(&mut self, net: &dyn UsrNet) {
        let active = net.is_active(self.handle).unwrap().unwrap();
        let listening = net.is_listening(self.handle).unwrap().unwrap();

        if !active {
            self.state = HttpState::ReadRequest;

            if self.request_buf.len() != 0 {
                self.request_buf.clear();
            }

            if !listening {
                net.listen(self.handle, 80).unwrap().unwrap();
            }
            return;
        }

        // Connection active
        match self.state {
            HttpState::ReadRequest => {
                if !net.can_recv(self.handle).unwrap().unwrap() {
                    return;
                }
                let buf = self.buf.take().unwrap();
                let (size, buf) = match net.read_socket(self.handle, buf).unwrap() {
                    Ok((size, buf)) => (size, buf),
                    Err(e) => {
                        // FIXME
                        println!("Read error: {:?}", e);
                        self.buf = Some(RRefVec::new(0, 1024));
                        return;
                    }
                };

                // now buf supposedly has valid data
                let (_, send_res) = self.read_request(&buf.as_slice()[..size]);
                self.buf.replace(buf);

                if send_res {
                    self.state = HttpState::SendResponse(false, 0, 0);
                    self.send_response(net);
                }
            }
            HttpState::SendResponse(_, _, _) => {
                // Continue sending response
                self.send_response(net);
            }
        }
    }

    fn send_response(&mut self, net: &dyn UsrNet) {
        if let HttpState::SendResponse(header_complete, header_sent, body_sent) = self.state {
            // We must have content-length to do keep-alive
            let response = self.response.as_ref().unwrap();

            if !header_complete {
                // let mut header = "HTTP/1.1 200 R\r\n\r\n".as_bytes();

                let mut header = ResponseHeader::new();
                let header = header.emit(response.body.len());

                // FIXME: This is ugly. Maybe use a closure
                let mut bufvec = self.buf.take().unwrap();
                let buf = bufvec.as_mut_slice();

                let remaining = header.len() - header_sent;
                let to_send = if remaining < buf.len() {
                    remaining
                } else {
                    buf.len()
                };

                let hslice = &header[header_sent..header_sent + to_send];
                buf[..hslice.len()].copy_from_slice(hslice);

                let (sent, bufvec) = net.write_socket(self.handle, bufvec, to_send).unwrap().unwrap();
                self.buf.replace(bufvec);

                let complete = sent == remaining;
                self.state = HttpState::SendResponse(complete, header_sent + sent, 0);

                if complete {
                    // Continue to send body
                    self.send_response(net);
                }
            } else {
                // Send body
                let body = response.body;

                let mut bufvec = self.buf.take().unwrap();
                let buf = bufvec.as_mut_slice();

                let remaining = body.len() - body_sent;
                let to_send = if remaining < buf.len() {
                    remaining
                } else {
                    buf.len()
                };

                let bslice = &body[body_sent..body_sent + to_send];
                buf[..bslice.len()].copy_from_slice(bslice);

                let (sent, bufvec) = net.write_socket(self.handle, bufvec, to_send).unwrap().unwrap();
                self.buf.replace(bufvec);

                let complete = sent == remaining;
                self.state = HttpState::SendResponse(complete, header_sent, body_sent + sent);

                if complete {
                    // We are done! Keep alive?
                    self.state = HttpState::ReadRequest;
                    self.request_buf.clear();
                }
            }
        } else {
            panic!("Invalid state");
        }
    }

    fn emit_error(error: HttpStatus) -> Option<HttpResponse> {
        match error {
            HttpStatus::NotFound => {
                Some(HttpResponse {
                    status: HttpStatus::NotFound,
                    body: read_file(b"/404.html").unwrap_or("Not Found".as_bytes()),
                })
            }
            HttpStatus::BadRequest => {
                Some(HttpResponse {
                    status: HttpStatus::BadRequest,
                    body: read_file(b"/400.html").unwrap_or("Bad Request".as_bytes()),
                })
            }
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
