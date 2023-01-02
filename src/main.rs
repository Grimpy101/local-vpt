mod camera;
mod pipeline;
mod math;
mod mcm_renderer;

use std::{fs, io::Error, time::Instant, env};

use toml;
use serde::Deserialize;

struct Arguments {
    volume: String,
    volume_dimensions: Option<[u32; 3]>,
    transfer_function: Option<String>,
    camera_position: [f32; 3],
    mvp_matrix: Option<[f32; 16]>,
    output_resolution: [u32; 2],
    output: String,
    steps: u32,
    anisotropy: f32,
    extinction: f32,
    bounces: u32,
    linear: bool,
    iterations: u32,
    focal_length: f32,
    tones: [f32; 3],
    saturation: f32,
    gamma: f32
}

#[derive(Deserialize)]
struct ConfigFileFormat {
    output: Option<String>,
    out_resolution: Option<Vec<u32>>,
    data: Option<ConfigFileData>,
    rendering: Option<ConfigFileRendering>,
    tone_mapping: Option<ConfigFileToneMapping>
}

#[derive(Deserialize)]
struct ConfigFileData {
    volume: Option<String>,
    volume_dimensions: Option<Vec<u32>>,
    transfer_function: Option<String>
}

#[derive(Deserialize)]
struct ConfigFileRendering {
    camera_position: Option<Vec<f32>>,
    mvp_matrix: Option<Vec<f32>>,
    steps: Option<u32>,
    anisotropy: Option<f32>,
    extinction: Option<f32>,
    bounces: Option<u32>,
    linear: Option<bool>,
    iterations: Option<u32>,
    focal_length: Option<f32>
}

#[derive(Deserialize)]
struct ConfigFileToneMapping {
    tones: Option<Vec<f32>>,
    saturation: Option<f32>,
    gamma: Option<f32>
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
    let mut mvp_matrix = None;
    let mut output_resolution = [512, 512];
    let mut output = "output.ppm".to_string();
    let mut steps = 100;
    let mut anisotropy = 0.0;
    let mut extinction = 100.0;
    let mut bounces = 8;
    let mut linear = false;
    let mut iterations = 1;
    let mut focal_length = 2.0;
    let mut tones = [0.0, 0.5, 1.0];
    let mut saturation = 1.0;
    let mut gamma = 2.2;

    for i in 0..args.len() {
        if args[i] == "--config" {
            match fs::read_to_string(&args[i+1]) {
                Ok(s) => {
                    match toml::from_str::<ConfigFileFormat>(&s) {
                        Ok(config) => {
                            if let Some(x) = config.output {
                                output = x;
                            }
                            if let Some(x) = config.out_resolution {
                                output_resolution = [x[0], x[1]];
                            }
                            if let Some(x) = config.data {
                                if let Some(y) = x.volume {
                                    volume = y;
                                }
                                if let Some(y) = x.volume_dimensions {
                                    volume_dimensions = Some([y[0], y[1], y[2]])
                                }
                                transfer_function = x.transfer_function;
                            }
                            if let Some(x) = config.rendering {
                                if let Some(y) = x.anisotropy {
                                    anisotropy = y;
                                }
                                if let Some(y) = x.bounces {
                                    bounces = y;
                                }
                                if let Some(y) = x.camera_position {
                                    camera_position = [y[0], y[1], y[2]];
                                }
                                if let Some(y) = x.extinction {
                                    extinction = y;
                                }
                                if let Some(y) = x.focal_length {
                                    focal_length = y;
                                }
                                if let Some(y) = x.iterations {
                                    iterations = y;
                                }
                                if let Some(y) = x.linear {
                                    linear = y;
                                }
                                if let Some(y) = x.mvp_matrix {
                                    mvp_matrix = Some([y[0],y[1],y[2],y[3],y[4],y[5],y[6],y[7],y[8],y[9],y[10],y[11],y[12],y[13],y[14],y[15]]);
                                }
                                if let Some(y) = x.steps {
                                    steps = y;
                                }
                            }
                            if let Some(x) = config.tone_mapping {
                                if let Some(y) = x.gamma {
                                    gamma = y;
                                }
                                if let Some(y) = x.saturation {
                                    saturation = y;
                                }
                                if let Some(y) = x.tones {
                                    tones = [y[0], y[1], y[2]];
                                }
                            }
                        },
                        Err(s) => {
                            println!("Failed to parse config file\n  - Error: {}", s.to_string());
                        }
                    };
                },
                _ => {
                    eprintln!("Failed to read config file");
                }
            }
        }
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
            output_resolution[0] = args[i+1].parse::<u32>().unwrap();
            output_resolution[1] = args[i+2].parse::<u32>().unwrap();
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
        else if args[i] == "--iterations" {
            iterations = args[i+1].parse::<u32>().unwrap();
        }
        else if args[i] == "--mvp-matrix" {
            mvp_matrix = Some([
                args[i+1].parse::<f32>().unwrap(),
                args[i+2].parse::<f32>().unwrap(),
                args[i+3].parse::<f32>().unwrap(),
                args[i+4].parse::<f32>().unwrap(),
                args[i+5].parse::<f32>().unwrap(),
                args[i+6].parse::<f32>().unwrap(),
                args[i+7].parse::<f32>().unwrap(),
                args[i+8].parse::<f32>().unwrap(),
                args[i+9].parse::<f32>().unwrap(),
                args[i+10].parse::<f32>().unwrap(),
                args[i+11].parse::<f32>().unwrap(),
                args[i+12].parse::<f32>().unwrap(),
                args[i+13].parse::<f32>().unwrap(),
                args[i+14].parse::<f32>().unwrap(),
                args[i+15].parse::<f32>().unwrap(),
                args[i+16].parse::<f32>().unwrap()
            ]);
        }
        else if args[i] == "--focal-length" {
            focal_length = args[i+1].parse::<f32>().unwrap();
        }
        else if args[i] == "--levels" {
            tones[0] = args[i+1].parse::<f32>().unwrap();
            tones[1] = args[i+2].parse::<f32>().unwrap();
            tones[2] = args[i+3].parse::<f32>().unwrap();
        }
        else if args[i] == "--saturation" {
            saturation = args[i+1].parse::<f32>().unwrap();
        }
        else if args[i] == "--gamma" {
            gamma = args[i+1].parse::<f32>().unwrap();
        }
        else if args[i] == "--help" {
            let text = format!(
                "** {} (version {}) **\nAuthors: {}\n\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
                "VPT Lazy Ripoff",
                "0.1.0",
                "Gorazd Gorup, Žiga Lesar (original)",
                "--volume : Path to file with raw volumetric data",
                "--volume-dimensions : Three integers representing width, height and depth of texture (optional)",
                "--tf : Path to the file with transfer function texture (optional)",
                "--camera-position : Three floats representing x,y,z coordinates of camera (optional)",
                "--mvp-matrix : 16 floats representing inversed transformation matrix (optional)",
                "--out-resolution : An integer representing resolution of output image (optional)",
                "--output : Path to output image file (optional)",
                "--steps : Number of rendering steps (optional)",
                "--anisotropy : Anisotropy (optional)",
                "--extinction : Extinction (optional)",
                "--bounces : Number of bounces per photon (optional)",
                "--iterations : Number of iterations (optional)",
                "--focal-length : A float representing distance of projection plane from camera origin (optional)",
                "--tones : Three floats representing low, mid and high tones (optional)",
                "--saturation : Saturation on post-processing (optional)",
                "--gamma : Gamma value on post-processing (optional)"
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
        mvp_matrix,
        output_resolution,
        output,
        steps,
        anisotropy,
        extinction,
        bounces,
        linear,
        iterations,
        focal_length,
        tones,
        saturation,
        gamma
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
    let mvp_matrix = args.mvp_matrix;
    let linear_filter = args.linear;
    let iterations = args.iterations;
    let focal_length = args.focal_length;
    let tones = args.tones;
    let saturation = args.saturation;
    let gamma = args.gamma;

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

    let image_size = out_res[0] * out_res[1] * 3;
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
                linear: linear_filter,
                iterations,
                mvp_matrix,
                focal_length,
                tones,
                saturation,
                gamma
            },
            &mut image
        )
    );

    match write_output(&output_file, out_res[0], out_res[1], image) {
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
