# sbus-protocol

![CI](https://github.com/jettify/sbus-protocol/actions/workflows/rust_ci.yml/badge.svg)

A `no_std` compatible Rust library for parsing and encoding SBUS (Serial Bus) packets
commonly used in RC systems. SBUS is a protocol developed by Futaba for RC
receivers to communicate with flight controllers and other devices.

## SBUS Protoc0l

The protocol runs on top of UART communication. Typical parameters are 100kbps (or 200kbps in fast mode), `8e2` frame format, inverted (high voltage level is logic low).

    Byte[0]: SBUS header, 0x0F
    Byte[1 -22]: 16 servo channels, 11 bits each
    Byte[23]
        Bit 0: channel 17 (0x01)
        Bit 1: channel 18 (0x02)
        Bit 2: frame lost (0x04)
        Bit 3: failsafe activated (0x08)
    Byte[24]: SBUS footer

## Installation

```bash
cargo add sbus-protocol
```

or

```toml
[dependencies]
uf-sbus = { version="0.1.0", features = ["defmt"] }

```

## Simple example

```rust
use uf_sbus::SbusParser;

fn main() {
    println!("simple example ");
    let data: [u8; 25] = [
        0x0F, 0xE0, 0x03, 0x1F, 0x58, 0xC0, 0x07, 0x16, 0xB0, 0x80, 0x05, 0x2C, 0x60, 0x01, 0x0B,
        0xF8, 0xC0, 0x07, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 00,
    ];
    let mut sbus = SbusParser::new();

    for result in sbus.iter_packets(&data) {
        match result {
            Ok(packet) => {
                println!("{:?}", packet)
            }
            Err(err) => {
                println!("Parsing failed with {:?}", err)
            }
        }
    }
}
```

## Embassy

See `esp32c6` example for complete code:

```rust
    let mut uart = uart0.into_async();
    let mut buf: [u8; 25] = [0; 25];
    let mut sbus = SbusParser::new();
    let timeout = 500;  // Default from betaflight

    loop {
        // Read with timeout using embassy_time features
        let result = with_timeout(Duration::from_millis(timeout), uart.read_async(&mut buf)).await;
        match result {
            Ok(Ok(size)) => {
                for result in sbus.iter_packets(&buf[..size]) {
                    match result {
                        Ok(packet) => info!("Sbus Packet: {:?}", packet.channels),
                        Err(e) => info!("Sbus Error: {:?}", e),
                    }
                }
            }
            Ok(Err(read_error)) => {
                info!("UART reading error {:?}", read_error)
            }
            // Timeout happened, internal buffer possibly contain partial packet,
           // good idea to reset parser and start waiting for header again.
            Err(TimeoutError) => sbus.reset(),
        }
    }
}
```

## References

1. Protocol decoder from `sigrok` <https://sigrok.org/wiki/Protocol_decoder:Sbus_futaba>
1. Arduino library <https://github.com/bolderflight/SBUS>

## License

Licensed under the Apache License, Version 2.0

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
