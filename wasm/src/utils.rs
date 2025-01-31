pub fn set_panic_hook() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

pub fn process_lines(s: &str, f: impl Fn(&str) -> String) -> String {
    s.lines()
        .map(f)
        .collect::<Vec<String>>()
        .join("\n")
}

