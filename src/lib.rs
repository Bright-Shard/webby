pub mod compiler;
pub mod minifier;
pub mod translator;

use std::{fs, path::PathBuf};

type Cow<'a> = std::borrow::Cow<'a, str>;

pub enum Mode {
    Compile,
    Copy,
    Link,
}

pub enum FileType {
    Html,
    Css,
    Gemtext,
    Markdown,
    Unknown,
}

pub struct Target {
    pub path: PathBuf,
    pub output: PathBuf,
    pub mode: Mode,
    pub file_type: FileType,
}

pub fn build_target(target: Target) -> Result<(), Cow<'static>> {
    match target.mode {
        Mode::Copy => {
            if target.path.is_file() | target.path.is_symlink() {
                fs::copy(target.path, target.output).unwrap();
            } else {
                compiler::copy_batch_target(&target.path, &target.output);
            }
        }
        Mode::Link => {
            if target.output.exists() {
                fs::remove_file(&target.output)
                    .unwrap_or_else(|err| panic!("Failed to link target {:?}: {err}", &target.path))
            }
            fs::hard_link(&target.path, target.output)
                .unwrap_or_else(|err| panic!("Failed to link target {:?}: {err}", &target.path));
        }
        Mode::Compile => {
            let original = fs::read_to_string(&target.path).unwrap_or_else(|err| {
                panic!(
                    "Failed to compile target {:?}: Error occurred while reading the source file: {err}",
                    &target.path
                )
            });
            let compiled_macros = compiler::compile_macros(&original, &target.path)?;

            let output = match target.file_type {
                FileType::Gemtext => Cow::Owned(translator::translate_gemtext(
                    &target.path,
                    compiled_macros.as_ref(),
                )?),
                FileType::Html => Cow::Owned(minifier::minify_html(
                    target.path.to_str().unwrap(),
                    &compiled_macros,
                    &original,
                )?),
                FileType::Css => Cow::Owned(minifier::minify_css(&compiled_macros)),
                FileType::Markdown => Cow::Owned(translator::translate_markdown(&compiled_macros)),
                FileType::Unknown => compiled_macros,
            };

            fs::write(&target.output, output.as_ref())
                .unwrap_or_else(|err| panic!("Failed to compile target {:?}: Error occured while writing the compiled file: {err}", &target.path));
        }
    }

    Ok(())
}

fn line_number_of_offset(src: &str, offset: usize) -> usize {
    src[..offset].bytes().filter(|byte| *byte == b'\n').count()
}
