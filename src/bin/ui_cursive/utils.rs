use std::borrow::Cow;
use cursive::{theme::{BaseColor, Color}, utils::markup::StyledString};

pub fn truncate_text<'a, S>(text: &'a S, len: usize) -> Cow<'a, str> where S: AsRef<str> {
    let ellipsis = "…";
    // let ellipsis = "...";
    let text = text.as_ref();
    if text.chars().count() <= len {
        return Cow::Borrowed(text);
    } else if len == 0 {
        return Cow::Borrowed("");
    }

    let result = text
    .chars()
    .take(len - ellipsis.len())
    .chain(ellipsis.chars())
    .collect();
    Cow::Owned(result)
}

pub fn get_style_no_dump<S>(content: S) -> StyledString where S: AsRef<str> {
    StyledString::styled(content.as_ref(), Color::Light(BaseColor::Magenta))
}

pub fn get_style_bad_dump<S>(content: S) -> StyledString where S: AsRef<str> {
    StyledString::styled(content.as_ref(), Color::Light(BaseColor::Red))
}