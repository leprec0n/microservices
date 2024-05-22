use askama::Template;
use serde::Deserialize;

#[derive(Template)]
#[template(path = "catalog.html")]
pub struct Catalogs {
    pub catalogs: Vec<Catalog>,
}

#[derive(Deserialize, Debug)]
pub struct Catalog {
    pub name: String,
    pub description: String,
}
