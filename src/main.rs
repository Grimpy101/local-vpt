mod camera;
mod pipeline;
mod math;
mod mcm_renderer;

use std::{path::PathBuf, fs, io::Error, time::Instant};

use clap::{Parser, command};


#[derive(Parser, Debug)]
#[command(name = "vpt-lazy-ripoff")]
#[command(author = "Gorazd Gorup, Žiga Lesar")]
#[command(version = "1.0")]
#[command(about = "Generates a volume visualization based on given data", long_about = None)]
struct Arguments {
    volume: PathBuf,
    #[arg(short, long)]
    dimensions_volume: Vec<u32>,
    #[arg(short, long)]
    transfer_function: Option<PathBuf>,
    #[arg(short, long)]
    camera_position: Option<Vec<f32>>,
    #[arg(short, long, default_value_t = 512)]
    resolution: u32,
    #[arg(short, long, default_value = "output.ppt")]
    output: PathBuf,
    #[arg(short, long, default_value_t = 50)]
    steps: u32,
    #[arg(short, long, default_value_t = 0.0)]
    anisotropy: f32,
    #[arg(short, long, default_value_t = 100.0)]
    extinction: f32,
    #[arg(short, long, default_value_t = 8)]
    bounces: u32
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
    let args = Arguments::parse();

    let output_file = args.output;
    let volume_file = args.volume;
    let transfer_function_file: Option<PathBuf> = args.transfer_function;
    let steps = args.steps;
    let out_res = args.resolution;
    let volume_dims = (
        args.dimensions_volume[0],
        args.dimensions_volume[1],
        args.dimensions_volume[2]
    );
    let anisotropy = args.anisotropy;
    let extinction = args.extinction;
    let bounces = args.bounces;
    let camera_position = match args.camera_position {
        Some(c) => {
            (c[0], c[1], c[2])
        },
        None => {
            (0.0, 0.0, 0.0)
        },
    };

    println!("Starting...");
    let timer = Instant::now();

    let volume = match read_u8_file(volume_file.to_str().unwrap()) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error: Coult not open volume {:?}: {}", volume_file, e);
            return;
        }
    };

    let transfer_function = match transfer_function_file {
        Some(tf_file) => {
            match read_u8_file(tf_file.to_str().unwrap()) {
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
                extinction,
                anisotropy,
                max_bounces: bounces,
                steps,
                camera_position
            },
            &mut image
        )
    );

    match write_output(output_file.to_str().unwrap(), out_res, out_res, image) {
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
