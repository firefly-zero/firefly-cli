use anyhow::{Context, Result};
use firefly_types::{
    serial::{Request, Response},
    Encode,
};
use serialport::SerialPort;

// Given the binary stream so far, read the first COBS frame and return the rest of bytes.
pub fn read_cobs_frame(chunk: &[u8]) -> (Vec<u8>, &[u8]) {
    let max_len = chunk.len();
    let mut out_buf = vec![0; max_len];
    let mut dec = cobs::CobsDecoder::new(&mut out_buf);
    match dec.push(chunk) {
        Ok(Some((n_out, n_in))) => {
            let msg = Vec::from(&out_buf[..n_out]);
            (msg, &chunk[n_in..])
        }
        Ok(None) => (Vec::new(), chunk),
        Err(err) => match err {
            cobs::DecodeError::EmptyFrame => (Vec::new(), &[]),
            cobs::DecodeError::InvalidFrame { decoded_bytes } => {
                (Vec::new(), &chunk[decoded_bytes..])
            }
            cobs::DecodeError::TargetBufTooSmall => unreachable!(),
        },
    }
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

    pub fn send(&mut self, req: &Request) -> Result<()> {
        let buf = req.encode_vec().context("encode request")?;
        self.port.write_all(&buf[..]).context("send request")?;
        self.port.flush().context("flush request")?;
        Ok(())
    }

    pub fn next(&mut self) -> Result<Response> {
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

pub fn is_timeout(err: &anyhow::Error) -> bool {
    if let Some(err) = err.downcast_ref::<std::io::Error>() {
        return err.kind() == std::io::ErrorKind::TimedOut;
    }
    false
}
