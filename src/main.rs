use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use serde::Serialize;

use structopt::StructOpt;
use tinytemplate::TinyTemplate;

#[derive(StructOpt)]
struct Arguments {
    #[structopt(short, default_value = ".", parse(from_os_str))]
    directory: PathBuf,
    #[structopt(short, default_value = "1")]
    level: usize,
    #[structopt(short)]
    blacklist: Vec<String>,
    #[structopt(short)]
    whitelist: Vec<String>,
    #[structopt(subcommand)]
    operation: Operation,
}

#[derive(StructOpt)]
enum Operation {
    Analyze,
    Rename {
        #[structopt(short = "n", default_value = "1")]
        start_number: usize,
        #[structopt(short = "o", default_value = "./output", parse(from_os_str))]
        output_directory: PathBuf,
        #[structopt(short = "t")]
        template: String,
    },
}

#[derive(Serialize)]
struct TemplateContext {
    number: usize,
}

fn ls(path: &Path) -> Vec<PathBuf> {
    fs::read_dir(path)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect()
}

fn ls_recursive(path: &Path, level: usize) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for path in ls(path) {
        paths.push(path.clone());
        if path.is_dir() && level > 0 {
            paths.append(&mut ls_recursive(&path, level - 1));
        }
    }
    paths
}

fn get_extension_str(path: &Path) -> String {
    match path.extension() {
        Some(extension) => extension.to_str().unwrap().to_string().to_lowercase(),
        None => String::from(""),
    }
}

fn filter_sum<K, V>(map: &HashMap<K, V>, set: &HashSet<K>) -> V
where
    K: std::cmp::Eq,
    K: std::hash::Hash,
    V: std::iter::Sum,
    V: Copy,
{
    map.iter()
        .filter(|entry| set.contains(entry.0))
        .map(|entry| *entry.1)
        .sum()
}

fn analyze(
    directory_path: &Path,
    level: usize,
    blacklist: HashSet<String>,
    whitelist: HashSet<String>,
) {
    let paths = ls_recursive(directory_path, level);
    let files = paths
        .iter()
        .filter(|path| !path.is_dir() && path.extension() != None);
    let file_count = files.clone().count();
    let file_types: HashMap<String, usize> =
        files
            .clone()
            .fold(HashMap::default(), |mut accumulator, file| {
                let extension = get_extension_str(file);
                *accumulator.entry(extension).or_insert(0) += 1;
                accumulator
            });
    print!("Detected {} file(s), ", file_count);
    if whitelist.is_empty() {
        println!("{} in blacklist", filter_sum(&file_types, &blacklist));
    } else {
        println!("{} in whitelist", filter_sum(&file_types, &whitelist));
    }
    if file_count > 0 {
        println!("File type(s):");
        for file_type in file_types {
            println!("\t{} '{}' file(s)", file_type.1, file_type.0)
        }
    }
}

fn rename(
    input_directory: &Path,
    level: usize,
    blacklist: HashSet<String>,
    whitelist: HashSet<String>,
    output_directory: &Path,
    start_number: usize,
    template: String,
) {
    if output_directory.exists() {
        fs::remove_dir_all(&output_directory).unwrap();
    }
    fs::create_dir_all(&output_directory).unwrap();
    let mut number = start_number;
    let mut tiny_template = TinyTemplate::new();
    tiny_template.add_template("rename", &template).unwrap();
    let files: Vec<PathBuf> = ls_recursive(input_directory, level)
        .iter()
        .filter(|path| !path.is_dir())
        .map(|path| path.clone())
        .collect();
    for file in files {
        let extension = get_extension_str(&file);
        if extension.is_empty() {
            continue;
        } else if whitelist.is_empty() {
            if blacklist.contains(&extension) {
                continue;
            }
        } else {
            if !whitelist.contains(&extension) {
                continue;
            }
        }
        let context = TemplateContext { number: number };
        fs::copy(
            file,
            &output_directory.join(tiny_template.render("rename", &context).unwrap()),
        )
        .unwrap();
        number += 1;
    }
}

fn main() {
    let arguments = Arguments::from_args();
    let blacklist: HashSet<String> = arguments.blacklist.into_iter().collect();
    let whitelist: HashSet<String> = arguments.whitelist.into_iter().collect();
    match arguments.operation {
        Operation::Analyze => analyze(&arguments.directory, arguments.level, blacklist, whitelist),
        Operation::Rename {
            start_number,
            output_directory,
            template,
        } => rename(
            &arguments.directory,
            arguments.level,
            blacklist,
            whitelist,
            &output_directory,
            start_number,
            template,
        ),
    };
}
