use std::{
    io::{Read, Write},
    net::TcpStream,
};

use anyhow::{Context, Result};
use firefly_types::{
    serial::{Request, Response},
    Encode,
};
use serialport::SerialPort;

// Given the binary stream so far, read the first COBS frame and return the rest of bytes.
fn read_cobs_frame(chunk: &[u8]) -> (Vec<u8>, &[u8]) {
    let max_len = chunk.len();
    let mut out_buf = vec![0; max_len];
    let mut dec = cobs::CobsDecoder::new(&mut out_buf);
    match dec.push(chunk) {
        Ok(Some(report)) => {
            let n_in = report.parsed_size();
            let n_out = report.frame_size();
            let msg = Vec::from(&out_buf[..n_out]);
            (msg, &chunk[n_in..])
        }
        Ok(None) => (Vec::new(), chunk),
        Err(err) => match err {
            cobs::DecodeError::EmptyFrame => (Vec::new(), &[]),
            cobs::DecodeError::InvalidFrame { decoded_bytes: _ } => {
                let new_chunk = find_frame(chunk);
                if new_chunk.len() == chunk.len() {
                    // Invalid frame and no frame separator in the current buffer.
                    // Don't modify the buffer, keep it growing until a frame separator arrives.
                    // This allows us to handle messages that are bigger than the buffer.
                    (Vec::new(), chunk)
                } else {
                    // There is an invalid frame followed by a frame separator.
                    // Skip the invalid frame and try parsing the next frame.
                    read_cobs_frame(new_chunk)
                }
            }
            cobs::DecodeError::TargetBufTooSmall => unreachable!(),
        },
    }
}

/// Cut out everything before the first `\x0` separator (skipping consecutive `\x0`'s).
fn find_frame(chunk: &[u8]) -> &[u8] {
    let mut iter = chunk.iter().enumerate();
    for (_, b) in iter.by_ref() {
        if *b == 0 {
            break;
        }
    }
    for (i, b) in iter {
        if *b != 0 {
            return &chunk[i..];
        }
    }
    chunk
}

pub trait Stream {
    fn send(&mut self, req: &Request) -> Result<()>;
    fn next(&mut self) -> Result<Response>;
}

pub struct SerialStream {
    port: Box<dyn SerialPort + 'static>,
    buf: Vec<u8>,
}

impl SerialStream {
    pub fn new(port: Box<dyn SerialPort + 'static>) -> Self {
        Self {
            port,
            buf: Vec::new(),
        }
    }

    fn load_more(&mut self) -> Result<()> {
        let mut chunk = vec![0; 64];
        let n = self.port.read(&mut chunk)?;
        self.buf.extend_from_slice(&chunk[..n]);
        Ok(())
    }
}

impl Stream for SerialStream {
    fn send(&mut self, req: &Request) -> Result<()> {
        let buf = req.encode_vec().context("encode request")?;
        self.port.write_all(&buf[..]).context("send request")?;
        self.port.flush().context("flush request")?;
        Ok(())
    }

    fn next(&mut self) -> Result<Response> {
        loop {
            let (frame, rest) = read_cobs_frame(&self.buf);
            self.buf = Vec::from(rest);
            if frame.is_empty() {
                self.load_more()?;
                continue;
            }
            let response = Response::decode(&frame)?;
            return Ok(response);
        }
    }
}

impl Stream for TcpStream {
    fn send(&mut self, req: &Request) -> Result<()> {
        let buf = req.encode_vec().context("encode request")?;
        self.write_all(&buf).context("send request")?;
        self.flush().context("flush request")?;
        Ok(())
    }

    fn next(&mut self) -> Result<Response> {
        let mut buf = vec![0; 64];
        self.read(&mut buf).context("read response")?;
        let resp = Response::decode(&buf).context("decode response")?;
        Ok(resp)
    }
}

pub fn is_timeout(err: &anyhow::Error) -> bool {
    if let Some(err) = err.downcast_ref::<std::io::Error>() {
        return err.kind() == std::io::ErrorKind::TimedOut;
    }
    false
}
