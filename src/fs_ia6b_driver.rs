use defmt::{write, Format};
use embedded_hal::serial::Read;

const PROTOCOL_LENGTH: usize = 0x20; // 32 bytes per frame
const PROTOCOL_COMMAND: u8 = 0x40; // only known command is 0x40
const PROTOCOL_CHANNELS: usize = 14;
const PROTOCOL_OVERHEAD: u8 = 4; // <len><cmd><...><chkl><chkh>

#[derive(Copy, Clone, Debug)]
pub enum Error<SerialError> {
    SerialError(SerialError),
    WouldBlock,
    InvalidLength(u8, u8),
    InvalidCommand(u8, u8),
    InvalidChecksumL(u8, u8),
    InvalidChecksumH(u8, u8),
}

impl<SerialError> From<nb::Error<SerialError>> for Error<SerialError> {
    fn from(e: nb::Error<SerialError>) -> Self {
        match e {
            nb::Error::WouldBlock => Error::WouldBlock,
            nb::Error::Other(e) => Error::SerialError(e),
        }
    }
}

impl<SerialError> Format for Error<SerialError> {
    fn format(&self, f: defmt::Formatter) {
        match *self {
            Error::SerialError(_) => {
                write!(f, "bus communication error");
            }
            Error::WouldBlock => {
                write!(f, "operation would cause blocking");
            }
            Error::InvalidLength(got, expected) => {
                write!(
                    f,
                    "invalid packet length: got {=u8}, expected {=u8}",
                    got, expected
                );
            }
            Error::InvalidCommand(got, expected) => {
                write!(
                    f,
                    "invalid packet command: got {=u8}, expected {=u8}",
                    got, expected
                );
            }
            Error::InvalidChecksumL(got, expected) => {
                write!(
                    f,
                    "invalid checksum lower byte: got {=u8}, expected {=u8}",
                    got, expected
                );
            }
            Error::InvalidChecksumH(got, expected) => {
                write!(
                    f,
                    "invalid checksum upper byte: got {=u8}, expected {=u8}",
                    got, expected
                );
            }
        }
    }
}

enum State {
    GetLength,
    GetCommand,
    GetData,
    GetChksumL,
    GetChksumH,
}

pub struct Driver<Serial>
where
    Serial: Read<u8>,
{
    serial: Serial,
    state: State,
    data_len: u8,
    idx: usize,
    checksum: u16,
    channel_data: [u16; PROTOCOL_CHANNELS],
}

impl<Serial, ReadError> Driver<Serial>
where
    Serial: Read<u8, Error = ReadError>,
{
    pub fn new(serial: Serial) -> Self {
        Self {
            serial,
            state: State::GetLength,
            data_len: 0,
            idx: 0,
            checksum: 0,
            channel_data: [0; PROTOCOL_CHANNELS],
        }
    }

    pub fn read(&mut self) -> Result<Option<&[u16; PROTOCOL_CHANNELS]>, Error<ReadError>> {
        let byte = self.serial.read()?;

        match self.state {
            State::GetLength => {
                // validate packet length
                if byte == PROTOCOL_LENGTH as u8 {
                    // store data length
                    self.data_len = byte - PROTOCOL_OVERHEAD;
                    // update checksum
                    self.checksum = 0xFFFF - (byte as u16);
                    // read command
                    self.state = State::GetCommand;
                } else {
                    // reset packet frame & return error
                    self.state = State::GetLength;
                    return Err(Error::InvalidLength(byte, PROTOCOL_LENGTH as u8));
                }
            }
            State::GetCommand => {
                // validate command
                if byte == PROTOCOL_COMMAND {
                    // reset index and channel data
                    self.idx = 0;
                    for channel in self.channel_data.iter_mut() {
                        *channel = 0;
                    }
                    // update checksum
                    self.checksum -= byte as u16;
                    // read channel data
                    self.state = State::GetData;
                } else {
                    // reset packet frame & return error
                    self.state = State::GetLength;
                    return Err(Error::InvalidCommand(byte, PROTOCOL_COMMAND));
                }
            }
            State::GetData => {
                // save byte of channel data
                self.channel_data[self.idx >> 1] |= (byte as u16) << ((self.idx & 0x0001) << 3);
                // increment
                self.idx += 1;
                // update cheksum
                self.checksum -= byte as u16;
                // once all bytes are recieved validate checksum
                if self.idx == self.data_len as usize {
                    self.state = State::GetChksumL;
                }
            }
            State::GetChksumL => {
                // validate lower half of checksum
                if self.checksum & 0x00FF == byte as u16 {
                    // continue to upper half
                    self.state = State::GetChksumH;
                } else {
                    // reset packet frame & return error
                    self.state = State::GetLength;
                    return Err(Error::InvalidChecksumL(
                        byte,
                        (self.checksum & 0x00FF) as u8,
                    ));
                }
            }
            State::GetChksumH => {
                // validate upper half of checksum
                if self.checksum & 0xFF00 == (byte as u16) << 8 {
                    // update state
                    self.state = State::GetLength;
                    // return data
                    return Ok(Some(&self.channel_data));
                } else {
                    // reset packet frame & return error
                    self.state = State::GetLength;
                    return Err(Error::InvalidChecksumH(byte, (self.checksum >> 8) as u8));
                }
            }
        }

        Ok(None)
    }
}
