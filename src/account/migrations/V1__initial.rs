use barrel::{backend::Pg, types, Migration};

pub fn migration() -> String {
    let mut m: Migration = Migration::new();

    m.create_table_if_not_exists("currencies", |t| {
        t.add_column("id", types::primary());
        t.add_column("acronym", types::text());
    });

    m.create_table_if_not_exists("users", |t| {
        t.add_column("id", types::primary());
        t.add_column("sub", types::text());
        t.add_column("balance", types::double());
        t.add_column("currency_id", types::integer());

        t.add_foreign_key(&["currency_id"], "currencies", &["id"])
    });

    m.create_table_if_not_exists("sessions", |t| {
        t.add_column("id", types::primary());
        t.add_column("expires", types::custom("timestamp with time zone"));
        t.add_column("type", types::text());
        t.add_column("sub_id", types::integer());

        t.add_foreign_key(&["sub_id"], "users", &["id"]);
    });

    m.make::<Pg>()
}
