use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::Console::{
    CreatePseudoConsole, ResizePseudoConsole, ClosePseudoConsole,
    HPCON, COORD,
};
use windows_sys::Win32::System::Pipes::CreatePipe;
use crush_types::{Result, CrushError};

pub struct PseudoConsole {
    hpcon: HPCON,
    input_read: HANDLE,
    input_write: HANDLE,
    output_read: HANDLE,
    output_write: HANDLE,
}

impl PseudoConsole {
    pub fn new(width: i16, height: i16) -> Result<Self> {
        let mut input_read: HANDLE = 0;
        let mut input_write: HANDLE = 0;
        let mut output_read: HANDLE = 0;
        let mut output_write: HANDLE = 0;

        // 1. Create standard Win32 anonymous pipes for ConPTY communication
        unsafe {
            if CreatePipe(&mut input_read, &mut input_write, std::ptr::null(), 0) == 0 {
                return Err(CrushError::NamespaceError(format!(
                    "Failed to create ConPTY input pipe: {}",
                    std::io::Error::last_os_error()
                )));
            }

            if CreatePipe(&mut output_read, &mut output_write, std::ptr::null(), 0) == 0 {
                CloseHandle(input_read);
                CloseHandle(input_write);
                return Err(CrushError::NamespaceError(format!(
                    "Failed to create ConPTY output pipe: {}",
                    std::io::Error::last_os_error()
                )));
            }
        }

        // 2. Setup COORD dimensions and initialize Win32 Pseudoconsole
        let size = COORD { X: width, Y: height };
        let mut hpcon: HPCON = 0;

        let result = unsafe {
            CreatePseudoConsole(size, input_read, output_write, 0, &mut hpcon)
        };

        if result != 0 {
            unsafe {
                CloseHandle(input_read);
                CloseHandle(input_write);
                CloseHandle(output_read);
                CloseHandle(output_write);
            }
            return Err(CrushError::NamespaceError(format!(
                "CreatePseudoConsole failed with HRESULT 0x{:x}",
                result
            )));
        }

        Ok(Self {
            hpcon,
            input_read,
            input_write,
            output_read,
            output_write,
        })
    }

    pub fn resize(&self, width: i16, height: i16) -> Result<()> {
        let size = COORD { X: width, Y: height };
        
        let result = unsafe {
            ResizePseudoConsole(self.hpcon, size)
        };

        if result != 0 {
            return Err(CrushError::NamespaceError(format!(
                "ResizePseudoConsole failed with HRESULT 0x{:x}",
                result
            )));
        }

        Ok(())
    }
}

impl Drop for PseudoConsole {
    fn drop(&mut self) {
        unsafe {
            if self.hpcon != 0 {
                ClosePseudoConsole(self.hpcon);
            }
            if self.input_read != 0 { CloseHandle(self.input_read); }
            if self.input_write != 0 { CloseHandle(self.input_write); }
            if self.output_read != 0 { CloseHandle(self.output_read); }
            if self.output_write != 0 { CloseHandle(self.output_write); }
        }
    }
}

unsafe impl Send for PseudoConsole {}
unsafe impl Sync for PseudoConsole {}
