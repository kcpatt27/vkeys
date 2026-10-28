//! Child-process plumbing: one pipe for stdout+stderr so the merge order is the
//! child's order (Python: `stderr=subprocess.STDOUT`). Cross-platform via
//! `os_pipe`.

use std::io;

pub fn merged_pipe() -> io::Result<(os_pipe::PipeReader, os_pipe::PipeWriter, os_pipe::PipeWriter)> {
    let (reader, writer) = os_pipe::pipe()?;
    let writer2 = writer.try_clone()?;
    Ok((reader, writer, writer2))
}
