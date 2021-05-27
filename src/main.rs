use image::GenericImageView;
use image::Pixel;
use image::Rgba;
use imageproc::drawing::draw_text_mut;
use imageproc::rect::Rect;

//bollow from https://stackoverflow.com/a/42139980/558348
fn get_resolutions() -> Vec<(i32, i32)> {
    let (screen_count, display) = unsafe {
        let display = x11::xlib::XOpenDisplay(std::ptr::null_mut());
        if display.is_null() {
            (0, std::ptr::null_mut())
        } else {
            (x11::xlib::XScreenCount(display), display)
        }
    };

    let mut retval = Vec::new();

    for i in 0..screen_count {
        unsafe {
            let screen = x11::xlib::XScreenOfDisplay(display, i);
            if screen.is_null() == false {
                retval.push(((*screen).width, (*screen).height));
            }
        }
    }
    retval
}

//https://github.com/image-rs/imageproc/issues/261#issuecomment-379575918
pub fn draw_blended_rect_mut<I>(image: &mut I, rect: Rect, color: I::Pixel)
where
    I: image::GenericImage,
    I::Pixel: 'static,
{
    let image_bounds = Rect::at(0, 0).of_size(image.width(), image.height());
    if let Some(intersection) = image_bounds.intersect(rect) {
        for dy in 0..intersection.height() {
            for dx in 0..intersection.width() {
                let x = intersection.left() as u32 + dx;
                let y = intersection.top() as u32 + dy;
                let mut pixel = image.get_pixel(x, y); // added
                pixel.blend(&color); // added
                unsafe {
                    image.unsafe_put_pixel(x, y, pixel); // changed
                }
            }
        }
    }
}

fn calculate_resize_of_image(screen_size: (i32, i32), img_size: (u32, u32)) -> (u32, u32) {
    let screen_height_ratio = screen_size.1 as f32 / screen_size.0 as f32;
    let img_height_ratio = img_size.1 as f32 / img_size.0 as f32;

    let scale_factor = if screen_height_ratio > img_height_ratio {
        screen_size.1 as f32 / img_size.1 as f32
    } else {
        screen_size.0 as f32 / img_size.0 as f32
    };

    (
        (img_size.0 as f32 * scale_factor) as u32,
        (img_size.1 as f32 * scale_factor) as u32,
    )
}

fn calculate_crop_x_y(screen_size: (i32, i32), img_size: (u32, u32)) -> (u32, u32) {
    let (img_width, img_height) = img_size;
    let (sc_width, sc_height) = screen_size;

    let x = if img_width > sc_width as u32 {
        (img_width - sc_width as u32) / 2
    } else {
        0
    };

    let y = if img_height > sc_height as u32 {
        (img_height - sc_height as u32) / 2
    } else {
        0
    };
    (x, y)
}

fn main() {
    let screen_size = if let Some(size) = get_resolutions().into_iter().next() {
        size
    } else {
        eprintln!("fail to get screen resolution");
        std::process::exit(1);
    };

    //get image filename
    let img_path = if let Some(img_path) = std::env::args().skip(1).next() {
        img_path
    } else {
        eprintln!("usage : ilock-rs <img_path>");
        std::process::exit(1);
    };

    let out_path = if let Some(out_path) = std::env::args().skip(2).next() {
        out_path
    } else {
        eprintln!("usage : ilock-rs <img_path> <out_path>");
        std::process::exit(1);
    };

    let img = if let Ok(img) = image::open(std::path::PathBuf::from(&img_path)) {
        img
    } else {
        eprintln!("Fail to load image from {}", img_path);
        std::process::exit(1);
    };

    let (to_img_width, to_img_height) =
        calculate_resize_of_image(screen_size, (img.width(), img.height()));

    let mut img_buffer = image::imageops::resize(
        &img,
        to_img_width,
        to_img_height,
        image::imageops::FilterType::Nearest,
    );

    let (crop_x, crop_y) = calculate_crop_x_y(screen_size, (to_img_width, to_img_height));

    let cropped = image::imageops::crop(
        &mut img_buffer,
        crop_x,
        crop_y,
        screen_size.0 as u32,
        screen_size.1 as u32,
    )
    .to_image();

    let mut cropped = image::imageops::blur(&cropped, 5.0);

    let font = Vec::from(include_bytes!("nerd.ttf") as &[u8]);
    let font = rusttype::Font::try_from_vec(font).unwrap();

    let scale = rusttype::Scale { x: 200.0, y: 200.0 };

    let candidate = vec!["", "", ""];

    use rand::RngCore;
    let lock = candidate[rand::thread_rng().next_u32() as usize % candidate.len()];

    draw_blended_rect_mut(
        &mut cropped,
        Rect::at(30, screen_size.1 - 110).of_size(300, 80),
        Rgba([0u8, 0u8, 0u8, 128u8]),
    );

    draw_text_mut(
        &mut cropped,
        Rgba([255u8, 255u8, 255u8, 128u8]),
        screen_size.0 as u32 / 2 - 60,
        screen_size.1 as u32 / 2 - 100,
        scale,
        &font,
        lock,
    );

    cropped.save(out_path).unwrap();
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculate_resize_of_image_test() {
        let (w, h) = calculate_resize_of_image((200, 110), (100,50));
        assert_eq!(h, 110);
        assert_eq!(w, 220);

        let (w, h) = calculate_resize_of_image((200, 90), (100,50));
        assert_eq!(w, 200);
        assert_eq!(h, 100);

        let (w, h) = calculate_resize_of_image((100, 50), (200,110));
        assert_eq!(h, 55);
        assert_eq!(w, 100);

        let (w, h) = calculate_resize_of_image((100, 50), (200,90));
        assert_eq!(w, 111);
        assert_eq!(h, 50);
    }

    #[test]
    fn calculate_crop_x_y_test(){
        let (x,y) = calculate_crop_x_y((200, 110), (210, 150));
        assert_eq!(x, 5);
        assert_eq!(y, 20);
    }

}
