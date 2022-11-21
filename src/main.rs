mod camera;
mod pipeline;
mod math;
mod mcm_renderer;

use std::{fs, io::Error, time::Instant, env};

struct Arguments {
    volume: String,
    volume_dimensions: Option<[u32; 3]>,
    transfer_function: Option<String>,
    camera_position: [f32; 3],
    output_resolution: u32,
    output: String,
    steps: u32,
    anisotropy: f32,
    extinction: f32,
    bounces: u32,
    linear: bool
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

fn parse_arguments() -> Result<Arguments, String> {
    let args: Vec<String> = env::args().collect();
    let mut volume = String::new();
    let mut volume_dimensions = None;
    let mut transfer_function = None;
    let mut camera_position = [-1.0, -1.0, 1.0];
    let mut output_resolution = 512;
    let mut output = "output.ppm".to_string();
    let mut steps = 100;
    let mut anisotropy = 0.0;
    let mut extinction = 100.0;
    let mut bounces = 8;
    let mut linear = false;

    for i in 0..args.len() {
        if args[i] == "--volume" {
            volume = args[i+1].to_string();
        }

        if args[i] == "--volume-dimensions" {
            volume_dimensions = Some([
                args[i+1].parse::<u32>().unwrap(),
                args[i+2].parse::<u32>().unwrap(),
                args[i+3].parse::<u32>().unwrap()
            ]);
        }
        else if args[i] == "--tf" {
            transfer_function = Some(args[i+1].to_string());
        }
        else if args[i] == "--camera-position" {
            camera_position = [
                args[i+1].parse::<f32>().unwrap(),
                args[i+2].parse::<f32>().unwrap(),
                args[i+3].parse::<f32>().unwrap()
            ];
        }
        else if args[i] == "--out-resolution" {
            output_resolution = args[i+1].parse::<u32>().unwrap();
        }
        else if args[i] == "--output" {
            output = args[i+1].to_string();
        }
        else if args[i] == "--steps" {
            steps = args[i+1].parse::<u32>().unwrap();
        }
        else if args[i] == "--anisotropy" {
            anisotropy = args[i+1].parse::<f32>().unwrap();
        }
        else if args[i] == "--extinction" {
            extinction = args[i+1].parse::<f32>().unwrap();
        }
        else if args[i] == "--bounces" {
            bounces = args[i+1].parse::<u32>().unwrap();
        }
        else if args[i] == "--linear" {
            linear = true;
        }
        else if args[i] == "--help" {
            let text = format!(
                "** {} (version {}) **\nAuthors: {}\n\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
                "VPT Lazy Ripoff",
                "0.1.0",
                "Gorazd Gorup, Žiga Lesar (original)",
                "--volume : Path to file with raw volumetric data",
                "--volume-dimensions : Three integers representing width, height and depth of texture (optional)",
                "--tf : Path to the file with transfer function texture (optional)",
                "--camera-position : Three floats representing x,y,z coordinates of camera (optional)",
                "--out-resolution : An integer representing resolution of output image (optional)",
                "--output : Path to output image file (optional)",
                "--steps : Number of rendering steps (optional)",
                "--anisotropy : Anisotropy (optional)",
                "--extinction : Extinction (optional)",
                "--bounces : Number of bounces per photon (optional)"
            );
            return Err(text);
        }
    }

    if volume.is_empty() {
        return Err("Error: No volume provided!".to_string());
    }

    return Ok(Arguments {
        volume,
        volume_dimensions,
        transfer_function,
        camera_position,
        output_resolution,
        output,
        steps,
        anisotropy,
        extinction,
        bounces,
        linear
    });
}

fn main() {
    let args = match parse_arguments() {
        Ok(a) => {
            a
        },
        Err(e) => {
            println!("{}", e);
            return;
        }
    };

    let output_file = args.output;
    let volume_file = args.volume;
    let transfer_function_file = args.transfer_function;
    let steps = args.steps;
    let out_res = args.output_resolution;
    let anisotropy = args.anisotropy;
    let extinction = args.extinction;
    let bounces = args.bounces;
    let camera_position = args.camera_position;
    let linear_filter = args.linear;

    println!("Starting...");
    let timer = Instant::now();

    let volume = match read_u8_file(&volume_file) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error: Coult not open volume {:?}: {}", volume_file, e);
            return;
        }
    };
    let volume_dims =  match args.volume_dimensions {
        Some(c) => {
            [c[0], c[1], c[2]]
        },
        None => {
            let vol_size = volume.len() as f32;
            let candidate = vol_size.cbrt().floor();
            let x = candidate as u32;
            let y = candidate as u32;
            let z = (vol_size / (candidate * candidate)) as u32;
            println!("WARNING: No dimensions provided. Using [{},{},{}] as calculated dimensions.", x, y, z);
            [x, y, z]
        },
    };

    let transfer_function = match transfer_function_file {
        Some(tf_file) => {
            match read_u8_file(&tf_file) {
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
                camera_position,
                linear: linear_filter
            },
            &mut image
        )
    );

    match write_output(&output_file, out_res, out_res, image) {
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
