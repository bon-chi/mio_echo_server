extern crate mio;

use mio::*;
use mio::tcp::*;
use mio::util::Slab;
use std::net::SocketAddr;
use std::str::FromStr;
use std::mem;

struct Server {
    socket: TcpListener,
    connections: Slab<Connection>,
}

impl Server {
    fn new(socket: TcpListener) -> Server {
        Server {
            socket: socket,
            connections: Slab::new_starting_at(mio::Token(1), 1024),
        }
    }
    fn conn_readable(&mut self, event_loop: &mut EventLoop<Server>, token: Token) {
        // self.connections[token].readable(event_loop);
        self.connections[token].read(event_loop);
    }
}

impl Handler for Server {
    type Timeout = ();
    type Message = ();

    fn ready(&mut self, event_loop: &mut EventLoop<Server>, token: Token, events: EventSet) {
        match token {
            mio::Token(0) => {
                match self.socket.accept() {
                    Ok(Some(socket_token)) => {
                        match socket_token {
                            (socket, _addr) => {
                                println!("accepted a new client socket");
                                let token = self.connections
                                                .insert_with(|token| {
                                                    Connection::new(socket, token)
                                                })
                                                .unwrap();
                                event_loop.register(&self.connections[token].socket,
                                                    token,
                                                    EventSet::readable(),
                                                    PollOpt::edge() | PollOpt::oneshot())
                                          .unwrap();
                            }
                        }
                    }
                    Ok(None) => {
                        println!("the server socket wasn't actually ready");
                    }
                    Err(e) => {
                        println!("encountered error while acceptiong connection; err={:?}", e);
                        event_loop.shutdown();
                    }
                }
            }
            i => {
                if events.is_readable() {
                    self.conn_readable(event_loop, i);
                }
            }
        }
    }
}

struct Connection {
    socket: TcpStream,
    token: Token,
    state: State,
}

impl Connection {
    fn new(socket: TcpStream, token: Token) -> Connection {
        Connection {
            socket: socket,
            token: token,
            state: State::Reading(vec![]),
        }
    }
    fn read(&mut self, event_loop: &mut EventLoop<Server>) {
        match self.socket.try_read_buf(self.state.mut_read_buf()) {
            Ok(Some(n)) => {
                println!("read {} bytes", n);
                self.state.try_transition_to_writing();
                self.reregister(event_loop);
            }
            Ok(None) => {}
            Err(e) => {}
        }
    }

    fn reregister(&self, event_loop: &mut EventLoop<Server>) {
        let event_set = match self.state {
            State::Reading(..) => EventSet::readable(),
            _ => {}
        };
        event_loop.reregister(&self.socket, self.token, event_set, mio::PollOpt::oneshot())
                  .unwrap();
    }
}

enum State {
    Reading(Vec<u8>),
    Closed,
}
impl State {
    fn try_transition_to_writing(&self) {
        if let Some(pos) = self.read_buf().iter().position(|b| *b == b'\n') {
            self.transition_to_writingg(pos + 1);
        }
    }

    fn transition_to_writingg(&mut self, pos: usize) {
        let buf = mem::replace(self, State::Closed).unwrap_read_buf();
    }

    fn unwrap_read_buf(self) -> Vec<u8> {}
    fn read_buf(&self) -> &[u8] {
        match *self {
            State::Reading(ref buf) => buf,
            _ => panic!("connection not in reading state"),
        }
    }
    fn mut_read_buf(&mut self) -> &mut Vec<u8> {
        match *self {
            State::Reading(ref mut buf) => buf,
            // _ => panic!("connection not in reading state"),
        }
    }
}

fn start(socket_addr: SocketAddr) {
    let socket = TcpListener::bind(&socket_addr).unwrap();
    let mut event_loop: EventLoop<Server> = mio::EventLoop::new().unwrap();
    event_loop.register(&socket,
                        mio::Token(0),
                        EventSet::readable(),
                        PollOpt::edge())
              .unwrap();
    let mut server = Server::new(socket);
    event_loop.run(&mut server).unwrap();
}
fn main() {
    start(SocketAddr::from_str("127.0.0.1:5555").unwrap());
}
