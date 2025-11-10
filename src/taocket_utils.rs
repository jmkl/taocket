use std::path::{Path, PathBuf};

pub fn resolve_frontend_path<P: AsRef<Path>>(path_to: P) -> PathBuf {
    let absolute = std::env::current_dir().unwrap().join(path_to);
    normalize_path(&absolute)
    // Path::new(env!("CARGO_MANIFEST_DIR"))
    //     .join(path_to)
    //     .to_path_buf()

    // let p = Path::new(path_to.as_ref());
    // if p.is_absolute() {
    //     p.to_path_buf()
    // } else {
    //     std::env::current_dir().unwrap().join(p)
    // }
}
fn normalize_path(path: &Path) -> PathBuf {
    let mut components = Vec::new();

    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                components.pop();
            }
            std::path::Component::CurDir => {}
            comp => components.push(comp),
        }
    }

    components.iter().collect()
}
