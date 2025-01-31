#![no_std]
const SBUS_PACKET_SIZE: usize = 25;
const NUM_SBUS_CHANNELS: usize = 16;
const SBUS_HEADER: u8 = 0x0F;
const SBUS_FLAG_BYTE_MASK: u8 = 0xF0;
const SBUS_FOOTER: u8 = 0x00;
const CHAN_MASK: u16 = 0x07FF;

#[derive(Debug, PartialEq, Eq)]
pub enum SbusParserError {
    InvalidFooter(u8),
    InvalidFlags(u8),
}

#[inline(always)]
pub fn is_sbus_footer(byte: u8) -> bool {
    match byte {
        0x00 => true, // SBUS packet end
        0x04 => true, // SBUS telemetry slot 0 to Slot 7
        0x14 => true, // SBUS telemetry slot 8 to Slot 15
        0x24 => true, // SBUS telemetry slot 16 to Slot 23
        0x34 => true, // SBUS telemetry slot 24 to Slot 31
        _ => false,
    }
}

#[inline(always)]
fn is_flag_set_at_position(flag_byte: u8, shift_by: u8) -> bool {
    (flag_byte >> shift_by) & 1 == 1
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Copy, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SbusPacket {
    pub channels: [u16; NUM_SBUS_CHANNELS],
    pub channel_17: bool,
    pub channel_18: bool,
    pub failsafe: bool,
    pub frame_lost: bool,
}

#[derive(Debug, Default, Ord, PartialOrd, Eq, PartialEq, Copy, Clone)]
pub enum State {
    #[default]
    AwaitingHead,
    Reading(usize),
}

#[derive(Debug, Default)]
pub struct SbusParser {
    buffer: [u8; SBUS_PACKET_SIZE],
    state: State,
}

impl SbusParser {
    pub fn new() -> Self {
        Self {
            buffer: [0; SBUS_PACKET_SIZE],
            state: State::AwaitingHead,
        }
    }
    pub fn push_byte(&mut self, byte: u8) -> Option<Result<SbusPacket, SbusParserError>> {
        match self.state {
            State::AwaitingHead => {
                if byte == SBUS_HEADER {
                    self.buffer[0] = byte;
                    self.state = State::Reading(1);
                }
            }
            State::Reading(n) if n == SBUS_PACKET_SIZE - 1 => {
                self.buffer[n] = byte;
                self.state = State::AwaitingHead;
                return Some(self.try_parse());
            }
            State::Reading(n) => {
                self.buffer[n] = byte;
                self.state = State::Reading(n + 1)
            }
        }
        None
    }

    pub fn reset(&mut self) {
        self.state = State::AwaitingHead;
        self.buffer = [0; SBUS_PACKET_SIZE];
    }

    pub fn try_parse(&self) -> Result<SbusPacket, SbusParserError> {
        if self.state != State::Reading(SBUS_PACKET_SIZE) {
            self.validate_frame()?;
        }
        let buf = &self.buffer;

        // Initialize channels with 11-bit mask
        let mut ch: [u16; NUM_SBUS_CHANNELS] = [CHAN_MASK; NUM_SBUS_CHANNELS];

        ch[0] &= (buf[1] as u16) | ((buf[2] as u16) << 8);
        ch[1] &= ((buf[2] as u16) >> 3) | ((buf[3] as u16) << 5);
        ch[2] &= ((buf[3] as u16) >> 6) | ((buf[4] as u16) << 2) | ((buf[5] as u16) << 10);
        ch[3] &= ((buf[5] as u16) >> 1) | ((buf[6] as u16) << 7);
        ch[4] &= ((buf[6] as u16) >> 4) | ((buf[7] as u16) << 4);
        ch[5] &= ((buf[7] as u16) >> 7) | ((buf[8] as u16) << 1) | ((buf[9] as u16) << 9);
        ch[6] &= ((buf[9] as u16) >> 2) | ((buf[10] as u16) << 6);
        ch[7] &= ((buf[10] as u16) >> 5) | ((buf[11] as u16) << 3);

        ch[8] &= (buf[12] as u16) | ((buf[13] as u16) << 8);
        ch[9] &= ((buf[13] as u16) >> 3) | ((buf[14] as u16) << 5);
        ch[10] &= ((buf[14] as u16) >> 6) | ((buf[15] as u16) << 2) | ((buf[16] as u16) << 10);
        ch[11] &= ((buf[16] as u16) >> 1) | ((buf[17] as u16) << 7);
        ch[12] &= ((buf[17] as u16) >> 4) | ((buf[18] as u16) << 4);
        ch[13] &= ((buf[18] as u16) >> 7) | ((buf[19] as u16) << 1) | ((buf[20] as u16) << 9);
        ch[14] &= ((buf[20] as u16) >> 2) | ((buf[21] as u16) << 6);
        ch[15] &= ((buf[21] as u16) >> 5) | ((buf[22] as u16) << 3);

        let flag_byte = buf[23];

        Ok(SbusPacket {
            channels: ch,
            channel_17: is_flag_set_at_position(flag_byte, 0),
            channel_18: is_flag_set_at_position(flag_byte, 1),
            frame_lost: is_flag_set_at_position(flag_byte, 2),
            failsafe: is_flag_set_at_position(flag_byte, 3),
        })
    }

    pub fn validate_frame(&self) -> Result<(), SbusParserError> {
        let footer = self.buffer[SBUS_PACKET_SIZE - 1];
        let flags = self.buffer[SBUS_PACKET_SIZE - 2];

        if !is_sbus_footer(footer) {
            Err(SbusParserError::InvalidFooter(footer))
        } else if flags & SBUS_FLAG_BYTE_MASK != 0 {
            Err(SbusParserError::InvalidFlags(flags))
        } else {
            Ok(())
        }
    }
}

#[inline(always)]
pub fn encode_packet(buf: &mut [u8; SBUS_PACKET_SIZE], packet: &SbusPacket) {
    let ch = &packet.channels;

    // Start byte
    buf[0] = SBUS_HEADER;

    // Encode channels by setting specific bits while preserving others
    // Ch 0: all bits in buf[1], bits 0-2 in buf[2]
    buf[1] = ch[0] as u8;
    buf[2] = (buf[2] & !0x07) | ((ch[0] >> 8) & 0x07) as u8;
    // Overlay ch[1] bits 0-4 into buf[2] bits 3-7
    buf[2] = (buf[2] & !0xF8) | ((ch[1] & 0x1F) << 3) as u8;

    // Ch 1 remaining bits + start of Ch 2
    buf[3] = ((ch[1] >> 5) & 0x3F) as u8 | ((ch[2] & 0x03) << 6) as u8;

    // Ch 2 middle bits
    buf[4] = ((ch[2] >> 2) & 0xFF) as u8;
    // Ch 2 last bit + Ch 3 first 7 bits
    buf[5] = ((ch[2] >> 10) & 0x01) as u8 | ((ch[3] & 0x7F) << 1) as u8;

    // Ch 3 remaining bits + Ch 4 first 4 bits
    buf[6] = ((ch[3] >> 7) & 0x0F) as u8 | ((ch[4] & 0x0F) << 4) as u8;

    // Ch 4 remaining bits + Ch 5 first bit
    buf[7] = ((ch[4] >> 4) & 0x7F) as u8 | ((ch[5] & 0x01) << 7) as u8;

    // Ch 5 middle bits
    buf[8] = ((ch[5] >> 1) & 0xFF) as u8;
    // Ch 5 last 2 bits + Ch 6 first 6 bits
    buf[9] = ((ch[5] >> 9) & 0x03) as u8 | ((ch[6] & 0x3F) << 2) as u8;

    // Ch 6 remaining bits + Ch 7 first 3 bits
    buf[10] = ((ch[6] >> 6) & 0x1F) as u8 | ((ch[7] & 0x07) << 5) as u8;

    // Ch 7 remaining bits
    buf[11] = ((ch[7] >> 3) & 0xFF) as u8;

    // Channels 8-15 follow same pattern
    buf[12] = ch[8] as u8;
    buf[13] = ((ch[8] >> 8) & 0x07) as u8 | ((ch[9] & 0x1F) << 3) as u8;

    buf[14] = ((ch[9] >> 5) & 0x3F) as u8 | ((ch[10] & 0x03) << 6) as u8;
    buf[15] = ((ch[10] >> 2) & 0xFF) as u8;
    buf[16] = ((ch[10] >> 10) & 0x01) as u8 | ((ch[11] & 0x7F) << 1) as u8;

    buf[17] = ((ch[11] >> 7) & 0x0F) as u8 | ((ch[12] & 0x0F) << 4) as u8;
    buf[18] = ((ch[12] >> 4) & 0x7F) as u8 | ((ch[13] & 0x01) << 7) as u8;

    buf[19] = ((ch[13] >> 1) & 0xFF) as u8;
    buf[20] = ((ch[13] >> 9) & 0x03) as u8 | ((ch[14] & 0x3F) << 2) as u8;

    buf[21] = ((ch[14] >> 6) & 0x1F) as u8 | ((ch[15] & 0x07) << 5) as u8;
    buf[22] = ((ch[15] >> 3) & 0xFF) as u8;
    // clear byte first then set nesseary bits
    buf[23] = 0x00;
    buf[23] = buf[23]
        | (packet.channel_17 as u8)
        | ((packet.channel_18 as u8) << 1)
        | ((packet.frame_lost as u8) << 2)
        | ((packet.failsafe as u8) << 3);

    buf[24] = SBUS_FOOTER;
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate std;

    use hex_literal::hex;
    const RAW_BYTES: [u8; 25] =
        hex!("0F E0 03 1F 58 C0 07 16 B0 80 05 2C 60 01 0B F8 C0 07 00 00 00 00 00 03 00");

    #[test]
    fn test_basic_packing_unpacking() {
        let mut p = SbusParser::new();
        assert!(p.state == State::AwaitingHead);

        for b in &RAW_BYTES[0..RAW_BYTES.len() - 1] {
            assert!(p.push_byte(*b).is_none());
        }
        let packet = p.push_byte(RAW_BYTES[24]).unwrap().unwrap();
        let expected = SbusPacket {
            channels: [
                992, 992, 352, 992, 352, 352, 352, 352, 352, 352, 992, 992, 0, 0, 0, 0,
            ],
            channel_17: true,
            channel_18: true,
            failsafe: false,
            frame_lost: false,
        };
        assert!(packet == expected);

        // dirty buffer
        let mut buffer: [u8; SBUS_PACKET_SIZE] = [255; SBUS_PACKET_SIZE];
        encode_packet(&mut buffer, &packet);
        assert!(buffer == RAW_BYTES);
    }

    #[test]
    fn test_low_value() {
        const EXPECTED: [u8; SBUS_PACKET_SIZE] =
            hex!("0F 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00");

        let packet = SbusPacket {
            channels: [0; 16],
            channel_17: false,
            channel_18: false,
            failsafe: false,
            frame_lost: false,
        };
        // dirty buffer
        let mut buffer: [u8; SBUS_PACKET_SIZE] = [255; SBUS_PACKET_SIZE];
        encode_packet(&mut buffer, &packet);
        assert!(buffer == EXPECTED);
    }

    #[test]
    fn test_high_value() {
        const EXPECTED: [u8; SBUS_PACKET_SIZE] =
            hex!("0F FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF 0F 00");

        let packet = SbusPacket {
            channels: [2047; 16],
            channel_17: true,
            channel_18: true,
            failsafe: true,
            frame_lost: true,
        };

        let mut buffer: [u8; SBUS_PACKET_SIZE] = [0; SBUS_PACKET_SIZE];
        encode_packet(&mut buffer, &packet);
        assert!(buffer == EXPECTED);
    }
    #[test]
    fn test_malformed_footer() {
        let mut p = SbusParser::new();
        const BAD_FOOTER: [u8; SBUS_PACKET_SIZE] =
            hex!("0F FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF 0F FF");

        assert!(p.state == State::AwaitingHead);
        for b in &BAD_FOOTER[0..BAD_FOOTER.len() - 1] {
            assert!(p.push_byte(*b).is_none());
        }
        let err = p.push_byte(BAD_FOOTER[24]).unwrap();
        assert!(err == Err(SbusParserError::InvalidFooter(0xff)));
    }

    #[test]
    fn test_malformed_flags() {
        let mut p = SbusParser::new();
        const BAD_FLAGS: [u8; SBUS_PACKET_SIZE] =
            hex!("0F FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF 00");

        assert!(p.state == State::AwaitingHead);
        for b in &BAD_FLAGS[0..BAD_FLAGS.len() - 1] {
            assert!(p.push_byte(*b).is_none());
        }
        let err = p.push_byte(BAD_FLAGS[24]).unwrap();
        assert!(err == Err(SbusParserError::InvalidFlags(0xff)));
    }
}
