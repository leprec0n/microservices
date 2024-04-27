use askama::Template;

#[derive(Template)]
#[template(path = "user_information.html")]
pub struct User<'a> {
    pub sub: &'a str,
    pub balance: &'a f64,
}
