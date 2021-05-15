// Common modules

mod ipc;
mod messagepipe;

// OS-specific modules

#[cfg_attr(target_os = "linux", path = "linux/mod.rs")]
#[cfg_attr(target_os = "windows", path = "windows/mod.rs")]
mod os;

pub use messagepipe::CrossPlatformMessagePipe;
pub use os::messagepipe::OSMessagePipe as MessagePipe;
pub use ipc::IPCMessagePipe;

