use barrel::{backend::Pg, types, Migration};

pub fn migration() -> String {
    let mut m: Migration = Migration::new();

    m.create_table("account", |t| {
        t.add_column("id", types::primary().increments(true));
        t.add_column("access_token", types::text());
        t.add_column("expires", types::custom("timestamp with time zone"));
        t.add_column("scope", types::text());
        t.add_column("token_type", types::text());
    });

    m.create_table("email", |t| {
        t.add_column("id", types::serial());
        t.add_column("email", types::text());
        t.add_column("expires", types::custom("timestamp with time zone"));
    });

    m.make::<Pg>()
}
