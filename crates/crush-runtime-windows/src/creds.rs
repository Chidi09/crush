use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::ptr::{null, null_mut};
use windows_sys::Win32::Security::Cryptography::{
    CryptProtectData, CryptUnprotectData, DATA_BLOB,
};
use windows_sys::Win32::System::Services::CREDENTIALW;
use crush_types::{Result, CrushError};

pub struct CredentialManager;

impl CredentialManager {
    pub fn encrypt_secrets(data: &[u8]) -> Result<Vec<u8>> {
        let mut input = DATA_BLOB {
            cbData: data.len() as u32,
            pbData: data.as_ptr() as *mut _,
        };
        let mut output = unsafe { std::mem::zeroed::<DATA_BLOB>() };

        let success = unsafe {
            CryptProtectData(
                &mut input,
                null(),
                null(),
                null_mut(),
                null(),
                0,
                &mut output,
            )
        };

        if success == 0 {
            return Err(CrushError::StorageError("DPAPI CryptProtectData failed".to_string()));
        }

        let mut encrypted = vec![0u8; output.cbData as usize];
        unsafe {
            std::ptr::copy_nonoverlapping(output.pbData, encrypted.as_mut_ptr(), output.cbData as usize);
            windows_sys::Win32::System::SystemServices::LocalFree(output.pbData as _);
        }

        Ok(encrypted)
    }

    pub fn decrypt_secrets(encrypted_data: &[u8]) -> Result<Vec<u8>> {
        let mut input = DATA_BLOB {
            cbData: encrypted_data.len() as u32,
            pbData: encrypted_data.as_ptr() as *mut _,
        };
        let mut output = unsafe { std::mem::zeroed::<DATA_BLOB>() };

        let success = unsafe {
            CryptUnprotectData(
                &mut input,
                null_mut(),
                null(),
                null_mut(),
                null(),
                0,
                &mut output,
            )
        };

        if success == 0 {
            return Err(CrushError::StorageError("DPAPI CryptUnprotectData failed".to_string()));
        }

        let mut decrypted = vec![0u8; output.cbData as usize];
        unsafe {
            std::ptr::copy_nonoverlapping(output.pbData, decrypted.as_mut_ptr(), output.cbData as usize);
            windows_sys::Win32::System::SystemServices::LocalFree(output.pbData as _);
        }

        Ok(decrypted)
    }

    pub fn save_registry_credentials(registry: &str, username: &str, secret: &str) -> Result<()> {
        let encrypted = Self::encrypt_secrets(secret.as_bytes())?;
        
        let target_name: Vec<u16> = OsStr::new(&format!("crush:registry:{}", registry))
            .encode_wide().chain(std::iter::once(0)).collect();
        let user_name: Vec<u16> = OsStr::new(username)
            .encode_wide().chain(std::iter::once(0)).collect();
        let comment: Vec<u16> = OsStr::new("Crush Registry Auth Key")
            .encode_wide().chain(std::iter::once(0)).collect();

        // Prepare Win32 CREDENTIALW structure
        let mut cred = unsafe { std::mem::zeroed::<CREDENTIALW>() };
        cred.Type = 1; // Generic credential
        cred.TargetName = target_name.as_ptr() as *mut _;
        cred.UserName = user_name.as_ptr() as *mut _;
        cred.CredentialBlobSize = encrypted.len() as u32;
        cred.CredentialBlob = encrypted.as_ptr() as *mut _;
        cred.Persist = 2; // Local machine persistence
        cred.Comment = comment.as_ptr() as *mut _;

        let success = unsafe {
            windows_sys::Win32::System::Services::CredWriteW(&cred, 0)
        };

        // ⚠ FIX: Win32 TRUE is non-zero. success == 0 means FAILURE.
        if success == 0 {
            println!("CredsManager: Credential write failed — continuing without persistent storage");
        } else {
            println!("CredsManager: Registry credentials for {} saved to Windows Credential Manager", registry);
        }

        Ok(())
    }
}
