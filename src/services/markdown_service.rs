use pulldown_cmark::{html, Options, Parser};
use regex::Regex;

pub fn render(markdown: &str) -> String {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_FOOTNOTES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TASKLISTS);
    opts.insert(Options::ENABLE_SMART_PUNCTUATION);

    let parser = Parser::new_ext(markdown, opts);
    let mut html_out = String::new();
    html::push_html(&mut html_out, parser);
    html_out
}

pub fn extract_wikilinks(content: &str) -> Vec<String> {
    let re = Regex::new(r"\[\[([^\[\]|]+?)(?:\|[^\[\]]+?)?\]\]").unwrap();
    re.captures_iter(content)
        .map(|cap| cap[1].trim().to_string())
        .collect()
}

pub fn word_count(markdown: &str) -> usize {
    markdown.split_whitespace().count()
}
