use std::panic;

#[macro_export]
macro_rules! define_schema {
    ($schema:expr) => {
        const _SCHEMA_STR: &str = concat!($schema, "\0");
        #[no_mangle]
        pub static SCHEMA: [u8; _SCHEMA_STR.len()] = match _SCHEMA_STR.as_bytes().first_chunk() {
            Some(chunk) => *chunk,
            None => unreachable!(),
        };
    };
}

extern "C" {
    #[link_name = "render"]
    fn c_render(s: *const std::ffi::c_char);
}

pub fn render(s: impl Into<Vec<u8>>) {
    unsafe {
        match std::ffi::CString::new(s) {
            Ok(s) => c_render(s.as_ptr()),
            Err(_) => c_render(
                std::ffi::CStr::from_bytes_with_nul_unchecked(
                    b"Error: String contains interior null bytes\0",
                )
                .as_ptr(),
            ),
        }
    }
}

pub fn setup_panic_hook() {
    panic::set_hook(Box::new(|panic_info| {
        let msg = format!("Panic: {}", panic_info);
        let text = svg::node::element::Text::new(msg)
            .set("x", "50")
            .set("y", "50")
            .set("text-anchor", "middle")
            .set("dominant-baseline", "middle")
            .set("fill", "red");

        let document = svg::Document::new()
            .set("width", "1000")
            .set("height", "1000")
            .set("viewBox", "0 0 1000 1000")
            .add(text);

        render(document.to_string());
    }));
}
