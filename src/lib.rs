use std::{
    fmt,
    io::{Error, Write},
    net::{AddrParseError, IpAddr, SocketAddr, TcpStream},
    str::FromStr,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const RETRIES: u8 = 3;

/// The Graphite Client
pub struct GraphiteClient {
    // TCP sock with Graphite server
    connection: TcpStream,
    // Socket address stored for reconnects
    sock_addr: SocketAddr,
}

impl GraphiteClient {
    pub fn new(g_addr: &str, g_port: u16) -> Result<Self, GraphiteError> {
        let sock_addr = SocketAddr::new(IpAddr::from_str(g_addr)?, g_port);
        let connection = TcpStream::connect_timeout(&sock_addr, Duration::from_secs(5))?;
        Ok(Self {
            connection,
            sock_addr,
        })
    }

    pub fn reconnect(&mut self) -> Result<(), GraphiteError> {
        self.connection = TcpStream::connect_timeout(&self.sock_addr, Duration::from_secs(5))?;
        Ok(())
    }

    pub fn send_message(&mut self, msg: &GraphiteMessage) -> Result<usize, GraphiteError> {
        let mut last_err: Error = Error::last_os_error();
        let mut i = 0;
        while i < RETRIES {
            let mut c = &self.connection;
            let res = c.write(msg.to_string().as_bytes());
            match res {
                Ok(size) => return Ok(size),
                Err(err) => last_err = err,
            }
            // In case the socket has been broken somewhere, reconnect it.
            self.reconnect()?;
            i += 1;
        }
        Err(GraphiteError {
            msg: format!("Graphite Error: {last_err}"),
        })
    }
}

impl Drop for GraphiteClient {
    fn drop(&mut self) {
        let _ = self.connection.shutdown(std::net::Shutdown::Both);
    }
}

/// The Graphite Message
#[derive(Debug)]
pub struct GraphiteMessage {
    metric_path: String,
    value: String,
    timestamp: u64,
}

impl GraphiteMessage {
    pub fn new(metric_path: &str, value: &str) -> Self {
        Self {
            metric_path: metric_path.to_string(),
            value: value.to_string(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }
}

impl fmt::Display for GraphiteMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{} {} {}", self.metric_path, self.value, self.timestamp)
    }
}

pub struct GraphiteError {
    pub msg: String,
}

// Implement Display trait (required for Error trait)
impl fmt::Display for GraphiteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.msg)
    }
}

// Implement Debug trait (required for Error trait)
impl fmt::Debug for GraphiteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GraphiteError {{ msg: {:?} }}", self.msg)
    }
}

// Implement the Error trait
impl std::error::Error for GraphiteError {}

impl From<AddrParseError> for GraphiteError {
    fn from(err: AddrParseError) -> Self {
        GraphiteError {
            msg: err.to_string(),
        }
    }
}

impl From<Error> for GraphiteError {
    fn from(err: Error) -> Self {
        GraphiteError {
            msg: err.to_string(),
        }
    }
}
