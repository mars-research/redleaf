// redhttpd
//
// A very naive HTTP 1.1 server that supports serving multiple clients at
// once.

#![no_std]
#![no_main]
#![forbid(unsafe_code)]
#![feature(const_fn, const_raw_ptr_to_usize_cast, untagged_unions)]

extern crate alloc;
extern crate malloc;
use alloc::boxed::Box;
use alloc::string::String;
use core::panic::PanicInfo;

use libsyscalls::syscalls::sys_println;
use syscalls::{Heap, Syscall};
use usr_interfaces::error::Result;
use usr_interfaces::vfs::{DirectoryEntry, DirectoryEntryRef, FileMode, INodeFileType};
use usr_interfaces::xv6::Xv6;
use usr_interfaces::usrnet::UsrNet;
use usrlib::syscalls::sys_sleep;
use usrlib::{eprintln, println};

use rref::RRefVec;

use alloc::vec;

use core::fmt;
use core::fmt::Write;

extern crate arrayvec;
use arrayvec::{ArrayVec, ArrayString};

#[macro_use]
use core::include_bytes;

// All sessions are pre-allocated in advance
const MAX_SESSIONS: usize = 128;

/// Our dummy backing storage :)
fn read_file(url: &str) -> Option<&'static [u8]> {
    match url {
        "/index.html" | "/" | "" => Some(include_bytes!("htdocs/index.html")),
        "/style.css" => Some(include_bytes!("htdocs/style.css")),
        "/404.html" => Some(include_bytes!("htdocs/404.html")),
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

/// An HTTP session.
struct HttpSession {
    handle: usize,
    response: Option<HttpResponse>,
    state: HttpState,
    buf: Option<RRefVec<u8>>,
}

impl HttpSession {
    // FIXME: better error handling - panics if socket cannot be created
    pub fn new(net: &dyn UsrNet) -> Self {
        let handle = net.listen(80).unwrap().unwrap();

        let r = HttpSession {
            handle,
            response: None,
            state: HttpState::ReadRequest,
            buf: Some(RRefVec::new(0, 1024)),
        };

        r

    }

    pub fn handle(&mut self, net: &dyn UsrNet) {
        if !net.is_usable(self.handle).unwrap().unwrap() {
            self.handle = net.listen(80).unwrap().unwrap();
            self.state = HttpState::ReadRequest;
        }

        if !net.is_active(self.handle).unwrap().unwrap() {
            return;
        }

        // Connection active
        match self.state {
            HttpState::ReadRequest => {
                let buf = self.buf.take().unwrap();
                let (size, buf) = net.read_socket(self.handle, buf).unwrap().unwrap();
                // self.buf.replace(buf);

                // well we are kind of dealing with a c array...
                // size is the actual size, not buf.len() *facepalm*

                if size == 0 {
                    self.buf.replace(buf);
                    return;
                }

                let buf = self.read_request(buf, size);
                self.buf.replace(buf);

                self.send_response(net);
            }
            HttpState::SendResponse(_, _, _) => {
                // Continue sending response
                self.send_response(net);
            }
        }
    }

    fn read_request(&mut self, mut buf: RRefVec<u8>, size: usize) -> RRefVec<u8> {
        let bufslice = &buf.as_mut_slice()[..size];

        if size < 5 {
            // Too short
            self.emit_error(HttpStatus::BadRequest);
            return buf;
        }
        if &bufslice[..4] != "GET ".as_bytes() || &bufslice[bufslice.len() - 4..] != "\r\n\r\n".as_bytes() {
            // Not GET, or request not complete in one
            // packet (against specs)
            self.emit_error(HttpStatus::BadRequest);
            return buf;
        }

        let mut urlend: usize = 4;
        for i in 4..bufslice.len() {
            if bufslice[i] == ' ' as u8 {
                urlend = i;
                break;
            }
        }
        if urlend == 4 {
            // No path
            self.emit_error(HttpStatus::BadRequest);
            return buf;
        }

        let url = if let Ok(url) = core::str::from_utf8(&bufslice[4..urlend]) {
            url
        } else {
            // Invalid UTF-8
            self.emit_error(HttpStatus::BadRequest);
            return buf;
        };

        match read_file(url) {
            Some(body) => {
                self.response = Some(HttpResponse {
                    status: HttpStatus::Success,
                    body,
                });
                self.state = HttpState::SendResponse(false, 0, 0);
            }
            None => {
                // 404
                self.emit_error(HttpStatus::NotFound);
            }
        }

        buf
    }

    fn send_response(&mut self, net: &dyn UsrNet) {
        if let HttpState::SendResponse(header_complete, header_sent, body_sent) = self.state {
            // We must have content-length to do keep-alive
            let response = self.response.as_ref().unwrap();

            if !header_complete {
                // let mut header = "HTTP/1.1 200 R\r\n\r\n".as_bytes();

                let mut header = ResponseHeader::new();
                let header = header.emit(response.body.len());

                let mut buf = self.buf.take().unwrap();
                let bufslice = buf.as_mut_slice();

                let remaining = header.len() - header_sent;
                let to_send = if remaining < bufslice.len() {
                    remaining
                } else {
                    bufslice.len()
                };

                let hslice = &header[header_sent..header_sent + to_send];
                bufslice[..hslice.len()].copy_from_slice(hslice);

                let (size, buf) = net.write_socket(self.handle, buf, to_send).unwrap().unwrap();
                self.buf.replace(buf);

                let complete = to_send == remaining;
                self.state = HttpState::SendResponse(complete, header_sent + to_send, 0);

                if complete {
                    // Continue to send body
                    self.send_response(net);
                }
            } else {
                // Send body
                let body = response.body;

                let mut buf = self.buf.take().unwrap();
                let bufslice = buf.as_mut_slice();

                let remaining = body.len() - body_sent;
                let to_send = if remaining < bufslice.len() {
                    remaining
                } else {
                    bufslice.len()
                };

                let bslice = &body[body_sent..body_sent + to_send];
                bufslice[..bslice.len()].copy_from_slice(bslice);

                let (size, buf) = net.write_socket(self.handle, buf, to_send).unwrap().unwrap();
                self.buf.replace(buf);

                let complete = to_send == remaining;
                self.state = HttpState::SendResponse(complete, header_sent, body_sent + to_send);

                if complete {
                    // We are done! Keep alive?
                    self.state = HttpState::ReadRequest;
                    // socket.close(); // FIXME
                }
            }
        } else {
            panic!("Invalid state");
        }
    }

    fn emit_error(&mut self, error: HttpStatus) {
        match error {
            HttpStatus::NotFound => {
                self.response = Some(HttpResponse {
                    status: HttpStatus::NotFound,
                    body: read_file("/404.html").unwrap_or("Not Found".as_bytes()),
                });
            }
            HttpStatus::BadRequest => {
                self.response = Some(HttpResponse {
                    status: HttpStatus::BadRequest,
                    body: read_file("/400.html").unwrap_or("Bad Request".as_bytes()),
                });
            }
            _ => unimplemented!(),
        }
        self.state = HttpState::SendResponse(false, 0, 0);
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
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        // FIXME: Bound (can panic here)
        self.buffer[self.len..self.len + s.len()].copy_from_slice(s.as_bytes());
        self.len += s.len();
        Ok(())
    }
}

#[no_mangle]
pub fn trusted_entry(
    s: Box<dyn Syscall + Send + Sync>,
    heap: Box<dyn Heap + Send + Sync>,
    rv6: Box<dyn Xv6>,
    args: &str,
) {
    libsyscalls::syscalls::init(s);
    rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id());
    usrlib::init(rv6.clone().unwrap());
    println!("Starting rv6 httpd with args: {}", args);

    main(rv6).unwrap();
}

fn main(rv6: Box<dyn Xv6>) -> Result<()> {
    let usrnet = rv6.get_usrnet()?;

    let mut httpd = Httpd::new();

    loop {
        httpd.handle(&*usrnet);
    }

    Ok(())
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    eprintln!("httpd panic: {:?}", info);
    libsyscalls::syscalls::sys_backtrace();
    loop {}
}
