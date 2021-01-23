use std::fs::DirEntry;
use std::{env, path::PathBuf};
use std::{fs, vec};

use image::{imageops, ImageBuffer, Rgba};
use imageops::FilterType;
use rayon::prelude::*;
/// * get all files -check
/// * find out how many columns
///     * start with "half the images"
///     * try column size, fill out columns (always go with lowest)
///     * if all columns are full, go op by 50% (binary search)
///     * if columns are empty, go down by 50%
/// * when found the right size, return the info about grid
/// * blend each image into the new image
/// * save the output image to file

fn main() -> Result<(), Box<dyn std::error::Error>> {
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
    let image_infos = grid.get_image_info(column_width);
    for image_info in image_infos {
        image::imageops::overlay(
            &mut target_img,
            &image_info.resized_image,
            image_info.x,
            image_info.offset,
        );
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

    pub fn get_image_info(&self, column_width: u32) -> Vec<ImageInfo> {
        self.columns
            .par_iter()
            .enumerate()
            .flat_map(|(column_number, column)| {
                let x = column_number as u32 * column_width as u32;
                let mut column_images: Vec<ImageInfo> = column
                    .image_paths
                    .par_iter()
                    .map(move |(height, path)| {
                        let img = image::io::Reader::open(path).unwrap().decode().unwrap();
                        let resized_image = image::imageops::resize(
                            &img,
                            column_width,
                            *height,
                            FilterType::Gaussian,
                        );
                        ImageInfo {
                            x,
                            offset: 0, //TODO
                            image_height: *height,
                            resized_image,
                        }
                    })
                    .collect();
                let mut offset = 0;
                for image in column_images.iter_mut() {
                    image.offset = offset;
                    offset = offset + image.image_height;
                }
                column_images
            })
            .collect()
    }
}

pub struct ImageInfo {
    x: u32,
    offset: u32,
    image_height: u32,
    resized_image: ImageBuffer<Rgba<u8>, Vec<u8>>,
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
