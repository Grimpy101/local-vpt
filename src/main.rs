mod camera;
mod pipeline;
mod math;
mod mcm_renderer;

use std::{path::PathBuf, fs, io::Error, time::Instant};

use clap::{Parser, command};


#[derive(Parser, Debug)]
#[command(name = "Local VPT")]
#[command(author, version, about, long_about = None)]
struct Arguments {
    volume: PathBuf,
    #[arg(short, long)]
    transfer_function: Option<PathBuf>,
    #[arg(short, long)]
    camera_position: Option<Vec<i32>>,
    #[arg(short, long, default_value = "output.ppt")]
    output: PathBuf,
    #[arg(short, long, default_value_t = 50)]
    steps: i32
}

fn read_u8_file(filename: &str) -> Result<Vec<u8>, Error> {
    let contents = fs::read(filename)?;
    return Ok(contents);
}

fn write_output(filename: &str, width: u32, height: u32, content: Vec<u8>) -> Result<(), Error> {
    let mut output = format!("P3\n{} {}\n{}\n", width, height, 255);
    for i in (0..content.len()).step_by(3) {
        let r = content[i];
        let g = content[i+1];
        let b = content[i+2];
        output.push_str(&format!("{} {} {}\n", r, g, b));
    }

    return fs::write(filename, output);
}

fn main() {
    let output_file = "output.ppm";
    let volume_file = "test_volume.raw";
    let transfer_function_file: Option<&str> = None;
    let steps = 1000;

    let out_res = 1024;

    println!("Starting...");
    let timer = Instant::now();

    let volume = match read_u8_file(volume_file) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error: Coult not open volume {:?}: {}", volume_file, e);
            return;
        }
    };

    let transfer_function = match transfer_function_file {
        Some(tf_file) => {
            match read_u8_file(tf_file) {
                Ok(tf) => tf,
                Err(e) => {
                    eprintln!("Error: Could not open transfer function {:?}: {}", tf_file, e);
                    return;
                }
            }
        },
        None => {
            vec![0, 0, 0, 0, 255, 0, 0, 255]
        }
    };

    let volume_dims = (256, 256, 113);
    let tf_len = transfer_function.len() / 4;

    let image_size = out_res * out_res * 3;
    let mut image: Vec<u8> = Vec::with_capacity(image_size as usize);

    pollster::block_on(
        pipeline::render(
            pipeline::RenderData {
                output_resolution: out_res,
                volume,
                volume_dims,
                transfer_function,
                transfer_function_len: tf_len as u32,
                extinction: 100.0,
                anisotropy: 0.0,
                max_bounces: 8,
                steps
                
            },
            &mut image
        )
    );

    match write_output(output_file, out_res, out_res, image) {
        Ok(()) => {
            println!("Image written!")
        },
        Err(e) => {
            eprintln!("Error: Could not write image to file {:?}: {}", output_file, e);
            return;
        }
    }

    println!("Time: {}", timer.elapsed().as_secs_f32());
}
