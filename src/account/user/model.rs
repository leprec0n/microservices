use std::fmt;

pub struct User {
    pub sub: String,
    pub balance: f64,
    pub currency: Currency,
}

#[derive(Debug)]
#[allow(clippy::upper_case_acronyms)]
pub enum Currency {
    EUR,
}

impl fmt::Display for Currency {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
