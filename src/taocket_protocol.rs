use std::path::PathBuf;
use wry::http::{Request, Response, header::CONTENT_TYPE};

pub fn handle_custom_protocol(request: Request<Vec<u8>>, root: &PathBuf) -> Response<Vec<u8>> {
    match get_response(request, root) {
        Ok(response) => response.map(Into::into),
        Err(e) => {
            eprintln!("Protocol error: {}", e);
            Response::builder()
                .header(CONTENT_TYPE, "text/plain")
                .status(500)
                .body(e.to_string().as_bytes().to_vec())
                .unwrap()
                .map(Into::into)
        }
    }
}

pub fn get_response(
    request: Request<Vec<u8>>,
    root: &PathBuf,
) -> Result<Response<Vec<u8>>, Box<dyn std::error::Error>> {
    let path = request.uri().path();

    let file_path = if path == "/" {
        "index.html"
    } else {
        &path[1..]
    };

    let full_path = root.join(file_path);
    let canonical_path = std::fs::canonicalize(&full_path)?;

    let canonical_root = std::fs::canonicalize(root)?;
    if !canonical_path.starts_with(&canonical_root) {
        return Err("Access denied: path outside root directory".into());
    }

    let content = std::fs::read(&canonical_path)?;

    let mime_type = get_mime_type(file_path);

    Response::builder()
        .header(CONTENT_TYPE, mime_type)
        .body(content)
        .map_err(Into::into)
}

fn get_mime_type(path: &str) -> &'static str {
    if path.ends_with(".html") || path == "/" {
        "text/html"
    } else if path.ends_with(".js") || path.ends_with(".mjs") {
        "text/javascript"
    } else if path.ends_with(".css") {
        "text/css"
    } else if path.ends_with(".png") {
        "image/png"
    } else if path.ends_with(".jpg") || path.ends_with(".jpeg") {
        "image/jpeg"
    } else if path.ends_with(".svg") {
        "image/svg+xml"
    } else if path.ends_with(".gif") {
        "image/gif"
    } else if path.ends_with(".webp") {
        "image/webp"
    } else if path.ends_with(".wasm") {
        "application/wasm"
    } else if path.ends_with(".json") {
        "application/json"
    } else if path.ends_with(".woff") {
        "font/woff"
    } else if path.ends_with(".woff2") {
        "font/woff2"
    } else if path.ends_with(".ttf") {
        "font/ttf"
    } else if path.ends_with(".otf") {
        "font/otf"
    } else if path.ends_with(".eot") {
        "application/vnd.ms-fontobject"
    } else if path.ends_with(".ico") {
        "image/x-icon"
    } else if path.ends_with(".xml") {
        "application/xml"
    } else if path.ends_with(".txt") {
        "text/plain"
    } else if path.ends_with(".pdf") {
        "application/pdf"
    } else if path.ends_with(".zip") {
        "application/zip"
    } else {
        "application/octet-stream"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mime_type_detection() {
        assert_eq!(get_mime_type("index.html"), "text/html");
        assert_eq!(get_mime_type("script.js"), "text/javascript");
        assert_eq!(get_mime_type("module.mjs"), "text/javascript");
        assert_eq!(get_mime_type("style.css"), "text/css");
        assert_eq!(get_mime_type("image.png"), "image/png");
        assert_eq!(get_mime_type("photo.jpg"), "image/jpeg");
        assert_eq!(get_mime_type("icon.svg"), "image/svg+xml");
        assert_eq!(get_mime_type("module.wasm"), "application/wasm");
        assert_eq!(get_mime_type("data.json"), "application/json");
        assert_eq!(get_mime_type("font.woff"), "font/woff");
        assert_eq!(get_mime_type("font.woff2"), "font/woff2");
        assert_eq!(get_mime_type("favicon.ico"), "image/x-icon");
        assert_eq!(get_mime_type("unknown.xyz"), "application/octet-stream");
    }

    #[test]
    fn test_root_path_mime_type() {
        assert_eq!(get_mime_type("/"), "text/html");
    }
}
