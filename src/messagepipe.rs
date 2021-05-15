pub trait CrossPlatformMessagePipe {

    fn as_handles(&self) -> Vec<u64>;

    unsafe fn from_raw_handles(handles: Vec<u64>) -> Result<Self, String> where Self: std::marker::Sized;

    fn new() -> Result<(Self, Self), String> where Self: std::marker::Sized;

    fn set_remote_pid(&mut self, pid: u64);

    fn recv(&mut self) -> Result<(Vec<u8>, Option<u64>), String>;

    fn send(&mut self, message: &[u8], handle: Option<u64>) -> Result<(), String>;
}

