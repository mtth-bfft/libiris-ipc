use serde::{Serialize, Deserialize};
use bincode::Options;
use crate::os::messagepipe::OSMessagePipe;
use crate::messagepipe::CrossPlatformMessagePipe;

// Maximum message size that can be serialized and deserialized over an
// IPC channel. Larger messages should use other (more efficient)
// strategies to send/receive data, like shared memory sections.
pub(crate) const IPC_MESSAGE_MAX_SIZE: u64 = 1 * 1024 * 1024;

#[derive(Serialize, Deserialize, PartialEq, Debug)]
enum IPCRequest {
    ReportFailedExecve {
        errno: u64,
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
enum IPCResponse {
    Unused {
        errno: u64,
    }
}

pub struct IPCMessagePipe {
    pipe: OSMessagePipe,
}

impl IPCMessagePipe {
    fn new(pipe: OSMessagePipe) -> Self {
        Self {
            pipe,
        }
    }

    fn send(&mut self, request: &IPCRequest) -> Result<(), String>
    {
        // FIXME: extract handles/file descriptors and send them as ancillary data
        let bincode_config = bincode::DefaultOptions::new()
                .with_limit(IPC_MESSAGE_MAX_SIZE)
                .with_native_endian()
                .with_fixint_encoding()
                .reject_trailing_bytes();
        let bytes = match bincode_config.serialize(&request) {
            Ok(v) => v,
            Err(e) => return Err(format!("Unable to serialize request: {}", e)),
        };
        self.pipe.send(&bytes, None)
    }

    fn recv(&mut self) -> Result<IPCResponse, String>
    {
        let bincode_config = bincode::DefaultOptions::new()
                .with_limit(IPC_MESSAGE_MAX_SIZE)
                .with_native_endian()
                .with_fixint_encoding()
                .reject_trailing_bytes();
        let (bytes, fd) = self.pipe.recv()?;
        let response = match bincode_config.deserialize(&bytes) {
            Ok(r) => r,
            Err(e) => return Err(format!("Unable to deserialize response: {}", e)),
        };
        Ok(response)
    }
}

