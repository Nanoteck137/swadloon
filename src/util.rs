use regex::Regex;

// Same as: https://github.com/metafates/mangal/blob/main/util/util.go
pub fn sanitize_name(name: &str) -> String {
    let rep = [
        (Regex::new(r#"[\\/<>:;"'|?!*{}#%&^+,~\s]"#).unwrap(), "_"),
        (Regex::new(r#"__+"#).unwrap(), "_"),
        (Regex::new(r#"^[_\-.]+|[_\-.]+$"#).unwrap(), ""),
    ];

    let mut name = name.to_string();

    for i in rep {
        name = i.0.replace_all(&name, i.1).to_string();
    }

    name
}
