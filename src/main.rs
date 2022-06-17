use std::env;
use std::error::Error;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::path::PathBuf;

use chrono::{offset::Local, DateTime};
use clap::Parser;
use unix_mode;
use users;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(value_parser)]
    files: Vec<PathBuf>,
    #[clap(short, long, value_parser)]
    all: bool,
    #[clap(short, value_parser)]
    l: bool,
    #[clap(short, value_parser)]
    g: bool,
    #[clap(short = 'G', long, value_parser)]
    no_group: bool,
    #[clap(short, long, value_parser)]
    inode: bool,
    #[clap(short, long, value_parser)]
    directory: bool,
    #[clap(short, long, value_parser)]
    group_directories_first: bool,
    #[clap(short = 'U', value_parser)]
    uu: bool,
}

// struct Line {
//     mode: Option<String>,
//     nlink: Option<String>,
//     user: Option<String>,
//     group: Option<String>,
//     size: Option<String>,
//     cdate: Option<String>,
//     mtime: Option<String>,
//     name: Option<String>,
// }

// #[derive(Default)]
// struct SectionLength {
//     mode: usize,
//     nlink: usize,
//     user: usize,
//     group: usize,
//     size: usize,
//     cdate: usize,
//     mtime: usize,
//     name: usize,
// }

// enum Align {
//     LeftAlign,
//     RightAlign,
// }

fn show_files(args: &Args, files_old: &Vec<&Path>) -> Result<(), Box<dyn Error>> {
    if files_old.is_empty() {
        return Ok(());
    }

    let mut files = files_old.clone();
    // Sort the file data
    if !args.uu {
        if args.group_directories_first {
            files.sort_by(|a, b| {
                (a.is_dir()
                    .cmp(&b.is_dir())
                    .reverse()
                    .then(a.file_name().cmp(&b.file_name())))
            });
        } else {
            files.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
        }
    }

    let mut grid = Vec::new();

    let total_blocks = 0;

    // let mut align = Vec::new();

    // Print the file data
    for file in &files {
        let filename = file.file_name().unwrap();
        let meta = file.metadata()?;
        let created: DateTime<Local> = meta.created()?.into();
        let modified: DateTime<Local> = meta.modified()?.into();
        let user = users::get_user_by_uid(meta.uid()).unwrap();
        let group = users::get_group_by_gid(meta.gid()).unwrap();
        // println!("{:?} {:?}", meta.blksize(), meta.blocks());

        let mut line = Vec::new();

        if args.inode {
            line.push(meta.ino().to_string());
        }

        if args.l {
            line.push(unix_mode::to_string(meta.permissions().mode()));
            line.push(meta.nlink().to_string());
            if !args.g {
                line.push(user.name().to_str().unwrap().to_string());
            }
            if !args.no_group {
                line.push(group.name().to_str().unwrap().to_string())
            }
            line.push(meta.size().to_string());
            line.push(created.format("%b %e").to_string());
            line.push(modified.format("%R").to_string());
        }
        let mut name = filename.to_str().unwrap().to_string();
        if args.l {
            if file.is_symlink() {
                name += " -> ";
                name += fs::read_link(file)?.to_str().unwrap();
            }
        }
        line.push(name);

        grid.push(line);

        // let line = Line {
        //     mode: unix_mode::to_string(meta.permissions().mode()),
        //     nlink: meta.nlink(),
        //     uid: users::get_user_by_uid(meta.uid()).unwrap(),
        //     user: uid.name().to_str().unwrap(),
        //     gid: users::get_group_by_gid(meta.gid()).unwrap(),
        //     group: gid.name().to_str().unwrap(),
        //     size: meta.size(),
        //     inode: meta.ino(),
        //     cdate: created.format("%b %e"),
        //     mtime: modified.format("%R"),
        //     name: filename.to_str().unwrap(),
        // }
        // lines.push(line)
    }

    // let max_lengths = SectionLength::default();
    let mut spacings = Vec::new();
    for i in 0..grid[0].len() {
        spacings.push(
            grid.iter()
                .map(|line| line[i].chars().count())
                .max()
                .unwrap(),
        );
    }

    // println!("{:?}", &.len());

    if args.l {
        for line in grid {
            for (part, spacing) in line.iter().zip(&spacings) {
                print!("{:>width$} ", part, width = spacing);
            }
            print!("\n");
        }
    } else {
        for line in grid {
            for part in line {
                print!("{} ", part);
            }
            print!(" ");
        }
        println!("")
    }

    Ok(())
}

fn show_directory(args: &Args, dir: &Path) -> Result<(), Box<dyn Error>> {
    // Get the file data for each file in the directory
    let files: Vec<_> = fs::read_dir(dir)?
        .map(|entry| entry.unwrap().path())
        .collect();
    // let files = entries.iter().map(|entry| entry.as_path());

    let mut files_new = Vec::new();

    for file in &files {
        let first_char = file
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .chars()
            .next()
            .unwrap();
        if first_char == '.' {
            if args.all {
                files_new.push(file.as_path());
            } else {
                continue;
            }
        } else {
            files_new.push(file.as_path());
        }
    }

    let total_blocks = 0;
    if args.l {
        println!("total {}", total_blocks);
    }

    show_files(args, &files_new)?;

    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    // OUTLINE
    // Process command line arguments
    let args = Args::parse();

    if args.files.is_empty() {
        show_directory(&args, &env::current_dir()?)?;
    } else {
        let files: Vec<&Path> = args.files.iter().map(|file| file.as_path()).collect();

        if !args.directory {
            let mut dirs: Vec<&Path> = files.iter().map(|f| *f).filter(|f| f.is_dir()).collect();
            let other_files: Vec<&Path> =
                files.iter().map(|f| *f).filter(|f| !f.is_dir()).collect();

            // Print files
            show_files(&args, &other_files)?;

            // Print directories
            if !dirs.is_empty() && !other_files.is_empty() {
                println!();
            }

            dirs.sort();
            if dirs.len() == 1 && other_files.len() == 0 {
                show_directory(&args, &dirs[0])?;
            } else {
                for (i, dir) in dirs.iter().enumerate() {
                    println!(
                        "{}:",
                        dir.file_name().unwrap().to_str().unwrap().to_string()
                    );
                    show_directory(&args, &dir)?;

                    if i != dirs.len() - 1 {
                        println!();
                    }
                }
            }
        } else {
            show_files(&args, &files)?;
        }
    }

    Ok(())
}
