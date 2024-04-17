use askama::Template;

#[derive(Template)]
#[template(path = "snackbar.html")]
pub struct Snackbar<'a> {
    pub title: &'a str,
    pub message: &'a str,
    pub color: &'a str,
}
