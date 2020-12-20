use std::io::Write;
use thin_trait_objects::OwnedFileHandle;

fn main() -> std::io::Result<()> {
    let mut handle = OwnedFileHandle::new(std::io::stdout());

    writeln!(handle, "Hello, World!")?;
    handle.flush()?;

    Ok(())
}
