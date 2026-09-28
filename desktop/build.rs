fn main() {
    #[cfg(windows)]
    winresource::WindowsResource::new()
        .set_icon("assets/app-icons/icon.ico")
        .compile()
        .expect("failed to embed Windows icon");
}