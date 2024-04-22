use std::fmt;

#[derive(Debug)]
pub enum Currency {
    EUR,
}

impl fmt::Display for Currency {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub struct Balance {
    pub amount: f64,
    pub currency: Currency,
}
