use std::env;

pub fn get_env_var(s: &str) -> String {
    env::var_os(s).unwrap().into_string().unwrap()
}
