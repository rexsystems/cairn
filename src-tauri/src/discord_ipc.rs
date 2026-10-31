//! Discord's local IPC socket (a named pipe on Windows), used both to set activity and to get join events.
//! Reads never block; on Windows that takes PeekNamedPipe, since a blocking pipe read also stalls writes.

use serde_json::{json, Value};
use std::io::{self, Read, Write};
use std::time::{Duration, Instant};

pub const OP_HANDSHAKE: u32 = 0;
pub const OP_FRAME: u32 = 1;
pub const OP_CLOSE: u32 = 2;
pub const OP_PING: u32 = 3;
pub const OP_PONG: u32 = 4;

/// Discord's own frames are a few KB; anything far larger means a broken stream.
const MAX_FRAME: usize = 64 * 1024;
const READY_WAIT: Duration = Duration::from_secs(5);

pub fn frame(op: u32, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(8 + payload.len());
    out.extend_from_slice(&op.to_le_bytes());
    out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    out.extend_from_slice(payload);
    out
}

/// Collects bytes from the pipe and hands back whole frames.
#[derive(Default)]
pub struct Frames {
    buf: Vec<u8>,
}

impl Frames {
    pub fn push(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
    }

    pub fn pop(&mut self) -> io::Result<Option<(u32, Value)>> {
        if self.buf.len() < 8 {
            return Ok(None);
        }
        let op = u32::from_le_bytes(self.buf[0..4].try_into().unwrap());
        let len = u32::from_le_bytes(self.buf[4..8].try_into().unwrap()) as usize;
        if len > MAX_FRAME {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Discord sent an oversized frame"));
        }
        if self.buf.len() < 8 + len {
            return Ok(None);
        }
        let body: Vec<u8> = self.buf.drain(..8 + len).skip(8).collect();
        let v = serde_json::from_slice(&body).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        Ok(Some((op, v)))
    }
}

#[cfg(unix)]
mod pipe {
    use super::*;
    use std::os::unix::net::UnixStream;
    use std::path::PathBuf;

    pub struct Pipe(UnixStream);

    fn candidates() -> Vec<PathBuf> {
        let mut roots: Vec<PathBuf> = ["XDG_RUNTIME_DIR", "TMPDIR", "TMP", "TEMP"].iter().filter_map(std::env::var_os).map(PathBuf::from).collect();
        roots.push("/tmp".into());
        // Flatpak and Snap builds of Discord put the socket in their own runtime folders.
        let subs = ["", "app/com.discordapp.Discord", "app/com.discordapp.DiscordCanary", ".flatpak/com.discordapp.Discord/xdg-run", "snap.discord"];
        let mut out = vec![];
        for r in &roots {
            for s in subs {
                for i in 0..10 {
                    out.push(r.join(s).join(format!("discord-ipc-{i}")));
                }
            }
        }
        out
    }

    impl Pipe {
        pub fn open() -> io::Result<Pipe> {
            for p in candidates() {
                if let Ok(s) = UnixStream::connect(&p) {
                    s.set_nonblocking(true)?;
                    return Ok(Pipe(s));
                }
            }
            Err(io::Error::new(io::ErrorKind::NotFound, "Discord isn't running"))
        }

        pub fn read_available(&mut self, out: &mut Vec<u8>) -> io::Result<()> {
            let mut chunk = [0u8; 4096];
            loop {
                match self.0.read(&mut chunk) {
                    Ok(0) => return Err(io::ErrorKind::ConnectionAborted.into()),
                    Ok(n) => out.extend_from_slice(&chunk[..n]),
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => return Ok(()),
                    Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                    Err(e) => return Err(e),
                }
            }
        }

        pub fn write_all(&mut self, bytes: &[u8]) -> io::Result<()> {
            self.0.set_nonblocking(false)?;
            let r = self.0.write_all(bytes);
            self.0.set_nonblocking(true)?;
            r
        }
    }
}

#[cfg(windows)]
mod pipe {
    use super::*;
    use std::fs::File;
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::System::Pipes::PeekNamedPipe;

    pub struct Pipe(File);

    impl Pipe {
        pub fn open() -> io::Result<Pipe> {
            for i in 0..10 {
                if let Ok(f) = std::fs::OpenOptions::new().read(true).write(true).open(format!(r"\\.\pipe\discord-ipc-{i}")) {
                    return Ok(Pipe(f));
                }
            }
            Err(io::Error::new(io::ErrorKind::NotFound, "Discord isn't running"))
        }

        pub fn read_available(&mut self, out: &mut Vec<u8>) -> io::Result<()> {
            loop {
                let mut avail: u32 = 0;
                // SAFETY: the handle belongs to self.0 and stays open for the call; the null
                // buffer with size 0 only asks how many bytes are waiting.
                let ok = unsafe { PeekNamedPipe(self.0.as_raw_handle() as _, std::ptr::null_mut(), 0, std::ptr::null_mut(), &mut avail, std::ptr::null_mut()) };
                if ok == 0 {
                    return Err(io::Error::last_os_error());
                }
                if avail == 0 {
                    return Ok(());
                }
                let mut chunk = vec![0u8; (avail as usize).min(64 * 1024)];
                let n = self.0.read(&mut chunk)?;
                if n == 0 {
                    return Err(io::ErrorKind::ConnectionAborted.into());
                }
                out.extend_from_slice(&chunk[..n]);
            }
        }

        pub fn write_all(&mut self, bytes: &[u8]) -> io::Result<()> {
            self.0.write_all(bytes)
        }
    }
}

pub struct Conn {
    pipe: pipe::Pipe,
    frames: Frames,
    /// Messages that arrived together with READY.
    early: Vec<Value>,
}

impl Conn {
    /// Connects and completes the handshake. Fails if Discord is closed or rejects the app id.
    pub fn connect(app_id: &str) -> io::Result<Conn> {
        let mut c = Conn { pipe: pipe::Pipe::open()?, frames: Frames::default(), early: vec![] };
        c.send(OP_HANDSHAKE, &json!({ "v": 1, "client_id": app_id }))?;
        let deadline = Instant::now() + READY_WAIT;
        while Instant::now() < deadline {
            let got = c.poll()?;
            if let Some(i) = got.iter().position(|v| v.get("evt").and_then(|e| e.as_str()) == Some("READY")) {
                c.early = got[i + 1..].to_vec();
                return Ok(c);
            }
            std::thread::sleep(Duration::from_millis(25));
        }
        Err(io::Error::new(io::ErrorKind::TimedOut, "Discord didn't answer the handshake"))
    }

    pub fn send(&mut self, op: u32, v: &Value) -> io::Result<()> {
        self.pipe.write_all(&frame(op, v.to_string().as_bytes()))
    }

    /// Sends one RPC command. Replies come back through poll().
    pub fn command(&mut self, cmd: &str, args: Value, evt: Option<&str>) -> io::Result<()> {
        let mut v = json!({ "cmd": cmd, "args": args, "nonce": uuid::Uuid::new_v4().to_string() });
        if let Some(e) = evt {
            v["evt"] = json!(e);
        }
        self.send(OP_FRAME, &v)
    }

    /// Reads whatever has arrived without waiting. Answers pings; a close frame ends the connection.
    pub fn poll(&mut self) -> io::Result<Vec<Value>> {
        let mut bytes = vec![];
        self.pipe.read_available(&mut bytes)?;
        self.frames.push(&bytes);
        let mut out = std::mem::take(&mut self.early);
        while let Some((op, v)) = self.frames.pop()? {
            match op {
                OP_FRAME => out.push(v),
                OP_PING => self.send(OP_PONG, &v)?,
                OP_CLOSE => {
                    let msg = v.get("message").and_then(|m| m.as_str()).unwrap_or("Discord closed the connection").to_string();
                    return Err(io::Error::new(io::ErrorKind::ConnectionAborted, msg));
                }
                _ => {}
            }
        }
        Ok(out)
    }

    pub fn close(mut self) {
        let _ = self.send(OP_CLOSE, &json!({}));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_split_across_reads_reassemble() {
        let a = frame(OP_FRAME, br#"{"evt":"READY"}"#);
        let b = frame(OP_PING, br#"{"n":1}"#);
        let split = a.len();
        let all = [a, b].concat();
        let mut f = Frames::default();
        f.push(&all[..5]);
        assert!(f.pop().unwrap().is_none());
        f.push(&all[5..split + 3]);
        assert_eq!(f.pop().unwrap().unwrap(), (OP_FRAME, json!({"evt":"READY"})));
        assert!(f.pop().unwrap().is_none());
        f.push(&all[split + 3..]);
        assert_eq!(f.pop().unwrap().unwrap(), (OP_PING, json!({"n":1})));
        assert!(f.pop().unwrap().is_none());
    }

    #[test]
    fn oversized_or_garbled_frames_are_errors() {
        let mut f = Frames::default();
        f.push(&frame(OP_FRAME, &[]).iter().take(4).copied().chain((MAX_FRAME as u32 + 1).to_le_bytes()).collect::<Vec<_>>());
        assert!(f.pop().is_err());
        let mut f = Frames::default();
        f.push(&frame(OP_FRAME, b"not json"));
        assert!(f.pop().is_err());
    }

    #[cfg(unix)]
    #[test]
    fn handshake_and_ping_over_a_real_socket() {
        use std::os::unix::net::UnixListener;
        let dir = std::env::temp_dir().join(format!("cairn-ipc-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let listener = UnixListener::bind(dir.join("discord-ipc-0")).unwrap();
        let server = std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            let mut head = [0u8; 8];
            s.read_exact(&mut head).unwrap();
            let len = u32::from_le_bytes(head[4..8].try_into().unwrap()) as usize;
            let mut body = vec![0u8; len];
            s.read_exact(&mut body).unwrap();
            let hello: Value = serde_json::from_slice(&body).unwrap();
            assert_eq!(hello["client_id"], "123");
            s.write_all(&frame(OP_FRAME, br#"{"evt":"READY","cmd":"DISPATCH"}"#)).unwrap();
            s.write_all(&frame(OP_PING, br#"{}"#)).unwrap();
            s.write_all(&frame(OP_FRAME, br#"{"evt":"ACTIVITY_JOIN","data":{"secret":"x"}}"#)).unwrap();
            s.read_exact(&mut head).unwrap();
            assert_eq!(u32::from_le_bytes(head[0..4].try_into().unwrap()), OP_PONG);
        });
        std::env::set_var("XDG_RUNTIME_DIR", &dir);
        let mut c = Conn::connect("123").unwrap();
        let mut got = vec![];
        for _ in 0..200 {
            got.extend(c.poll().unwrap());
            if !got.is_empty() {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        server.join().unwrap();
        assert_eq!(got[0]["evt"], "ACTIVITY_JOIN");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
