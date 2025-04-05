fn main() {
    #[cfg(target_os = "windows")]
    winresource::WindowsResource::new()
        .set_icon("assets/evapanel.ico")
        .compile()
        .unwrap();
}
