#[cfg(target_os = "windows")]
mod windows_impl {
    use std::io::Write;
    use windows_sys::Win32::Foundation::*;
    use windows_sys::Win32::System::Pipes::*;
    use windows_sys::Win32::System::Threading::*;
    use windows_sys::Win32::Storage::FileSystem::*;

    const PIPE_NAME_STR: &str = "\\\\.\\pipe\\MediaForge";

    fn mutex_name_wide() -> Vec<u16> {
        "Global\\MediaForge_SingleInstance\0"
            .encode_utf16()
            .collect()
    }

    fn pipe_name_wide() -> Vec<u16> {
        "\\\\.\\pipe\\MediaForge\0"
            .encode_utf16()
            .collect()
    }

    pub struct SingleInstanceGuard {
        _handle: HANDLE,
    }

    /// Try to acquire the single instance lock.
    pub fn try_acquire(files: &[String]) -> Result<Option<SingleInstanceGuard>, String> {
        unsafe {
            let name = mutex_name_wide();
            let handle = CreateMutexW(std::ptr::null(), 1, name.as_ptr());

            if handle.is_null() {
                return Err("Failed to create mutex".to_string());
            }

            let last_error = GetLastError();
            if last_error == ERROR_ALREADY_EXISTS {
                CloseHandle(handle);
                if !files.is_empty() {
                    send_files_to_running_instance(files);
                }
                return Ok(None);
            }

            Ok(Some(SingleInstanceGuard { _handle: handle }))
        }
    }

    fn send_files_to_running_instance(files: &[String]) {
        let payload = files.join("\n");
        if let Ok(mut stream) = std::fs::OpenOptions::new()
            .write(true)
            .open(PIPE_NAME_STR)
        {
            let _ = stream.write_all(payload.as_bytes());
        }
    }

    /// Start a background thread that listens for file paths from new instances
    pub fn start_pipe_listener(sender: crossbeam_channel::Sender<Vec<String>>) {
        std::thread::spawn(move || loop {
            match create_named_pipe_server() {
                Ok(handle) => {
                    if connect_client(handle) {
                        let mut buffer = vec![0u8; 65536];
                        let bytes_read = read_pipe(handle, &mut buffer);
                        if bytes_read > 0 {
                            let data = String::from_utf8_lossy(&buffer[..bytes_read]);
                            let files: Vec<String> = data
                                .lines()
                                .filter(|l| !l.is_empty())
                                .map(String::from)
                                .collect();
                            if !files.is_empty() {
                                let _ = sender.send(files);
                            }
                        }
                        close_pipe(handle);
                    } else {
                        unsafe {
                            CloseHandle(handle);
                        }
                    }
                }
                Err(_) => {
                    std::thread::sleep(std::time::Duration::from_secs(1));
                }
            }
        });
    }

    fn create_named_pipe_server() -> Result<HANDLE, ()> {
        let name_wide = pipe_name_wide();

        unsafe {
            let handle = CreateNamedPipeW(
                name_wide.as_ptr(),
                PIPE_ACCESS_INBOUND,
                PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
                1,
                65536,
                65536,
                0,
                std::ptr::null(),
            );

            if handle == INVALID_HANDLE_VALUE {
                return Err(());
            }

            Ok(handle)
        }
    }

    fn connect_client(handle: HANDLE) -> bool {
        unsafe { ConnectNamedPipe(handle, std::ptr::null_mut()) != 0 }
    }

    fn read_pipe(handle: HANDLE, buffer: &mut [u8]) -> usize {
        let mut bytes_read: u32 = 0;
        unsafe {
            let result = ReadFile(
                handle,
                buffer.as_mut_ptr().cast(),
                buffer.len() as u32,
                &mut bytes_read,
                std::ptr::null_mut(),
            );
            if result != 0 {
                bytes_read as usize
            } else {
                0
            }
        }
    }

    fn close_pipe(handle: HANDLE) {
        unsafe {
            DisconnectNamedPipe(handle);
            CloseHandle(handle);
        }
    }
}

#[cfg(target_os = "windows")]
pub use windows_impl::*;

#[cfg(not(target_os = "windows"))]
mod unix_impl {
    pub struct SingleInstanceGuard {}

    pub fn try_acquire(_files: &[String]) -> Result<Option<SingleInstanceGuard>, String> {
        // Dummy implementation for unix
        Ok(Some(SingleInstanceGuard {}))
    }

    pub fn start_pipe_listener(_sender: crossbeam_channel::Sender<Vec<String>>) {}
}

#[cfg(not(target_os = "windows"))]
pub use unix_impl::*;
