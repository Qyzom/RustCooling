use crate::config::AppConfig;
use log::info;
use std::fs::File;
use std::path::PathBuf;

pub struct SingleInstanceLock {
    _file: File,
}

impl SingleInstanceLock {
    pub fn acquire(name: &str) -> Option<Self> {
        let path = Self::lock_file_path(name);
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let file = File::create(&path).ok()?;

        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;
            let fd = file.as_raw_fd();
            let ret = unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) };
            if ret != 0 {
                return None;
            }
        }

        #[cfg(windows)]
        {
            use std::os::windows::io::AsRawHandle;
            use windows_sys::Win32::Foundation::HANDLE;
            use windows_sys::Win32::Storage::FileSystem::{LockFile, LOCKFILE_EXCLUSIVE_LOCK, LOCKFILE_FAIL_IMMEDIATELY};
            let handle = file.as_raw_handle() as HANDLE;
            let ret = unsafe {
                LockFile(handle, 0, 0, 1, 0)
            };
            if ret == 0 {
                return None;
            }
        }

        info!("Single instance lock acquired: {:?}", path);
        Some(Self { _file: file })
    }

    fn lock_file_path(name: &str) -> PathBuf {
        AppConfig::config_dir().join(format!("{}.lock", name))
    }
}
