use std::env;
use std::error::Error;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::path::PathBuf;

use chrono::{offset::Local, DateTime};
use clap::Parser;
use itertools::izip;
use unix_mode;
use users;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(value_parser)]
    files: Vec<PathBuf>,
    #[clap(short, long, value_parser)]
    all: bool,
    #[clap(short = 'A', long, value_parser)]
    almost_all: bool,
    #[clap(long, value_parser)]
    author: bool,
    #[clap(short = 'B', long, value_parser)]
    ignore_backups: bool,
    #[clap(short, value_parser)]
    c: bool,
    #[clap(short, value_parser)]
    l: bool,
    #[clap(short, value_parser)]
    g: bool,
    #[clap(short = 'G', long, value_parser)]
    no_group: bool,
    #[clap(short, long, value_parser)]
    numeric_uid_gid: bool,
    #[clap(short, long, value_parser)]
    inode: bool,
    #[clap(short, long, value_parser)]
    directory: bool,
    #[clap(short, value_parser)]
    t: bool,
    #[clap(short, long, value_parser)]
    group_directories_first: bool,
    #[clap(short, long, value_parser)]
    reverse: bool,
    #[clap(short, long, value_parser)]
    size: bool,
    #[clap(short = 'U', value_parser)]
    uu: bool,
    #[clap(skip)]
    long_listing: bool,
}

#[derive(Copy, Clone)]
enum Align {
    Left,
    Right,
    None,
}

fn show_files(args: &Args, files_old: &Vec<&Path>) -> Result<(), Box<dyn Error>> {
    if files_old.is_empty() {
        return Ok(());
    }

    let mut files = files_old.clone();
    // Sort the file data
    if !args.uu {
        if args.c && (!args.l || args.t) {
            files.sort_by(|a, b| {
                a.symlink_metadata()
                    .unwrap()
                    .created()
                    .unwrap()
                    .cmp(&b.symlink_metadata().unwrap().created().unwrap())
                    .reverse()
            });
        } else {
            files.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
        }

        if args.reverse {
            files.reverse();
        }
        // second ordering is "major" ordering
        if args.group_directories_first {
            files.sort_by(|a, b| a.is_dir().cmp(&b.is_dir()).reverse());
        }
    }

    let mut grid = Vec::new();

    let total_blocks = 0;

    let mut align = Vec::new();

    // Print the file data
    for (i, file) in files.iter().enumerate() {
        // let filename = file.as_os_str().to_str().unwrap().to_string();
        let filename = file.file_name().or(Some(file.as_os_str())).unwrap();
        let meta = file.symlink_metadata()?;
        let created: DateTime<Local> = meta.created()?.into();
        let modified: DateTime<Local> = meta.modified()?.into();
        // println!("{:?} {:?}", meta.blksize(), meta.blocks());

        let mut line = Vec::new();

        if args.inode {
            line.push(meta.ino().to_string());
            if i == 0 {
                align.push(Align::Right);
            }
        }

        if args.size {
            // note: not necessarily 0...
            let blocks = meta.blocks();
            let blocks_adj = blocks / (meta.blksize() / 512);
            line.push(blocks_adj.to_string());
            if i == 0 {
                align.push(Align::Right);
            }
        }

        if args.long_listing {
            line.push(unix_mode::to_string(meta.permissions().mode()));
            if i == 0 {
                align.push(Align::Left);
            }
            line.push(meta.nlink().to_string());
            if i == 0 {
                align.push(Align::Right);
            }
            let user_string = if args.numeric_uid_gid {
                meta.uid().to_string()
            } else {
                users::get_user_by_uid(meta.uid())
                    .unwrap()
                    .name()
                    .to_str()
                    .unwrap()
                    .to_string()
            };
            let user_align = if args.numeric_uid_gid {
                Align::Right
            } else {
                Align::Left
            };
            let group_string = if args.numeric_uid_gid {
                meta.gid().to_string()
            } else {
                users::get_group_by_gid(meta.gid())
                    .unwrap()
                    .name()
                    .to_str()
                    .unwrap()
                    .to_string()
            };
            if !args.g {
                line.push(user_string.clone());
                if i == 0 {
                    align.push(user_align.clone());
                }
            }
            if !args.no_group {
                line.push(group_string);
                if i == 0 {
                    align.push(user_align);
                }
            }
            if args.author {
                line.push(user_string);
                if i == 0 {
                    align.push(user_align);
                }
            }
            line.push(meta.size().to_string());
            if i == 0 {
                align.push(Align::Right);
            }
            line.push(created.format("%b %e").to_string());
            if i == 0 {
                align.push(Align::Left);
            }
            let time = if args.c { created } else { modified };
            line.push(time.format("%R").to_string());
            if i == 0 {
                align.push(Align::Left);
            }
        }
        let mut name = filename.to_str().unwrap().to_string();
        if args.long_listing {
            if file.is_symlink() {
                name += " -> ";
                name += fs::read_link(file)?.to_str().unwrap();
            }
        }
        line.push(name);
        if i == 0 {
            align.push(Align::None);
        }

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

    // println!("{:?}", align.len());

    if args.long_listing {
        for line in grid {
            for (part, spacing, align) in izip!(&line, &spacings, &align) {
                match align {
                    Align::Right => print!("{:>width$} ", part, width = spacing),
                    Align::Left => print!("{:<width$} ", part, width = spacing),
                    Align::None => print!("{} ", part),
                }
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
        let mut chars = file.file_name().unwrap().to_str().unwrap().chars();
        let first_char = chars.next().unwrap();
        let last_char = chars.last().unwrap();
        if !(args.all || args.almost_all) && first_char == '.' {
            continue;
        } else if args.ignore_backups && last_char == '~' {
            continue;
        }
        files_new.push(file.as_path());
    }

    if args.all {
        files_new.push(&Path::new("."));
        files_new.push(&Path::new(".."));
    }

    let total_blocks = 0;
    if args.long_listing {
        println!("total {}", total_blocks);
    }

    show_files(args, &files_new)?;

    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let (height, width) = termion::terminal_size().unwrap();

    // OUTLINE
    // Process command line arguments
    let mut args = Args::parse();

    args.long_listing = args.l || args.numeric_uid_gid;

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
            if args.reverse {
                dirs.reverse();
            }

            if dirs.len() == 1 && other_files.len() == 0 {
                show_directory(&args, &dirs[0])?;
            } else {
                for (i, dir) in dirs.iter().enumerate() {
                    let filename = dir.file_name().or(Some(dir.as_os_str())).unwrap();
                    println!("{}:", filename.to_str().unwrap().to_string());
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
