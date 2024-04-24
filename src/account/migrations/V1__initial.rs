use barrel::{backend::Pg, types, Migration};

pub fn migration() -> String {
    let mut m: Migration = Migration::new();

    m.create_table_if_not_exists("currency", |t| {
        t.add_column("id", types::primary());
        t.add_column("acronym", types::text());
    });

    m.create_table_if_not_exists("user", |t| {
        t.add_column("id", types::primary());
        t.add_column("email", types::text());
        t.add_column("balance", types::double());
        t.add_column("currency_id", types::integer());

        t.add_foreign_key(&["currency_id"], "currency", &["id"])
    });

    m.create_table_if_not_exists("session", |t| {
        t.add_column("id", types::primary());
        t.add_column("expires", types::custom("timestamp with time zone"));
        t.add_column("type", types::text());
        t.add_column("email_id", types::integer());

        t.add_foreign_key(&["email_id"], "user", &["id"]);
    });

    m.make::<Pg>()
}
