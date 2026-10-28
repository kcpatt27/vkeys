//! Child-process plumbing: one pipe for stdout+stderr so the merge order is the
//! child's order (Python: `stderr=subprocess.STDOUT`).

use std::fs::File;
use std::os::windows::io::{FromRawHandle, OwnedHandle};

use windows::core::Error;
use windows::Win32::Foundation::{DuplicateHandle, DUPLICATE_SAME_ACCESS, HANDLE};
use windows::Win32::System::Pipes::CreatePipe;
use windows::Win32::System::Threading::GetCurrentProcess;

pub fn merged_pipe() -> Result<(File, OwnedHandle, OwnedHandle), Error> {
    unsafe {
        let mut read = HANDLE::default();
        let mut write = HANDLE::default();
        CreatePipe(&mut read, &mut write, None, 0)?;

        let mut write2 = HANDLE::default();
        DuplicateHandle(
            GetCurrentProcess(),
            write,
            GetCurrentProcess(),
            &mut write2,
            0,
            false,
            DUPLICATE_SAME_ACCESS,
        )?;

        Ok((
            File::from_raw_handle(read.0),
            OwnedHandle::from_raw_handle(write.0),
            OwnedHandle::from_raw_handle(write2.0),
        ))
    }
}
