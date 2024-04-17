use std::collections::HashMap;

pub fn generate_db_conn(params: &HashMap<&str, &String>) -> String {
    format!(
        "postgresql://{user}:{password}@{host}/{db}",
        user = params.get("user").unwrap(),
        password = params.get("password").unwrap(),
        host = params.get("host").unwrap(),
        db = params.get("db").unwrap(),
    )
}
