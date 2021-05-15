use std::convert::TryInto;
use core::ptr::null_mut;
use winapi::um::{errhandlingapi, namedpipeapi, fileapi, handleapi};
use winapi::shared::minwindef::DWORD;
use winapi::um::winnt::HANDLE;
use crate::ipc::IPC_MESSAGE_MAX_SIZE;
use crate::messagepipe::CrossPlatformMessagePipe;

pub struct OSMessagePipe {
    read_handle: HANDLE,
    write_handle: HANDLE,
    remote_pid: Option<u64>,
}

impl Drop for OSMessagePipe {
    fn drop(&mut self) {
        unsafe { handleapi::CloseHandle(self.read_handle) };
        unsafe { handleapi::CloseHandle(self.write_handle) };
    }
}

impl CrossPlatformMessagePipe for OSMessagePipe {

    fn as_handles(&self) -> Vec<u64> {
        vec![self.read_handle as u64, self.write_handle as u64]
    }

    unsafe fn from_raw_handles(handles: Vec<u64>) -> Result<Self, String> {
        if handles.len() != 2 {
            return Err(format!("Invalid number of handles to construct OSMessagePipe (given: {})", handles.len()));
        }
        Ok(Self {
            read_handle: handles[0] as HANDLE,
            write_handle: handles[1] as HANDLE,
            remote_pid: None,
        })
    }

    fn new() -> Result<(Self, Self), String>
    {
        let mut a_to_b_read = null_mut();
        let mut a_to_b_write = null_mut();
        let res = unsafe { namedpipeapi::CreatePipe(&mut a_to_b_read as *mut HANDLE, &mut a_to_b_write as *mut HANDLE, null_mut(), IPC_MESSAGE_MAX_SIZE.try_into().unwrap()) };
        if res == 0 {
            return Err(format!("CreatePipe() failed with code {}", unsafe { errhandlingapi::GetLastError() }));
        }
        let mut b_to_a_read = null_mut();
        let mut b_to_a_write = null_mut();
        let res = unsafe { namedpipeapi::CreatePipe(&mut b_to_a_read as *mut HANDLE, &mut b_to_a_write as *mut HANDLE, null_mut(), IPC_MESSAGE_MAX_SIZE.try_into().unwrap()) };
        if res == 0 {
            return Err(format!("CreatePipe() failed with code {}", unsafe { errhandlingapi::GetLastError() }));
        }
        let a_to_b = Self {
            read_handle: a_to_b_read,
            write_handle: a_to_b_write,
            remote_pid: None,
        };
        let b_to_a = Self {
            read_handle: b_to_a_read,
            write_handle: b_to_a_write,
            remote_pid: None,
        };
        Ok((a_to_b, b_to_a))
    }

    fn set_remote_pid(&mut self, pid: u64)
    {
        self.remote_pid = Some(pid);
    }

    fn recv(&mut self) -> Result<(Vec<u8>, Option<u64>), String>
    {
        let mut buf = vec![0u8; IPC_MESSAGE_MAX_SIZE.try_into().unwrap()];
        //let res = unsafe { fileapi::ReadFile(self.read_handle, ) };
        Ok((buf, None))
    }

    fn send(&mut self, message: &[u8], handle: Option<u64>) -> Result<(), String>
    {
        Ok(())
    }

}
