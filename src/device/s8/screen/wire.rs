//! Binary frame records the app writes to the bridge's stdin.
//!
//! A record is the 12-byte header `S8FR`, panel `u8`, three zero bytes and the
//! payload length as `u32` little-endian (`PIXEL_BYTES`), then that many bytes
//! of big-endian RGB565 pixels, row-major from the top left. The bridge adds
//! the frame header, command and footer. Specification:
//! `docs/s8/screen-transport.md`.

use std::fmt;
use std::io::{self, Read};

use super::worker::Handle;
use super::{PIXEL_BYTES, Panel};

pub const MAGIC: [u8; 4] = *b"S8FR";
pub const HEADER_BYTES: usize = 12;

/// Why a record could not be read. Every case except `Eof` desynchronises or
/// ends the stream.
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    BadMagic([u8; 4]),
    BadPanel(u8),
    BadReserved([u8; 3]),
    BadLength(u32),
    /// The stream ended `got` bytes into a record part of `wanted` bytes.
    Truncated {
        part: &'static str,
        got: usize,
        wanted: usize,
    },
    Io(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadMagic(magic) => write!(f, "bad record magic {magic:02x?}"),
            Self::BadPanel(panel) => write!(f, "bad panel {panel}"),
            Self::BadReserved(bytes) => write!(f, "reserved bytes not zero {bytes:02x?}"),
            Self::BadLength(length) => {
                write!(f, "bad payload length {length}, expected {PIXEL_BYTES}")
            }
            Self::Truncated { part, got, wanted } => {
                write!(f, "stream ended in record {part}: {got} of {wanted} bytes")
            }
            Self::Io(error) => write!(f, "read error: {error}"),
        }
    }
}

impl std::error::Error for Error {}

/// Reads until `buffer` is full or the stream ends; returns the bytes read.
fn fill(reader: &mut impl Read, buffer: &mut [u8]) -> Result<usize, Error> {
    let mut filled = 0;
    while let Some(rest) = buffer.get_mut(filled..).filter(|rest| !rest.is_empty()) {
        match reader.read(rest) {
            Ok(0) => break,
            Ok(count) => filled += count,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(Error::Io(error.to_string())),
        }
    }
    Ok(filled)
}

/// Reads one record into `pixels`.
///
/// Returns `Ok(None)` when the stream ends cleanly between records.
///
/// # Errors
/// A wrong magic, panel, reserved byte or length, a stream that ends inside a
/// record, or a read error. `pixels` is unspecified after an error.
pub fn read_record(
    reader: &mut impl Read,
    pixels: &mut [u8; PIXEL_BYTES],
) -> Result<Option<Panel>, Error> {
    let mut header = [0_u8; HEADER_BYTES];
    match fill(reader, &mut header)? {
        0 => return Ok(None),
        HEADER_BYTES => {}
        got => {
            return Err(Error::Truncated {
                part: "header",
                got,
                wanted: HEADER_BYTES,
            });
        }
    }
    let [m0, m1, m2, m3, panel, r0, r1, r2, l0, l1, l2, l3] = header;
    let magic = [m0, m1, m2, m3];
    if magic != MAGIC {
        return Err(Error::BadMagic(magic));
    }
    let panel = match panel {
        0 => Panel::Left,
        1 => Panel::Right,
        other => return Err(Error::BadPanel(other)),
    };
    if [r0, r1, r2] != [0, 0, 0] {
        return Err(Error::BadReserved([r0, r1, r2]));
    }
    let length = u32::from_le_bytes([l0, l1, l2, l3]);
    if usize::try_from(length) != Ok(PIXEL_BYTES) {
        return Err(Error::BadLength(length));
    }
    let got = fill(reader, pixels)?;
    if got != PIXEL_BYTES {
        return Err(Error::Truncated {
            part: "payload",
            got,
            wanted: PIXEL_BYTES,
        });
    }
    Ok(Some(panel))
}

/// Feeds every record of `reader` to the worker until the stream ends.
///
/// A protocol or read error aborts the screen path with the error as the
/// reason, after which the rest of the stream is read and discarded so the
/// writer never blocks and the end of the stream is still seen. Returns when
/// the stream ends.
pub fn serve(mut reader: impl Read, handle: &Handle) {
    let Ok(mut pixels) = Box::<[u8; PIXEL_BYTES]>::try_from(vec![0_u8; PIXEL_BYTES]) else {
        handle.abort("could not allocate the frame buffer");
        return;
    };
    loop {
        match read_record(&mut reader, &mut pixels) {
            Ok(Some(panel)) => handle.submit(panel, &pixels),
            Ok(None) => return,
            Err(error) => {
                handle.abort(&error.to_string());
                // Draining cannot fail in a way that matters: either error
                // or end of stream finishes the reader.
                io::copy(&mut reader, &mut io::sink()).ok();
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    fn record(panel: u8, fill: u8) -> Vec<u8> {
        let mut bytes = MAGIC.to_vec();
        bytes.extend_from_slice(&[panel, 0, 0, 0]);
        bytes.extend_from_slice(&u32::try_from(PIXEL_BYTES).unwrap_or(0).to_le_bytes());
        bytes.resize(HEADER_BYTES + PIXEL_BYTES, fill);
        bytes
    }

    fn pixels() -> Box<[u8; PIXEL_BYTES]> {
        Box::<[u8; PIXEL_BYTES]>::try_from(vec![0; PIXEL_BYTES]).unwrap_or_else(|_| unreachable!())
    }

    /// Hands out one byte per read.
    struct Trickle(Cursor<Vec<u8>>);

    impl Read for Trickle {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            let one = buffer.len().min(1);
            self.0.read(&mut buffer[..one])
        }
    }

    #[test]
    fn reads_a_valid_record_for_each_panel() {
        for (byte, panel) in [(0, Panel::Left), (1, Panel::Right)] {
            let mut stream = Cursor::new(record(byte, 0xab));
            let mut buffer = pixels();
            assert_eq!(read_record(&mut stream, &mut buffer), Ok(Some(panel)));
            assert!(buffer.iter().all(|byte| *byte == 0xab));
            assert_eq!(read_record(&mut stream, &mut buffer), Ok(None));
        }
    }

    #[test]
    fn reads_records_split_across_reads() {
        let mut bytes = record(1, 0x5a);
        bytes.extend(record(0, 0xa5));
        let mut stream = Trickle(Cursor::new(bytes));
        let mut buffer = pixels();
        assert_eq!(
            read_record(&mut stream, &mut buffer),
            Ok(Some(Panel::Right))
        );
        assert_eq!(buffer[PIXEL_BYTES - 1], 0x5a);
        assert_eq!(read_record(&mut stream, &mut buffer), Ok(Some(Panel::Left)));
        assert_eq!(buffer[0], 0xa5);
        assert_eq!(read_record(&mut stream, &mut buffer), Ok(None));
    }

    #[test]
    fn rejects_wrong_magic() {
        let mut bytes = record(0, 0);
        bytes[3] = b'X';
        let mut buffer = pixels();
        assert_eq!(
            read_record(&mut Cursor::new(bytes), &mut buffer),
            Err(Error::BadMagic(*b"S8FX"))
        );
    }

    #[test]
    fn rejects_wrong_panel_and_reserved_bytes() {
        let mut buffer = pixels();
        assert_eq!(
            read_record(&mut Cursor::new(record(2, 0)), &mut buffer),
            Err(Error::BadPanel(2))
        );
        let mut bytes = record(0, 0);
        bytes[6] = 1;
        assert_eq!(
            read_record(&mut Cursor::new(bytes), &mut buffer),
            Err(Error::BadReserved([0, 1, 0]))
        );
    }

    #[test]
    fn rejects_wrong_length_before_reading_the_payload() {
        for length in [0_u32, 261_119, 261_121, u32::MAX] {
            let mut bytes = record(0, 0);
            bytes[8..12].copy_from_slice(&length.to_le_bytes());
            let mut buffer = pixels();
            assert_eq!(
                read_record(&mut Cursor::new(bytes), &mut buffer),
                Err(Error::BadLength(length))
            );
        }
    }

    #[test]
    fn eof_inside_a_record_is_truncation() {
        let mut buffer = pixels();
        assert_eq!(
            read_record(&mut Cursor::new(record(0, 0)[..5].to_vec()), &mut buffer),
            Err(Error::Truncated {
                part: "header",
                got: 5,
                wanted: HEADER_BYTES
            })
        );
        assert_eq!(
            read_record(&mut Cursor::new(record(0, 0)[..100].to_vec()), &mut buffer),
            Err(Error::Truncated {
                part: "payload",
                got: 100 - HEADER_BYTES,
                wanted: PIXEL_BYTES
            })
        );
    }

    #[test]
    fn io_errors_are_reported() {
        struct Broken;
        impl Read for Broken {
            fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
                Err(io::Error::other("pipe"))
            }
        }
        let mut buffer = pixels();
        assert_eq!(
            read_record(&mut Broken, &mut buffer),
            Err(Error::Io("pipe".to_owned()))
        );
    }
}
