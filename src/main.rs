use {
    boml::{table::TomlGetError, Toml},
    std::{
        borrow::Cow,
        env, fs,
        path::Path,
        thread::{self, JoinHandle},
    },
    webby::{build_target, FileType, Mode, Target},
};

type ErrorMsg = Cow<'static, str>;
type Tasks = Vec<JoinHandle<Result<(), ErrorMsg>>>;

pub fn main() -> Result<(), ErrorMsg> {
    let cwd = env::current_dir().expect("Failed to find current directory");
    let mut root = cwd.as_path();

    while !root
        .read_dir()
        .expect("Failed to list files in current folder")
        .any(|file| {
            if let Ok(ref file) = file {
                if let Some(name) = file.file_name().to_str() {
                    if name == "webby.toml" && file.path().is_file() {
                        return true;
                    }
                }
            }

            false
        })
    {
        let Some(parent) = root.parent() else {
            return Err("Failed to find webby.toml".into());
        };
        root = parent;
    }

    let cfg = fs::read_to_string(root.join("webby.toml")).expect("Failed to read webby.toml");
    let toml = Toml::parse(&cfg).unwrap();

    let tasks = parse_cfg(toml, root)?;
    for task in tasks {
        match task.join().unwrap() {
            Ok(()) => {}
            Err(err) => println!("{err}"),
        }
    }

    Ok(())
}

fn parse_cfg(toml: Toml, root: &Path) -> Result<Tasks, ErrorMsg> {
    let output_dir = if let Ok(output) = toml.get_string("output") {
        root.join(output)
    } else {
        root.join("webby")
    };

    if !output_dir.exists() {
        let Ok(()) = fs::create_dir(&output_dir) else {
            return Err("Failed to create output directory".into());
        };
    }

    let mut tasks = Vec::default();

    match toml.get_array("target") {
        Ok(targets) => {
            for target in targets {
                let Some(table) = target.table() else {
                    return Err("All target entries in webby.toml must be a TOML table.".into());
                };
                let Ok(path) = table.get_string("path") else {
                    return Err("Target in webby.toml didn't have a path".into());
                };
                let path = root.join(path);
                let mode = if let Ok(mode) = table.get_string("mode") {
                    match mode {
                        "compile" => Mode::Compile,
                        "copy" => Mode::Copy,
                        "link" => Mode::Link,
                        other => {
                            return Err(format!("Unknown mode: {other} for target: {path:?}").into())
                        }
                    }
                } else {
                    match path.extension().and_then(|osstr| osstr.to_str()) {
                        Some("gmi" | "html" | "svg" | "md" | "css") => Mode::Compile,
                        _ => Mode::Copy,
                    }
                };
                let output = if let Ok(output_name) = table.get_string("output") {
                    output_dir.join(output_name)
                } else {
                    output_dir.join(path.file_name().unwrap())
                };
                let file_type = if let Ok(file_type) = table.get_string("filetype") {
                    match file_type {
                    "html" => FileType::Html,
                    "css" => FileType::Css,
                    "gmi" | "gemtext" => FileType::Gemtext,
                    "markdown" | "md" => FileType::Markdown,
                    _ => return Err(format!("Target `{path:?}` had an unexpected filetype: {file_type}\n`filetype` must be one of html, css, or gemtext").into())
                }
                } else {
                    match path.extension().and_then(|str| str.to_str()) {
                        Some("html") => FileType::Html,
                        Some("css") => FileType::Css,
                        Some("gmi") | Some("gemtext") => FileType::Gemtext,
                        Some("md") | Some("markdown") => FileType::Markdown,
                        _ => FileType::Unknown,
                    }
                };

                let target = Target {
                    path,
                    output,
                    mode,
                    file_type,
                };
                let worker = thread::spawn(move || build_target(target));
                tasks.push(worker);
            }
        }
        Err(e) => match e {
            TomlGetError::InvalidKey => {
                return Err("No targets specified. See the GitHub for an example on setting up a webby project: https://github.com/bright-shard/webby".into());
            }
            TomlGetError::TypeMismatch(_, _) => {
                return Err("The 'target' entry has to an array in webby.toml".into());
            }
        },
    }

    Ok(tasks)
}
