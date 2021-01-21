use std::fs;
use std::fs::DirEntry;
use std::{env, path::PathBuf};

use image::{imageops, GenericImage, GenericImageView, ImageBuffer, RgbImage};
use imageops::FilterType;
use FilterType::Nearest;
/// * get all files -check
/// * find out how many columns
///     * start with "half the images"
///     * try column size, fill out columns (always go with lowest)
///     * if all columns are full, go op by 50% (binary search)
///     * if columns are empty, go down by 50%
/// * when found the right size, return the info about grid
/// * blend each image into the new image
/// * save the output image to file

fn main() -> Result<(), Box<std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        //0 is program itself
        println!("Expecting 1 argument, an input dir with images");
        return Ok(());
    }

    let (width, height) = (3840, 2160);
    //0 is program itself
    let files_path = &args[1];
    let number_of_columns = args[2]
        .parse::<i32>()
        .expect("column number must be an int")
        .clone();

    let images = get_images_in_folder(files_path);

    let mut target_img = ImageBuffer::new(width, height);

    let column_width = (width as f32 / number_of_columns as f32).ceil() as u32;

    let mut columns = vec![];
    for _ in 0..number_of_columns {
        columns.push(ImageColumn::default());
    }

    let mut grid = ImageGrid { columns };

    for image in images {
        let (org_width, org_height) = image::io::Reader::open(image.path())?.into_dimensions()?;
        let ratio = org_height as f32 / org_width as f32;
        let new_height = (column_width as f32 * ratio) as u32;
        grid.add_to_lowest_column(new_height, image.path());
    }
    let mut x = 0;

    for colum in grid.columns {
        let mut offset = 0;
        for (image_height, image_path) in colum.image_paths {
            let img = image::io::Reader::open(image_path)?.decode()?;
            let resized_image =
                image::imageops::resize(&img, column_width, image_height, FilterType::Gaussian);

            image::imageops::overlay(&mut target_img, &resized_image, x, offset);
            offset = offset + image_height;
        }
        x = x + column_width;
    }

    target_img.save("output.png")?;

    Ok(())
}

#[derive(Default, Debug)]
pub struct ImageGrid {
    columns: Vec<ImageColumn>,
}

impl ImageGrid {
    pub fn add_to_lowest_column(&mut self, image_height: u32, image_path: PathBuf) {
        self.columns.sort_by_key(|f| f.column_height);
        self.columns[0].image_paths.push((image_height, image_path));
        self.columns[0].column_height += image_height;
    }
}

#[derive(Default, Debug)]
pub struct ImageColumn {
    image_paths: Vec<(u32, PathBuf)>,
    column_height: u32,
}

fn get_images_in_folder(path: &String) -> Vec<DirEntry> {
    match fs::read_dir(path) {
        Ok(entries) => entries.filter_map(|f| match f {
            Ok(entry) => match entry.path().extension() {
                Some(extension)
                    if extension == "png" || extension == "jpg" || extension == "jpeg" =>
                {
                    Some(entry)
                }
                _ => None,
            },
            Err(_) => None,
        }),
        Err(message) => {
            panic!(
                "Did not find any images in folder, got error : {} ",
                message
            );
        }
    }
    .collect()
}
