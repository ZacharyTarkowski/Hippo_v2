use std::fs::{File};
use std::env::args;
use std::io::prelude::*;
use std::io::BufReader;
    
use std::io::Cursor;
use embedded_graphics::prelude::IntoStorage;
use image::{ColorType, EncodableLayout, ImageFormat, ImageReader, Pixel};
use toml::Table;

use serde_derive::Deserialize;
use std::fs;
use std::process::exit;
use toml;

use toml::de::from_str;
use serde::Deserialize;

use embedded_graphics::pixelcolor::Rgb565;
use rgb565::*;

#[derive(Deserialize)]
struct Options {
    dummy: i32,
}

#[derive(Deserialize)]
struct Images {
    image_list: Vec<String>,
}

#[derive(Deserialize)]
struct Config {
    options : Options,
    images: Images,
}

static default_config_path: &str = "rle_config.toml";

fn main() {
    println!("hello world");
    //need one of those monitor rules to recompile if the images change

    let args: Vec<String> = args().collect();

    let config_path = if args.len() > 1 {
        String::from(&args[1])
    } else {
        default_config_path.to_owned()
    };

    let mut image_name_file = File::create("src/image_names.rs").unwrap();
    image_name_file.write("pub mod image_names {\n".as_bytes()).unwrap();


    println!("Using Config : {config_path}");

    let contents = fs::read_to_string(default_config_path).unwrap();
    
    let config: Config = from_str(&contents).expect("Failed to parse TOML");

    // Now you can use the vector of strings
    let mut count  = 0;
    for item in config.images.image_list {
        println!("{}", item);
        let mut img: image::DynamicImage = ImageReader::open(&item).unwrap().decode().unwrap();

        img = img.resize_exact(240, 320, image::imageops::FilterType::Nearest);

        let converted_image = img.to_rgb8();

        let data = converted_image.as_raw();
        let data = data.as_bytes();

        let mut packed_data: Vec<u16> = Vec::with_capacity(320*240);
        for chunk in data.chunks(3) {
            let newr = ((chunk[0] as u16 *32)/ 256) as u8; 
            let newg = ((chunk[1] as u16 *32)/ 256) as u8; 
            let newb = ((chunk[2] as u16 *32)/ 256) as u8; 
            let tmp_color = Rgb565::new(newr,newg,newb);
            packed_data.push(tmp_color.into_storage());
        }

        let mut rle_data : Vec<u16> = Vec::with_capacity(320*240);
        let mut last_pixel = packed_data[0];
        let mut run_count = 0;

        for pixel in packed_data {
            if pixel == last_pixel {
                run_count += 1;
            } else {
                rle_data.push(last_pixel);
                rle_data.push(run_count);

                last_pixel = pixel;
                run_count = 1;
            }
        }

        let formatted_output_name = format!("{}.bin", item);
        let mut file = File::create(&formatted_output_name).unwrap();
        file.write_all(&rle_data.as_bytes()).unwrap();
        count += 1;

        
        
        let formatted_var_name = format!("pub static image_{} : &'static str = \"{}\";\n", count, formatted_output_name);
        image_name_file.write(formatted_var_name.as_bytes()).unwrap();
        //file.write(format!("image_{} = {}", count, format!("{}.bin", item) )).unwrap();
    }
    image_name_file.write("}\n".as_bytes()).unwrap();

}
