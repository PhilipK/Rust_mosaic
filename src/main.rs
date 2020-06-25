extern crate raster;

use raster::editor;
use std::env;
use std::fs;
use std::fs::DirEntry;

/// * get all files -check
/// * find out how many columns
///     * start with "half the images"
///     * try column size, fill out columns (always go with lowest)
///     * if all columns are full, go op by 50% (binary search)
///     * if columns are empty, go down by 50%
/// * when found the right size, return the info about grid
/// * blend each image into the new image
/// * save the output image to file
fn main() {
    let args: Vec<String> = env::args().collect();
    //0 is program itself
    if args.len() < 3 {
        println!("Expecting 2 arguments, an input dir and an output file");
        return;
    }

    //0 is program itself
    let files_path = &args[1];
    let output_file = &args[2];

    //we can unwrap here as we would have just paniced otherwise.
    let files_directory = get_images_in_folder(files_path);

    //

    let mut image = raster::open("testimage.png").unwrap();
    let target_width = (image.width as f32 / 2.0) as i32;
    let target_height = (image.height as f32 / 2.0) as i32;
    if let Err(error_message) = editor::resize(
        &mut image,
        target_width,
        target_height,
        editor::ResizeMode::Fill,
    ) {
        println!("Error resizing image: {:?}", error_message);
    };
    if let Err(message) = raster::save(&mut image, "output.png") {
        println!("Error saving image: {:?}", message);
    }
}

fn get_images_in_folder(path: &String) -> Vec<DirEntry> {
    let mut result = Vec::new();
    match fs::read_dir(path) {
        Ok(entries) => {
            for entry in entries {
                if let Ok(entry) = entry {
                    // Here, `entry` is a `DirEntry`.
                    if let Ok(file_type) = entry.file_type() {
                        // Now let's show our entry's file type!
                        if file_type.is_file() {
                            if let Some(extension) = entry.path().extension() {
                                if extension == "png" || extension == "jpg" || extension == "jpeg" {
                                    result.push(entry);
                                }
                            }
                        }
                    }
                }
            }
        }
        Err(message) => {
            panic!(
                "Did not find any images in folder, got error : {} ",
                message
            );
        }
    }
    result
}

struct ImageColumn<'s> {
    images: Vec<&'s raster::Image>,
    current_height: i32,
}

struct ImageGrid<'s> {
    columns: Vec<ImageColumn<'s>>,
}
