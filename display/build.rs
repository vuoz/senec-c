use std::{env, path::PathBuf};
fn main() -> std::io::Result<()> {
    let manifest_dir = match env::var("CARGO_MANIFEST_DIR") {
        Ok(val) => val,
        Err(err) => {
            eprintln!("Error getting CARGO_MANIFEST_DIR: {}", err);
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to get CARGO_MANIFEST_DIR",
            ));
        }
    };

    let manifest = PathBuf::from(manifest_dir);
    // assumes display/types.proto lives next to build.rs
    let proto = manifest.join("types.proto");

    prost_build::compile_protos(&[proto.to_str().unwrap()], &[manifest.to_str().unwrap()])?;
    Ok(())
}
