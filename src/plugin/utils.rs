use once_cell::sync::Lazy;
use regex::Regex;

pub fn clean_manufacturer_name(name: &str) -> String {
    static SUFFIX_REGEX: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"(?i)[\s,]+(ltd|llc|lcc|inc|gmbh|corp|co|ag|a/s)\.?$").unwrap()
    });
    SUFFIX_REGEX.replace_all(name, "").trim().to_string()
}
