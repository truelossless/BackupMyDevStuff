use rayon::prelude::*;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::Path;
use std::{
    error::Error,
    fs::{self},
};
use std::{sync::Arc, sync::Mutex};
use zip::ZipWriter;
use zip::{write::FileOptions, CompressionMethod};

fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: backupmydevstuff [DIR_PATH] [ARCHIVE_PATH]");
        std::process::exit(1);
    }

    let dir_path = Path::new(&args[1]);
    if dir_path.is_file() {
        eprintln!("[DIR_PATH] should be a directory");
        std::process::exit(1);
    }
    let dir_path = dir_path.canonicalize().unwrap();

    let archive_path = Path::new(&args[2]);
    if archive_path.is_dir() {
        eprintln!("[ARCHIVE_PATH] should be a file");
        std::process::exit(1);
    }

    // the archive may include itself so build it in the parent folder
    let tmp_archive_path = dir_path.parent().unwrap().join("backupmydevstuff_tmp.zip");

    let archive_file = match File::create(&tmp_archive_path) {
        Ok(archive_file) => archive_file,
        Err(e) => {
            eprintln!("Could not create archive file:\n{}", e);
            std::process::exit(1);
        }
    };

    let writer = Arc::new(Mutex::new(ZipWriter::new(archive_file)));
    if let Err(e) = smart_zip(&dir_path, writer.clone()) {
        eprintln!("Could not zip a directory:\n{}", e);
        std::process::exit(1);
    }

    if let Err(e) = writer.lock().unwrap().finish() {
        eprintln!("Could not finalize the archive:\n{}", e);
        std::process::exit(1);
    }

    println!("backupmydevstuff successfully finished !");

    // move back the archive to its desired location
    if let Err(e) = fs::rename(&tmp_archive_path, &archive_path) {
        eprintln!("Cannot move the resulting archive to its destination\nYou can still find it there: {}\n{}", tmp_archive_path.to_string_lossy(), e);
    }
}

/// add to a zip by excluding build artifacts and respecting .gitignore files
fn smart_zip(
    path: &Path,
    writer: Arc<Mutex<ZipWriter<File>>>,
) -> Result<(), Box<dyn Error + Sync + Send>> {
    let options = FileOptions::default().compression_method(CompressionMethod::Bzip2);

    println!("Entering {} ...", path.to_string_lossy());

    let mut excludes = Vec::new();

    // get the path iterator
    // CAVEAT: this doesn't take into account upper gitignores
    let (mut path_iter, is_recursive) = match gitignore::File::new(&path.canonicalize().unwrap().join(".gitignore"))
    {
        // follow the gitignore file first, if it exists
                Ok(gitignore) => {
            println!(".gitignore file detected");
            (gitignore.included_files()?, true)
        }
        // else try to infer ourselves the rules
        Err(_) => {
            // Rust project
            if path.join("cargo.toml").is_file() {
                // println!("Rust project detected, excluding target/");
                excludes.push(path.join("target"));

            // Node project
            } else if path.join("node_modules").is_dir() {
                // println!("NodeJS project detected, excluding node_modules/");
                excludes.push(path.join("node_modules"));
            }

            let iter = fs::read_dir(path)?
                .map(|res| res.map(|e| e.path()))
                .filter(|path| {
                    !excludes
                        .iter()
                        .any(|exclude| path.as_ref().unwrap() == exclude)
                })
                .collect::<Result<Vec<_>, io::Error>>()?;
            (iter, false)
    }
};

    // recursively explore other folders
    path_iter
        .par_iter_mut()
        .map(|subdir_path| {
            // add the file to the zip
            if subdir_path.is_file() {
                // read the file to a buffer
                let mut buf = Vec::new();
                let mut file = File::open(&subdir_path)?;
                file.read_to_end(&mut buf)?;

                let mut writer_lock = writer.lock().unwrap();
                #[allow(deprecated)] // start_file doesn't work too for me
                writer_lock.start_file_from_path(&subdir_path, options)?;
                writer_lock.write_all(&buf)?;

            // if our iter doesn't take into account subfolder do it ourselves
            } else if !is_recursive {
                smart_zip(&subdir_path, writer.clone())?;
            }

            Ok(())
        })
        .collect::<Result<Vec<()>, Box<dyn Error + Sync + Send>>>()?;

    Ok(())
}
