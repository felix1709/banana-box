use image::{GenericImageView, RgbaImage};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

const FRAME_SIZE: u32 = 256;
const FRAME_COUNT: u32 = 12;

#[test]
fn banana_sprite_has_twelve_square_frames_and_transparent_edges() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let sprite_path = root.join("src/assets/banana/banana-peel-sprite.webp");
    let closed_path = root.join("src/assets/banana/banana-closed-mirrored-approved.png");
    let open_path = root.join("src/assets/banana/banana-open-mirrored-approved.png");
    let hash_path = root.join("docs/design/banana-open-mirrored-approved.sha256");

    let sprite = image::open(&sprite_path).expect("banana sprite must exist");
    let approved_open = image::open(&open_path)
        .expect("approved open endpoint must exist")
        .to_rgba8();
    let approved_closed = image::open(&closed_path)
        .expect("approved mirrored closed endpoint must exist")
        .to_rgba8();

    assert_eq!(sprite.dimensions(), (FRAME_SIZE * FRAME_COUNT, FRAME_SIZE));
    assert_eq!(approved_closed.dimensions(), (FRAME_SIZE, FRAME_SIZE));
    assert_eq!(approved_open.dimensions(), (FRAME_SIZE, FRAME_SIZE));
    assert!(sprite.color().has_alpha(), "sprite must preserve alpha");

    let recorded_hash =
        fs::read_to_string(hash_path).expect("approved endpoint hash record must exist");
    let actual_hash = format!(
        "{:x}",
        Sha256::digest(fs::read(open_path).expect("endpoint must be readable"))
    );
    assert!(
        recorded_hash.starts_with(&actual_hash),
        "endpoint hash record must match the approved file"
    );

    let frames: Vec<RgbaImage> = (0..FRAME_COUNT)
        .map(|frame| {
            sprite
                .crop_imm(frame * FRAME_SIZE, 0, FRAME_SIZE, FRAME_SIZE)
                .to_rgba8()
        })
        .collect();

    for (index, frame) in frames.iter().enumerate() {
        let bbox = alpha_bbox(frame).expect("frame must contain visible pixels");
        assert!(
            bbox.0 >= 46 && bbox.1 <= 209 && bbox.2 >= 46 && bbox.3 <= 209,
            "frame {index} must keep transparent padding on all sides"
        );
        assert_transparent_border(frame);
        assert_centroid_within(frame, &approved_open, 8.0);
    }

    for index in 0..(FRAME_COUNT - 1) as usize {
        let changed_ratio = changed_pixel_ratio(&frames[index], &frames[index + 1]);
        assert!(
            (0.005..=0.28).contains(&changed_ratio),
            "adjacent frame {index} change must be visible but bounded: {changed_ratio}"
        );
    }

    assert_visible_pixels_equal(
        &frames[0],
        &approved_closed,
        "frame 0 must equal the approved mirrored closed endpoint",
    );
    assert_visible_pixels_equal(
        &frames[11],
        &approved_open,
        "frame 11 must equal the approved endpoint",
    );
}

fn assert_visible_pixels_equal(actual: &RgbaImage, expected: &RgbaImage, message: &str) {
    for (actual, expected) in actual.pixels().zip(expected.pixels()) {
        assert_eq!(actual[3], expected[3], "{message}: alpha must match");
        if actual[3] != 0 {
            assert_eq!(actual, expected, "{message}: visible pixels must match");
        }
    }
}

fn alpha_bbox(image: &RgbaImage) -> Option<(u32, u32, u32, u32)> {
    let mut min_x = FRAME_SIZE;
    let mut max_x = 0;
    let mut min_y = FRAME_SIZE;
    let mut max_y = 0;
    let mut found = false;

    for (x, y, pixel) in image.enumerate_pixels() {
        if pixel[3] == 0 {
            continue;
        }
        found = true;
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }

    found.then_some((min_x, max_x, min_y, max_y))
}

fn assert_transparent_border(image: &RgbaImage) {
    for coordinate in 0..FRAME_SIZE {
        assert_eq!(image.get_pixel(coordinate, 0)[3], 0);
        assert_eq!(image.get_pixel(coordinate, FRAME_SIZE - 1)[3], 0);
        assert_eq!(image.get_pixel(0, coordinate)[3], 0);
        assert_eq!(image.get_pixel(FRAME_SIZE - 1, coordinate)[3], 0);
    }
}

fn assert_centroid_within(frame: &RgbaImage, endpoint: &RgbaImage, tolerance: f64) {
    let (frame_x, frame_y) = alpha_centroid(frame);
    let (endpoint_x, endpoint_y) = alpha_centroid(endpoint);

    assert!((frame_x - endpoint_x).abs() <= tolerance);
    assert!((frame_y - endpoint_y).abs() <= tolerance);
}

fn alpha_centroid(image: &RgbaImage) -> (f64, f64) {
    let mut total_alpha = 0_f64;
    let mut weighted_x = 0_f64;
    let mut weighted_y = 0_f64;

    for (x, y, pixel) in image.enumerate_pixels() {
        let alpha = f64::from(pixel[3]);
        total_alpha += alpha;
        weighted_x += f64::from(x) * alpha;
        weighted_y += f64::from(y) * alpha;
    }

    (weighted_x / total_alpha, weighted_y / total_alpha)
}

fn changed_pixel_ratio(left: &RgbaImage, right: &RgbaImage) -> f64 {
    let changed = left
        .pixels()
        .zip(right.pixels())
        .filter(|(left, right)| left != right && (left[3] != 0 || right[3] != 0))
        .count();

    changed as f64 / f64::from(FRAME_SIZE * FRAME_SIZE)
}
