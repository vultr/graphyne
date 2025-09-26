# graphyne
A simple but useful Rust client for sending messages to a Graphite Carbon collector daemon.


## Getting started
```
cargo add graphyne
```

## Example usage

### Create a GraphiteClient
```rust
let host = "127.0.0.1";
let port: u16 = 2023;
let client = GraphiteClient.new(host, port).unwrap();
```

### Construct a GraphiteMessage
```rust
let my_key_path = "my.key.path";
let my_metric = 42;
let message = GraphiteMessage.new(my_key_path, my_metric);
```

### Send the GraphiteMessage
```rust
client.send_message(&message);
```

## Known limitations

- `GraphiteClient::new()` will fail when passed a DNS name instead of IP address.

## Contributing
We welcome contributions, please open a pull request.

## License
Apache 2.0
