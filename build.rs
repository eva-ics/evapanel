use std::fs;

fn main() {
    #[cfg(target_os = "windows")]
    winresource::WindowsResource::new()
        .set_icon("assets/evapanel.ico")
        .compile()
        .unwrap();
    let icon = ico::IconImage::read_png(fs::File::open("assets/evapanel.png").unwrap()).unwrap();
    fs::create_dir_all("res").unwrap();
    fs::write("res/evapanel.rgba", icon.rgba_data()).unwrap();
}
