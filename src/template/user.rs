use askama::Template;

#[derive(Template)]
#[template(path = "user_information.html", escape = "none")]
pub struct UserInformation {
    #[template(path = "user_information/account_details.html")]
    pub account_details: AccountDetails,
    pub name_input: NameInput,
    pub address_input: AddressInput,
}

#[derive(Template)]
#[template(path = "user_information/account_details.html")]
pub struct AccountDetails {
    pub sub: String,
    pub balance: f64,
    pub currency: String,
}

#[derive(Template)]
#[template(path = "user_information/name_input.html")]
pub struct NameInput {
    pub first_name: Option<String>,
    pub middle_name: Option<String>,
    pub last_name: Option<String>,
}

#[derive(Template)]
#[template(path = "user_information/address_input.html")]
pub struct AddressInput {
    pub postal_code: Option<String>,
    pub street_name: Option<String>,
    pub street_nr: Option<String>,
    pub premise: Option<String>,
    pub settlement: Option<String>,
    pub country: Option<String>,
    pub country_code: Option<String>,
}
