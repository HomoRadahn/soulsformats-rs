use soulsformats_rs::{DcxIO, binder::Bnd4};
use std::{env, error::Error, fs, path::Path, process};

fn decompress_write(path: String) -> Result<(), Box<dyn Error>> {
    let (bnd, _) = Bnd4::decompress_file(&path)?;
    let input_path = Path::new(&path);
    let output_dir = input_path.with_extension("");
    fs::create_dir_all(&output_dir)?;

    for file in bnd.files {
        if let Some(name) = &file.name {
            let name_path = Path::new(name);
            let display_name: std::path::PathBuf = name_path.components().skip(4).collect();
            let output_path = output_dir.join(display_name);
            if let Some(parent) = output_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(output_path, file.bytes)?;
        } else {
            eprintln!("Nameless file skipped");
        }
    }

    Ok(())
}

fn main() {
    let path = match env::args().nth(1) {
        Some(arg) => arg,
        None => {
            println!("No path was given");
            process::exit(0);
        }
    };

    if let Err(e) = decompress_write(path) {
        eprintln!("Program exited with error: {e}");
        process::exit(1);
    }
}
