use image::{ImageBuffer, Rgba, RgbaImage};

fn png(width: u32, height: u32) -> Vec<u8> {
    let image: RgbaImage = ImageBuffer::from_pixel(width, height, Rgba([1, 2, 3, 255]));
    let mut output = std::io::Cursor::new(Vec::new());
    image
        .write_to(&mut output, image::ImageFormat::Png)
        .expect("a generated PNG must encode");
    output.into_inner()
}

#[test]
fn hd_square_skins_are_accepted() {
    for size in [64, 128, 256, 512, 1024, 2048] {
        assert!(
            oneclient_app::validate_local_skin_png(&png(size, size)).is_ok(),
            "{size}x{size}"
        );
    }
}

#[test]
fn cape_power_of_two_2x1_sizes_are_accepted() {
    for size in [64, 128, 256, 512, 1024, 2048] {
        assert!(
            oneclient_app::validate_local_cape_png(&png(size, size / 2)).is_ok(),
            "{size}x{}",
            size / 2
        );
    }
}

#[test]
fn skins_and_capes_are_not_interchangeable() {
    assert!(oneclient_app::validate_local_skin_png(&png(64, 32)).is_err());
    assert!(oneclient_app::validate_local_cape_png(&png(64, 64)).is_err());
}
