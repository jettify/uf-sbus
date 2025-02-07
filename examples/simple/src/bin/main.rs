use sbus_protocol::SbusParser;

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
