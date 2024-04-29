use askama::Template;

#[derive(Template)]
#[template(path = "snackbar.html")]
pub struct Snackbar<'a> {
    pub title: &'a str,
    pub message: &'a str,
    pub color: &'a str,
}

impl<'a> Snackbar<'a> {
    pub fn new() -> Snackbar<'a> {
        return Snackbar {
            title: "Error",
            message: "Could not process request",
            color: "red",
        };
    }
}
