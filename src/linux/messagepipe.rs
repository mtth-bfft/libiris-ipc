use libc::{c_int, c_void};
use core::ptr::null_mut;
use std::io::Error;
use std::convert::TryInto;
use crate::ipc::IPC_MESSAGE_MAX_SIZE;
use crate::messagepipe::CrossPlatformMessagePipe;

pub struct OSMessagePipe {
    file_descriptor: c_int,
}

impl Drop for OSMessagePipe {
    fn drop(&mut self) {
        unsafe { libc::close(self.file_descriptor); }
    }
}

impl CrossPlatformMessagePipe for OSMessagePipe {

    fn as_handles(&self) -> Vec<u64> {
        vec![self.file_descriptor.try_into().unwrap()]
    }

    unsafe fn from_raw_handles(handles: Vec<u64>) -> Result<Self, String> {
        if handles.len() != 1 {
            return Err(format!("Invalid number of handles to construct OSMessagePipe ({} given)", handles.len()));
        }
        Ok(Self {
            file_descriptor: handles[0].try_into().unwrap(),
        })
    }

    fn new() -> Result<(Self, Self), String>
    {
        let mut socks: Vec<c_int> = vec![-1, 2];
        let res = unsafe { libc::socketpair(libc::AF_UNIX, libc::SOCK_SEQPACKET, 0, socks.as_mut_ptr()) };
        if res < 0 {
            return Err(format!("socketpair() failed (error {})", Error::last_os_error()));
        }
        Ok((Self { file_descriptor: socks[0] }, Self { file_descriptor: socks[1] }))
    }

    fn set_remote_pid(&mut self, _pid: u64)
    {
        // Remote PID is not needed on Unix to send a file descriptor, so this is a no-op
    }

    fn recv(&mut self) -> Result<(Vec<u8>, Option<u64>), String>
    {
        let cmsg_space = unsafe { libc::CMSG_SPACE(std::mem::size_of::<c_int>() as u32) } as usize;
        let mut cbuf = vec![0u8; cmsg_space];
        let mut buffer = vec![0u8; IPC_MESSAGE_MAX_SIZE.try_into().unwrap()];
        let msg_iovec = libc::iovec {
            iov_base: buffer.as_mut_ptr() as *mut c_void,
            iov_len: buffer.len(),
        };
        let mut msg = libc::msghdr {
            msg_name: null_mut(), // socket is already connected, no need for this
            msg_namelen: 0,
            msg_iov: &msg_iovec as *const libc::iovec as *mut libc::iovec, // mut is not used here, just required by API
            msg_iovlen: 1,
            msg_control: cbuf.as_mut_ptr() as *mut c_void,
            msg_controllen: cmsg_space,
            msg_flags: 0, // unused
        };
        let res = unsafe { libc::recvmsg(self.file_descriptor, &mut msg as *mut libc::msghdr, libc::MSG_NOSIGNAL | libc::MSG_CMSG_CLOEXEC | libc::MSG_WAITALL) };
        if res < 0 {
            return Err(format!("recvmsg() failed (error {})", Error::last_os_error()));
        }
        let fd = if msg.msg_controllen > 0 {
            let cmsghdr = unsafe { libc::CMSG_FIRSTHDR(&msg as *const libc::msghdr) };
            if cmsghdr.is_null() {
                return Err("Failed to parse ancillary data from worker request".to_owned());
            }
            let (clevel, ctype) = unsafe { ((*cmsghdr).cmsg_level, (*cmsghdr).cmsg_type) };
            if (clevel, ctype) != (libc::SOL_SOCKET, libc::SCM_RIGHTS) {
                return Err(format!("Unexpected ancillary data level={} type={} received with worker request", clevel, ctype));
            }
            let fd = unsafe { *(libc::CMSG_DATA(cmsghdr) as *const c_int) };
            Some(fd)
        } else {
            None
        };
        if (msg.msg_flags & libc::MSG_CTRUNC) != 0 {
            if let Some(fd) = fd {
                unsafe { libc::close(fd) };
            }
            return Err("recvmsg() failed due to truncated ancillary data".to_owned());
        }
        if (msg.msg_flags & libc::MSG_TRUNC) != 0 {
            if let Some(fd) = fd {
                unsafe { libc::close(fd) };
            }
            return Err("recvmsg() failed due to truncated message".to_owned());
        }
        let fd = match fd {
            None => None,
            Some(n) => Some(n.try_into().unwrap()),
        };
        buffer.truncate(res.try_into().unwrap());
        Ok((buffer, fd))
    }

    fn send(&mut self, message: &[u8], handle: Option<u64>) -> Result<(), String>
    {
        let cmsg_space = unsafe { libc::CMSG_SPACE(std::mem::size_of::<c_int>() as u32) } as usize;
        let mut cbuf = vec![0u8; cmsg_space];
        let msg_iovec = libc::iovec {
            iov_base: message.as_ptr() as *mut c_void, // mut is not used here, just required because iovec is used by recvmsg too
            iov_len: message.len(),
        };
        let msg = libc::msghdr {
            msg_name: null_mut(), // socket is already connected, no need for this
            msg_namelen: 0,
            msg_iov: &msg_iovec as *const libc::iovec as *mut libc::iovec, // mut is not really used here either
            msg_iovlen: 1,
            msg_control: cbuf.as_mut_ptr() as *mut c_void,
            msg_controllen: cmsg_space * (if handle.is_some() { 1 } else { 0 }),
            msg_flags: 0, // unused
        };
        if let Some(file_descriptor) = handle {
            let file_descriptor: c_int = match file_descriptor.try_into() {
                Ok(n) => n,
                Err(_) => return Err(format!("sendmsg() called with invalid file descriptor: {}", file_descriptor)),
            };
            let cmsghdr = unsafe { libc::CMSG_FIRSTHDR(&msg as *const _ as *mut libc::msghdr) };
            unsafe {
                (*cmsghdr).cmsg_level = libc::SOL_SOCKET;
                (*cmsghdr).cmsg_type = libc::SCM_RIGHTS;
                (*cmsghdr).cmsg_len = libc::CMSG_LEN(std::mem::size_of::<c_int>() as u32) as usize;
                std::ptr::copy_nonoverlapping(&file_descriptor as *const c_int, libc::CMSG_DATA(cmsghdr) as *mut c_int, 1);
            }
        }
        let res = unsafe { libc::sendmsg(self.file_descriptor, &msg as *const libc::msghdr, libc::MSG_NOSIGNAL) };
        if res < 0 {
            return Err(format!("sendmsg() failed with error: {}", Error::last_os_error()));
        }
        Ok(())
    }

}
