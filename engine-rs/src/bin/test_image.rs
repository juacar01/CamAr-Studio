use camar_engine::processing::VideoProcessor;
use camar_engine::types::{ColorRange, ColorSpace, StudioConfig};
use image::{GenericImageView, ImageBuffer, Rgba};

fn main() {
    println!("Testing Rust VideoProcessor pipeline on webcam screenshot...");

    let input_path = r"C:\Users\juaca\.gemini\antigravity\brain\90861736-99a4-4aad-a93b-ed3fd99c41b4\.user_uploaded\media_1792103510438_ecfcefa9.png";
    let img = match image::open(input_path) {
        Ok(img) => img,
        Err(e) => {
            eprintln!("Failed to open image at {}: {}", input_path, e);
            return;
        }
    };

    println!("Image dimensions: {}x{}", img.width(), img.height());

    // Crop the video preview area (y: 75..525, x: 20..860 -> 840 x 450)
    let crop_x = 20;
    let crop_y = 75;
    let crop_w = 840;
    let crop_h = 450;
    let cropped = img.crop_imm(crop_x, crop_y, crop_w, crop_h);
    let width = cropped.width() as usize;
    let height = cropped.height() as usize;

    let mut rgba_bytes = Vec::with_capacity(width * height * 4);
    for y in 0..cropped.height() {
        for x in 0..cropped.width() {
            let pixel = cropped.get_pixel(x, y);
            rgba_bytes.push(pixel[0]);
            rgba_bytes.push(pixel[1]);
            rgba_bytes.push(pixel[2]);
            rgba_bytes.push(255);
        }
    }

    let mut processor = VideoProcessor::new(width, height);

    let mut config = StudioConfig::default();
    config.ai_auto_tune_enabled = true;
    config.ai_lighting_enabled = true;
    config.ai_denoise_enabled = false;
    config.beauty_enabled = false;
    config.exposure = 0.0;
    config.white_balance = 0.0;
    config.brightness = 0.0;
    config.contrast = 1.0;
    config.saturation = 1.0;
    config.sharpness = 0.0;
    config.hue = 0.0;
    config.color_range = ColorRange::Full;
    config.color_space = ColorSpace::Rec709;

    let empty_mask = vec![0u8; width * height];
    let empty_skin_mask = vec![0u8; width * height];
    let empty_feature_mask = vec![0u8; width * height * 4];

    // Warm-up several frames to simulate real-time temporal EMA convergence
    let mut out_bytes = Vec::new();
    for frame_idx in 0..30 {
        let (processed, stats) = processor.process_frame(
            &rgba_bytes,
            &empty_mask,
            &empty_skin_mask,
            &empty_feature_mask,
            &[],
            None,
            &config,
        );
        if frame_idx == 29 {
            out_bytes = processed.to_vec();
            println!(
                "Converged Stats: detected_luma={:.3}, auto_exp={:.3}, auto_wb={:.3}",
                stats.detected_luma, stats.auto_exposure, stats.auto_wb
            );
        }
    }

    let mut output_img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::new(width as u32, height as u32);
    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) * 4;
            output_img.put_pixel(
                x as u32,
                y as u32,
                Rgba([
                    out_bytes[idx],
                    out_bytes[idx + 1],
                    out_bytes[idx + 2],
                    out_bytes[idx + 3],
                ]),
            );
        }
    }

    let out_path = r"d:\proyectos\CamAr Studio\test_rust_output.png";
    output_img.save(out_path).expect("Failed to save output image");
    println!("Successfully saved processed test image to: {}", out_path);
}
