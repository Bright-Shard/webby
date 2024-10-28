# webby

> The smol web compiler

As seen in [my website](https://github.com/bright-shard/website).

**webby** is a small and efficient compiler for making static sites. It adds macros, minifiers, and translators to compile your project into a tiny static site.

> Note: Webby is WIP. The above is a summary of what I want it to do when it's finished. For the current project status, see [todo](#todo).

# macros

webby adds a few simple macros to make writing HTML simpler. Macros open with `#!`, followed by the macro name, followed by arguments in parentheses, like so:

```
#!MACRO_NAME(args)
```

Macros can be combined, like this:

```
#!MACRO1(#!MACRO2(args))
```

- `#!INCLUDE(path/to/file)`: Webby will compile the given file, then embed it at the macro's location. The file must contain valid UTF-8 text.
- `#!BASE64(text)`: Base64-encode the given text.
- `#!MINIFY(format, text)`: Run `text` through the minifier for `format`. For example, `#!MINIMISE(html, <body>    <p  >hello!</p></body>)` will output `<body><p>hello!</p></body>`.
- `#!INCLUDE_BASE64(path/to/file)`: Base64-encode the given file. This differs from `#!BASE64(#!INCLUDE(path/to/file))` because it can also base64-encode binary files.

# minifiers

webby will automatically strip comments and unneeded whitespace from your code to make it as small as possible.

# translators

Translators cross-compile between languages - for example, Markdown to HTML, or Gemtext to HTML.



# usage

webby projects have a `webby.toml` in the root of their project, just like Rust projects have a `Cargo.toml` in the root of theirs. The format of `webby.toml` is given in [config](#config).

To install webby, just install it with Cargo:

```sh
cargo install --git https://github.com/bright-shard/webby
```

Then just run `webby` in your webby project.

# config

In its simplest form, the `webby.toml` file will look like this:

```toml
# For every file you want to compile with webby, add a `[[target]]` section
[[target]]
# The path to the file to compile
path = "index.html"

[[target]]
path = "blog.html"
```

However, webby allows customising more if you need it:

```toml
# (Optional) the directory to put the output files at
# If this isn't specified it defaults to `webby`
# The path is relative to the webby.toml file
output = "my/custom/build/dir"

[[target]]
# The path to the file, relative to the webby.toml file
# If you list a folder instead of a file, webby will compile all of the files
# in that folder
path = "path/to/file.html"
# (Optional) Where to put the compiled file
# If this isn't specified it defaults to the name of the file given in path
# The path is relative to the output directory
output = "file.out.html"
# (Optional) The compilation mode
# This can be "compile", "copy", or "link". Compile will compile the file. Copy
# will just copy the file as-is and will not compile it at all. Link is the same
# as copy, but it creates a hard link (not a symlink) to the file instead of
# copying it.
# If this isn't specified, webby will infer if it should compile or copy the
# file based on the file's ending.
mode = "compile"
# (Optional) Override the file type
# By default webby will treat files differently based on their file type. Files
# ending in .html will be run through the HTML minifier, while files ending in
# .css will be run through the CSS minifier. This setting will make webby treat
# the file as what's given here instead of the file's file extension.
filetype = "html"
```

# todo

- [x] Macros
  - [x] INCLUDE
  - [x] BASE64
  - [x] BASE64_INCLUDE
- [x] HTML minifier
- [x] CSS minifier
- [ ] JS minifier
- [x] Gemtext translator
- [ ] Markdown translator
- [ ] Redo macro compiler... it's old and has bugs
- [ ] Replace any instances of `panic!()` with returning an error string
