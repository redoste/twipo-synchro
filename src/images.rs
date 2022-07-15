use async_std::io;

use std::collections::HashMap;
use std::error::Error;

use futures::prelude::*;

struct ImageOffset {
    name: &'static str,
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    rotate: bool,
}

impl ImageOffset {
    const fn new(name: &'static str, x: u32, y: u32, w: u32, h: u32, rotate: bool) -> ImageOffset {
        ImageOffset {
            name,
            x,
            y,
            w,
            h,
            rotate,
        }
    }
}

const IMAGES_OFFSET: [ImageOffset; 9] = [
    ImageOffset::new("bg", 176, 0, 613, 1090, false),
    ImageOffset::new("header", 0, 0, 74, 1090, true),
    ImageOffset::new("unsel", 74, 0, 93, 1090, true),
    ImageOffset::new("sel_a", 1949, 7, 94, 294, true),
    ImageOffset::new("sel_b", 2045, 11, 94, 286, true),
    ImageOffset::new("sel_c", 2141, 7, 94, 288, true),
    ImageOffset::new("sel_d", 2237, 7, 94, 276, true),
    ImageOffset::new("reply", 1115, 1545, 334, 94, false),
    ImageOffset::new("send", 1493, 1545, 334, 94, false),
];

pub type ImageList = HashMap<String, Vec<u8>>;

fn add_to_imagelist(
    image_list: &mut ImageList,
    name: String,
    image: image::DynamicImage,
) -> Result<(), Box<dyn Error>> {
    let mut buff = std::io::Cursor::new(Vec::with_capacity(0x4000));
    image.write_to(&mut buff, image::ImageFormat::Png)?;
    image_list.insert(name, buff.into_inner());
    Ok(())
}

pub async fn read_images_from_stdin() -> Result<ImageList, Box<dyn Error>> {
    let mut stdin = io::stdin();

    let mut size_buf = [0u8; 4];
    stdin.read_exact(&mut size_buf).await?;
    let size: usize = u32::from_ne_bytes(size_buf) as usize;

    let mut buffer = vec![0u8; size];
    stdin.read_exact(&mut buffer).await?;

    let ar_chip3 = image::load_from_memory_with_format(&buffer, image::ImageFormat::Png)?;
    let mut image_list: ImageList = HashMap::new();

    for image_offset in IMAGES_OFFSET.iter() {
        let mut subimage = ar_chip3.crop_imm(
            image_offset.x,
            image_offset.y,
            image_offset.w,
            image_offset.h,
        );
        if image_offset.rotate {
            subimage = subimage.rotate270();
        }
        add_to_imagelist(
            &mut image_list,
            format!("{}.png", image_offset.name),
            subimage,
        )?;
    }

    for pfp_id in 0..26 {
        let subimage = ar_chip3.crop_imm(1 + 154 * pfp_id, 1895, 152, 152);
        add_to_imagelist(&mut image_list, format!("pfp{:02}.png", pfp_id), subimage)?;
    }
    for pfp_id in 26..34 {
        let subimage = ar_chip3.crop_imm(1 + 154 * (pfp_id - 26), 1741, 152, 152);
        add_to_imagelist(&mut image_list, format!("pfp{:02}.png", pfp_id), subimage)?;
    }

    Ok(image_list)
}
