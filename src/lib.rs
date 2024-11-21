pub mod compiler;
pub mod minifier;
pub mod translator;

use std::{
    fs::{self, File},
    path::{Path, PathBuf},
};

type Cow<'a> = std::borrow::Cow<'a, str>;

#[derive(Clone, Copy)]
pub enum Mode {
    Compile,
    Copy,
    Link,
}

#[derive(Clone, Copy)]
pub enum FileType {
    Html,
    Css,
    Gemtext,
    Markdown,
    Unknown,
}
impl From<&Path> for FileType {
    fn from(value: &Path) -> Self {
        match value.extension().and_then(|str| str.to_str()) {
            Some("html") => FileType::Html,
            Some("css") => FileType::Css,
            Some("gmi") | Some("gemtext") => FileType::Gemtext,
            Some("md") | Some("markdown") => FileType::Markdown,
            _ => FileType::Unknown,
        }
    }
}

pub struct Target {
    pub path: PathBuf,
    pub output: PathBuf,
    pub mode: Mode,
    pub file_type: FileType,
}

pub fn build_target(target: Target) -> Result<(), Cow<'static>> {
    let op: fn(&Path, &Path, &Path, FileType) -> Result<(), String> = match target.mode {
        Mode::Copy => |target_path, path, output, _| {
            fs::copy(path, output).map_err(|err| {
                    panic!("Failed to copy batch target {target_path:?}. Couldn't copy file at {path:?} because: {err}");
                }).map(|_| {})
        },
        Mode::Link => |target_path, path, output, _| {
            fs::hard_link(path, output)
                .map_err(|err| format!("Failed to link target {target_path:?}: {err}"))
        },
        Mode::Compile => |target_path, path, output, file_type| {
            let original = fs::read_to_string(path).map_err(|err| {
                    format!(
                        "Failed to compile target {target_path:?}: Error occurred while reading the source file: {err}"
                    )
                })?;

            let compiled = compile_file(&original, path, file_type)?;

            fs::write(output, compiled.as_ref())
                    .map_err(|err| format!("Failed to compile target {target_path:?}: Error occured while writing the compiled file: {err}"))
        },
    };

    if target.path.is_file() | target.path.is_symlink() {
        op(&target.path, &target.path, &target.output, target.file_type)?;
    } else {
        if !target.output.exists() {
            fs::create_dir_all(&target.output).map_err(|err| {
                            format!("Failed to copy batch target {:?}. Couldn't create its output folder at {:?} because: {err}", &target.path, &target.output)
                        })?;
        }

        let src = target.path.read_dir().map_err(|err| {
                    format!(
                        "Failed to copy batch target {:?}. Couldn't open its source directory because: {err}", &target.path
                    )
                })?;

        for dir_entry in src.filter_map(|dir_entry| dir_entry.ok()) {
            let dir_entry = dir_entry.path();

            if dir_entry.is_file() {
                let output = target.output.join(dir_entry.file_name().unwrap());
                op(
                    &target.path,
                    &dir_entry,
                    &output,
                    FileType::from(dir_entry.as_path()),
                )?;
            } else {
                let subdir = dir_entry.file_name().unwrap();
                let subtarget = Target {
                    path: target.path.join(subdir),
                    output: target.output.join(subdir),
                    mode: target.mode,
                    file_type: target.file_type,
                };
                build_target(subtarget)?;
            }
        }
    }

    Ok(())
}

fn compile_file<'a>(
    input: &'a str,
    source_path: &'a Path,
    file_type: FileType,
) -> Result<Cow<'a>, String> {
    let compiled_macros = compiler::compile_macros(input, source_path)?;

    let output = match file_type {
        FileType::Gemtext => Cow::Owned(translator::translate_gemtext(
            source_path,
            compiled_macros.as_ref(),
        )?),
        FileType::Html => Cow::Owned(minifier::minify_html(
            source_path.to_str().unwrap(),
            &compiled_macros,
            input,
        )?),
        FileType::Css => Cow::Owned(minifier::minify_css(&compiled_macros)),
        FileType::Markdown => Cow::Owned(translator::translate_markdown(&compiled_macros)),
        FileType::Unknown => compiled_macros,
    };

    Ok(output)
}

fn line_number_of_offset(src: &str, offset: usize) -> usize {
    src[..offset].bytes().filter(|byte| *byte == b'\n').count()
}
