use {
    crate::{line_number_of_offset, minifier, Cow},
    base64::{engine::general_purpose::STANDARD, Engine},
    std::{fs, path::Path},
};

pub fn compile_macros<'a>(original: &'a str, source_path: &'a Path) -> Result<Cow<'a>, String> {
    let mut output = String::default();
    let mut offset = 0;

    while let Some(start_idx) = original[offset..].find("#!") {
        if original[offset..]
            .as_bytes()
            .get(start_idx.saturating_sub(1))
            .copied()
            == Some(b'\\')
        {
            if !output.is_empty() {
                output += &original[offset..offset + start_idx + 1]
            }
            offset += start_idx + 1;
            continue;
        }

        output += &original[offset..offset + start_idx];
        offset += start_idx;

        let macro_src = &original[offset..];
        let Some(paren_open) = macro_src.find('(') else {
            return Err(format!(
                "Expected ( in macro invocation at {source_path:?}:{}",
                line_number_of_offset(original, offset)
            ));
        };
        let mut paren_depth = 0u8;
        let mut bytes = macro_src[paren_open..].bytes().enumerate();
        let paren_close: usize;
        loop {
            let Some((idx, byte)) = bytes.next() else {
                return Err(format!(
                    "Expected ) to end macro invocation at {source_path:?}:{}",
                    line_number_of_offset(original, offset)
                ));
            };

            if byte == b'(' {
                paren_depth += 1;
            } else if byte == b')' {
                paren_depth -= 1;
                if paren_depth == 0 {
                    paren_close = idx + paren_open;
                    break;
                }
            }
        }

        let macro_name = &macro_src[2..paren_open];
        let macro_args = &macro_src[paren_open + 1..paren_close];
        let macro_args = compile_macros(macro_args, source_path)?;
        let macro_args = macro_args.as_ref();

        match macro_name {
            "INCLUDE" => {
                let path = source_path.parent().unwrap().join(macro_args);
                let src = fs::read_to_string(&path).unwrap_or_else(|err| {
                    panic!(
                        "Error in INCLUDE macro at {source_path:?}:{}: {err}",
                        line_number_of_offset(original, offset)
                    )
                });
                let compiled = compile_macros(&src, &path)?;
                output += compiled.as_ref();
            }
            "BASE64" => {
                output += STANDARD.encode(macro_args).as_str();
            }
            "INCLUDE_BASE64" => {
                let path = source_path.parent().unwrap().join(macro_args);
                let src = fs::read(&path).unwrap_or_else(|err| {
                    panic!(
                        "Error in INCLUDE_BASE64 macro at {source_path:?}:{}: {err}",
                        line_number_of_offset(original, offset)
                    )
                });
                output += STANDARD.encode(&src).as_str();
            }
            "MINIFY" => {
                let mut split = macro_args.split(',');
                let Some(file_type) = split.next() else {
                    return Err(format!("Expected two arguments (file type and code) in {macro_name} at {source_path:?}:{}", line_number_of_offset(original, offset)));
                };
                let remaining = &macro_args[file_type.len() + 1..];
                match file_type {
                    "html" => output += &minifier::minify_html(source_path.to_str().unwrap(), remaining, original)?,
                    "css" => output += &minifier::minify_css(remaining),
                    _ => return Err(format!("Unknown file type given in {macro_name} macro at {source_path:?}:{} - file type was `{file_type}`, but can only be html or css", line_number_of_offset(original, offset)))
                }
            }
            other => {
                return Err(format!(
                    "Unknown macro '{other}' in macro invocation at {source_path:?}:{}",
                    line_number_of_offset(original, offset)
                ))
            }
        }

        offset += paren_close + 1;
    }

    if output.is_empty() {
        Ok(Cow::Borrowed(original))
    } else {
        output += &original[offset..];
        Ok(Cow::Owned(output))
    }
}

pub fn copy_batch_target(src: &Path, dest: &Path) {
    if dest.is_file() {
        fs::remove_file(dest).unwrap_or_else(|err| {
                panic!("Failed to copy batch target {src:?}. There was already a file where its output should go ({dest:?}), which couldn't be removed: {err}");
            });
    }
    if !dest.exists() {
        fs::create_dir_all(dest).unwrap_or_else(|err| {
                panic!("Failed to copy batch target {src:?}. Couldn't create its output folder at {dest:?} because: {err}");
            });
    }

    let src = src.read_dir().unwrap_or_else(|err| {
        panic!(
            "Failed to copy batch target {dest:?}. Couldn't open its source directory because: {err}"
        );
    });

    for dir_entry in src.filter_map(|dir_entry| dir_entry.ok()) {
        let dir_entry = &dir_entry.path();

        if dir_entry.is_file() {
            fs::copy(dir_entry, dest.join(dir_entry.file_name().unwrap())).unwrap_or_else(|err| {
                panic!("Failed to copy batch target {dest:?}. Couldn't copy file at {dir_entry:?} because: {err}");
            });
        } else {
            copy_batch_target(dir_entry, &dest.join(dir_entry.file_name().unwrap()));
        }
    }
}
