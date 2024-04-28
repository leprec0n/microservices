use askama::Template;

#[derive(Template)]
#[template(path = "user_information.html")]
pub struct User {
    pub sub: String,
    pub balance: f64,
    pub currency: String,
    pub first_name: Option<String>,
    pub middle_name: Option<String>,
    pub last_name: Option<String>,
    pub postal_code: Option<String>,
    pub street_name: Option<String>,
    pub street_nr: Option<String>,
    pub premise: Option<String>,
    pub settlement: Option<String>,
    pub country: Option<String>,
    pub country_code: Option<String>,
}
