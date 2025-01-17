extern crate rand;

use std::fs::DirEntry;
use std::path::PathBuf;
use std::time::SystemTime;
use std::{fs, vec};

use image::{imageops, ImageBuffer, Rgba};
use imageops::FilterType;
use rand::{prelude::SliceRandom, thread_rng};
use rayon::prelude::*;

use clap::Clap;

#[derive(Clap)]
#[clap(
    version = "1.0",
    author = "Philip Kristoffersen <philipkristoffersen@gmail.com>"
)]
struct Opts {
    #[clap(
        short,
        long,
        default_value = "output.png",
        about = "The output file to write to mosaic to"
    )]
    output_file: String,

    #[clap(
        short,
        long,
        default_value = ".",
        about = "The folder that contains the images to put in the mosaic"
    )]
    images_folder: String,

    #[clap(long, default_value = "1920")]
    image_width: u32,

    #[clap(long, default_value = "1080")]
    image_height: u32,

    #[clap(short, long, about = "Randomize the order of images in the columns")]
    randomize: Option<bool>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let now = SystemTime::now();
    let opts: Opts = Opts::parse();

    let (width, height) = (opts.image_width, opts.image_height);

    println!("Loading images {}", now.elapsed().unwrap().as_millis());

    let images = get_images_in_folder(&opts.images_folder);
    if images.is_empty() {
        println!("Did not find any images in folder {}", opts.images_folder);
        return Ok(());
    }
    if images.len() == 1 {
        println!("Only found one image in folder {}", opts.images_folder);
        return Ok(());
    }
    println!("Finding image sizes {}", now.elapsed().unwrap().as_millis());
    let mut org_image_info: Vec<OrgImageInfo> = images
        .par_iter()
        .map(|image| {
            let (org_width, org_height) = image::io::Reader::open(image.path())
                .unwrap()
                .into_dimensions()
                .unwrap();
            OrgImageInfo {
                width: org_width,
                height: org_height,
                path: image.path(),
            }
        })
        .collect();
    org_image_info.sort_by_key(|f| ((f.height as f32 / f.width as f32) * 10000.) as u32);
    if let Some(true) = opts.randomize.or(Some(true)) {
        println!("Randomizing images {}", now.elapsed().unwrap().as_millis());
        let mut rng = thread_rng();
        org_image_info.shuffle(&mut rng);
    }

    println!(
        "Finding optimal grid size {}",
        now.elapsed().unwrap().as_millis()
    );

    let before = SystemTime::now();
    let try_range = 2..images.len() / 2;
    println!(
        "Trying {} column configurations, from {:?} to {:?}",
        try_range.len(),
        2,
        images.len() / 2
    );

    let all_grids: Vec<ImageGrid> = try_range
        .into_par_iter()
        .map(|number_of_columns| {
            create_image_grid(number_of_columns as u32, width, &org_image_info)
        })
        .collect();

    let gridh = all_grids
        .iter()
        .filter(|c| (c.has_overfull_columns(height)))
        .count();
    let grid = all_grids.get(gridh).unwrap();

    println!(
        "Found optimal in {}nano",
        before.elapsed().unwrap().as_nanos()
    );
    let wasted_pixels = grid.get_wasted_pixels(height);
    println!(
        "Best with  {} columns: Wasted pixels: {} - {}",
        grid.columns.len(),
        wasted_pixels,
        now.elapsed().unwrap().as_millis()
    );

    let mut target_img = ImageBuffer::new(width, height);
    println!(
        "Scaling  {} images  - {}",
        images.len(),
        now.elapsed().unwrap().as_millis()
    );
    let image_infos = grid.get_image_info(height);
    println!(
        "Merging into one image - {}",
        now.elapsed().unwrap().as_millis()
    );
    for image_info in image_infos {
        image::imageops::overlay(
            &mut target_img,
            &image_info.resized_image,
            image_info.x,
            image_info.offset,
        );
    }
    println!("Saving image {}", now.elapsed().unwrap().as_millis());
    target_img.save(opts.output_file)?;
    println!("Done {}", now.elapsed().unwrap().as_millis());
    Ok(())
}

pub fn create_image_grid(
    number_of_columns: u32,
    target_width: u32,
    org_image_info: &[OrgImageInfo],
) -> ImageGrid {
    let column_width = (target_width as f32 / number_of_columns as f32).ceil() as u32;

    let mut columns = vec![];
    for _ in 0..number_of_columns {
        columns.push(ImageColumn::default());
    }

    let mut grid = ImageGrid {
        column_width,
        columns,
        number_of_columns,
    };
    for image in org_image_info {
        let ratio = image.height as f32 / image.width as f32;
        let new_height = (column_width as f32 * ratio) as u32;
        grid.add_to_lowest_column(new_height, &image.path);
    }
    grid
}

#[derive(Default, Debug)]
pub struct ImageGrid<'a> {
    column_width: u32,
    columns: Vec<ImageColumn<'a>>,
    number_of_columns: u32,
}

impl<'a> ImageGrid<'a> {
    pub fn get_wasted_pixels(&self, target_height: u32) -> u32 {
        self.columns
            .par_iter()
            .map(|column| {
                let column_pixels = (column.column_height * self.column_width) as i32;
                let image_pixels = (self.column_width * target_height) as i32;
                (column_pixels - image_pixels).abs() as u32
            })
            .sum()
    }

    pub fn has_empty_space(&self, target_height: u32) -> bool {
        self.columns
            .par_iter()
            .any(|c| c.column_height < target_height)
    }

    pub fn has_overfull_columns(&self, target_height: u32) -> bool {
        self.columns
            .par_iter()
            .any(|c| c.column_height > target_height)
    }

    pub fn add_to_lowest_column(&mut self, image_height: u32, image_path: &'a PathBuf) {
        self.columns.par_sort_by_key(|f| f.column_height);
        self.columns[0].image_paths.push((image_height, image_path));
        self.columns[0].column_height += image_height;
    }

    pub fn get_image_info(&self, target_height: u32) -> Vec<FinalImageInfo> {
        let column_width = self.column_width;
        self.columns
            .par_iter()
            .enumerate()
            .flat_map(|(column_number, column)| {
                let x = column_number as u32 * column_width as u32;
                let mut column_images: Vec<FinalImageInfo> = column
                    .image_paths
                    .par_iter()
                    .map(|(height, path)| {
                        let img = image::io::Reader::open(path).unwrap().decode().unwrap();
                        let resized_image = image::imageops::resize(
                            &img,
                            column_width,
                            *height,
                            FilterType::Gaussian,
                        );
                        FinalImageInfo {
                            x,
                            offset: 0,
                            image_height: *height,
                            resized_image,
                        }
                    })
                    .collect();
                let mut offset = 0;
                for image in column_images.iter_mut() {
                    image.offset = offset;
                    offset += image.image_height;
                }
                column_images.sort_by_key(|c| c.image_height);
                column_images.reverse();

                let mut i = 0;
                while offset < target_height {
                    let mut pad_image = column_images[i].clone();
                    pad_image.offset = offset;
                    offset += pad_image.image_height;
                    column_images.push(pad_image);
                    i += 1;
                }
                column_images
            })
            .collect()
    }
}
pub struct OrgImageInfo {
    width: u32,
    height: u32,
    path: PathBuf,
}

#[derive(Clone)]
pub struct FinalImageInfo {
    x: u32,
    offset: u32,
    image_height: u32,
    resized_image: ImageBuffer<Rgba<u8>, Vec<u8>>,
}

#[derive(Default, Debug)]
pub struct ImageColumn<'a> {
    image_paths: Vec<(u32, &'a PathBuf)>,
    column_height: u32,
}

fn get_images_in_folder(path: &str) -> Vec<DirEntry> {
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
