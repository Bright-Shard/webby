use {
    std::{fs, path::PathBuf},
    webby::translator,
};

#[test]
fn test() {
    let tests = ["link", "header", "text", "list"];

    for test in tests {
        let gmi_path = PathBuf::from(format!("tests/gemtext/{test}.gmi"));
        let html_path = PathBuf::from(format!("tests/gemtext/{test}.html"));
        let html =
            translator::translate_gemtext(&gmi_path, &fs::read_to_string(&gmi_path).unwrap())
                .unwrap();
        assert_eq!(
            html,
            format!("<p>{}</p>", fs::read_to_string(&html_path).unwrap())
        )
    }
}
