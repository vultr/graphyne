use bon::bon;
use std::{
    fmt,
    io::{Error, Write},
    net::{AddrParseError, IpAddr, SocketAddr, TcpStream},
    str::FromStr,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const DEFAULT_RETRIES: u8 = 3;
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

/// The Graphite Client
#[derive(Debug)]
pub struct GraphiteClient {
    // TCP sock with Graphite server
    connection: TestableTcpStream,
    // Socket address stored for reconnects
    sock_addr: SocketAddr,
    // Configuration
    // (address and port is not used currently
    // but will to provide dns with reconnects in the future)
    _address: String,
    _port: u16,
    retries: u8,
    timeout: Duration,
}

#[bon]
impl GraphiteClient {
    #[builder]
    pub fn new(
        address: impl Into<String>,
        port: u16,
        #[builder(default = DEFAULT_RETRIES)] retries: u8,
        #[builder(default = DEFAULT_TIMEOUT)] timeout: Duration,
    ) -> Result<Self, GraphiteError> {
        let address = address.into();
        let sock_addr = SocketAddr::new(IpAddr::from_str(&address)?, port);
        let connection = TcpStream::connect_timeout(&sock_addr, timeout)?;

        Ok(Self {
            connection: TestableTcpStream(connection),
            sock_addr,
            _address: address,
            _port: port,
            retries,
            timeout,
        })
    }

    pub fn reconnect(&mut self) -> Result<(), GraphiteError> {
        let mut last_err: Error = Error::last_os_error();
        let mut i = 0;
        while i < self.retries {
            let connect = TcpStream::connect_timeout(&self.sock_addr, self.timeout);
            match connect {
                Ok(connect) => {
                    self.connection = TestableTcpStream(connect);
                    return Ok(());
                }
                Err(err) => last_err = err,
            }
            i += 1;
        }
        Err(GraphiteError {
            msg: format!("Graphite Error: {last_err}"),
        })
    }

    pub fn send_message(&mut self, msg: &GraphiteMessage) -> Result<usize, GraphiteError> {
        let mut last_err: Error = Error::last_os_error();
        let mut i = 0;
        while i < self.retries {
            let res = self.connection.0.write(msg.to_string().as_bytes());
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
        let _ = self.connection.0.shutdown(std::net::Shutdown::Both);
    }
}

/// The Graphite Message
#[derive(Debug, Clone, PartialEq)]
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

#[derive(Clone)]
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

/// Wrapper for TcpStream for a better Debug impl for snapshots
#[cfg_attr(not(test), derive(Debug))]
struct TestableTcpStream(TcpStream);

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    impl fmt::Debug for TestableTcpStream {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let peer = self.0.peer_addr().ok();
            f.debug_struct("TcpStream")
                .field("peer_addr", &peer)
                .finish()
        }
    }

    // Dummy listener that accepts connections
    struct DummyGraphiteServer {
        _handle: std::thread::JoinHandle<()>,
    }

    impl DummyGraphiteServer {
        fn start(port: u16) -> Self {
            let handle = std::thread::spawn(move || {
                let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).unwrap();
                while let Ok((_stream, _addr)) = listener.accept() {
                    // Just accept and drop the connection
                }
            });

            // pause to start -- seems to be a race without waiting
            std::thread::sleep(Duration::from_millis(50));

            Self { _handle: handle }
        }
    }

    #[test]
    fn test_client_builder_defaults() {
        let port = 20031;
        let _ = DummyGraphiteServer::start(port);

        let client = GraphiteClient::builder()
            .address("127.0.0.1")
            .port(port)
            .build()
            .unwrap();

        insta::assert_debug_snapshot!(client);
    }

    #[test]
    fn test_client_builder_custom_retries() {
        let port = 20032;
        let _ = DummyGraphiteServer::start(port);

        let client = GraphiteClient::builder()
            .address("127.0.0.1")
            .port(port)
            .retries(10)
            .build()
            .unwrap();

        insta::assert_debug_snapshot!(client);
    }

    #[test]
    fn test_client_builder_custom_timeout() {
        let port = 20033;
        let _ = DummyGraphiteServer::start(port);

        let client = GraphiteClient::builder()
            .address("127.0.0.1")
            .port(port)
            .timeout(Duration::from_millis(100))
            .build()
            .unwrap();

        insta::assert_debug_snapshot!(client);
    }

    #[test]
    fn test_client_builder_all_options() {
        let port = 20034;
        let _ = DummyGraphiteServer::start(port);

        let client = GraphiteClient::builder()
            .address("127.0.0.1")
            .port(port)
            .retries(7)
            .timeout(Duration::from_secs(3))
            .build()
            .unwrap();

        insta::assert_debug_snapshot!(client);
    }

    #[test]
    fn test_connection_failure() {
        let result = GraphiteClient::builder()
            .address("127.0.0.1")
            .port(6969)
            .timeout(Duration::from_millis(100))
            .build();

        assert!(result.is_err());
    }
}
