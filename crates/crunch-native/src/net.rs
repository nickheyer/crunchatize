use std::io::ErrorKind;
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, TryRecvError, channel};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crunch::host::NetEvent;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{Error, Message, WebSocket};

const IDLE_SLEEP: Duration = Duration::from_millis(2);
const CLOSE_ABNORMAL: u16 = 1006;
const CLOSE_NO_STATUS: u16 = 1005;

// One WebSocket to the game server, driven on its own thread so connecting never stalls a frame
pub struct NetClient {
    outgoing: Sender<Vec<u8>>,
    incoming: Receiver<NetEvent>,
    closing: Arc<AtomicBool>,
}

impl NetClient {
    pub fn connect(url: String) -> Self {
        let (outgoing, to_socket) = channel::<Vec<u8>>();
        let (from_socket, incoming) = channel::<NetEvent>();
        let closing = Arc::new(AtomicBool::new(false));
        let stop = Arc::clone(&closing);
        thread::spawn(move || run_socket(&url, &to_socket, &from_socket, &stop));
        Self { outgoing, incoming, closing }
    }

    pub fn send(&self, data: &[u8]) {
        let _ = self.outgoing.send(data.to_vec());
    }

    pub fn poll(&self) -> Option<NetEvent> {
        self.incoming.try_recv().ok()
    }

    pub fn close(&self) {
        self.closing.store(true, Ordering::Relaxed);
    }
}

impl Drop for NetClient {
    fn drop(&mut self) {
        self.close();
    }
}

fn set_nonblocking(socket: &mut WebSocket<MaybeTlsStream<TcpStream>>) -> std::io::Result<()> {
    match socket.get_mut() {
        MaybeTlsStream::Plain(stream) => stream.set_nonblocking(true),
        MaybeTlsStream::Rustls(stream) => stream.get_mut().set_nonblocking(true),
        _ => Ok(()),
    }
}

fn would_block(error: &Error) -> bool {
    matches!(error, Error::Io(e) if e.kind() == ErrorKind::WouldBlock)
}

// Connects, then pumps frames both ways until either side closes
fn run_socket(url: &str, to_socket: &Receiver<Vec<u8>>, from_socket: &Sender<NetEvent>, closing: &AtomicBool) {
    let mut socket = match tungstenite::connect(url) {
        Ok((socket, _)) => socket,
        Err(e) => {
            let _ = from_socket.send(NetEvent::Closed { code: CLOSE_ABNORMAL, reason: e.to_string() });
            return;
        }
    };
    if let Err(e) = set_nonblocking(&mut socket) {
        let _ = from_socket.send(NetEvent::Closed { code: CLOSE_ABNORMAL, reason: e.to_string() });
        return;
    }
    let _ = from_socket.send(NetEvent::Open);
    loop {
        if closing.load(Ordering::Relaxed) {
            let _ = socket.close(None);
            let _ = socket.flush();
            return;
        }
        let mut idle = true;
        loop {
            match to_socket.try_recv() {
                Ok(data) => {
                    idle = false;
                    if let Err(e) = socket.send(Message::Binary(data.into()))
                        && !would_block(&e)
                    {
                        let _ = from_socket.send(NetEvent::Closed { code: CLOSE_ABNORMAL, reason: e.to_string() });
                        return;
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    let _ = socket.close(None);
                    let _ = socket.flush();
                    return;
                }
            }
        }
        if let Err(e) = socket.flush()
            && !would_block(&e)
        {
            let _ = from_socket.send(NetEvent::Closed { code: CLOSE_ABNORMAL, reason: e.to_string() });
            return;
        }
        match socket.read() {
            Ok(Message::Binary(bytes)) => {
                idle = false;
                let _ = from_socket.send(NetEvent::Message(bytes.to_vec()));
            }
            Ok(Message::Close(frame)) => {
                let (code, reason) = frame.map_or((CLOSE_NO_STATUS, String::new()), |f| (f.code.into(), f.reason.to_string()));
                let _ = from_socket.send(NetEvent::Closed { code, reason });
                return;
            }
            Ok(_) => idle = false,
            Err(e) if would_block(&e) => {}
            Err(Error::ConnectionClosed | Error::AlreadyClosed) => {
                let _ = from_socket.send(NetEvent::Closed { code: CLOSE_NO_STATUS, reason: String::new() });
                return;
            }
            Err(e) => {
                let _ = from_socket.send(NetEvent::Closed { code: CLOSE_ABNORMAL, reason: e.to_string() });
                return;
            }
        }
        if idle {
            thread::sleep(IDLE_SLEEP);
        }
    }
}
