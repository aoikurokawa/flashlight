use std::{
    env,
    path::{Path, PathBuf},
};

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut target_dir = ".";
    if args.len() >= 2 {
        target_dir = &args[1];
    }

    let target = PathBuf::from(target_dir);
    println!("{}", target_dir);
    tree(&target, 0);
}

fn tree(target: &Path, level: isize) {
    let files = target.read_dir().expect("does not exist");

    for ent in files {
        let path = ent.unwrap().path();
        for _ in 1..=level {
            print!("|  ");
        }
        let fname = path.file_name().unwrap();
        if path.is_dir() {
            println!("|--<{:?}>", fname);
            tree(&path, level + 1);
            continue;
        }
        println!("|-- {:?}", fname);
    }
}
