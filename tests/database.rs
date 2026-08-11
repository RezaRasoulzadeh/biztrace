// tests/database.rs

use nexora::database::Database;

#[test]
fn initial_schema_is_created_and_versioned() {
    let database = Database::open_in_memory().unwrap();
    assert_eq!(database.schema_version().unwrap(), 1);
    assert_eq!(database.overview_counts().unwrap(), Default::default());
}

#[test]
fn migrations_are_idempotent_for_new_connections() {
    let first = Database::open_in_memory().unwrap();
    let second = Database::open_in_memory().unwrap();
    assert_eq!(
        first.schema_version().unwrap(),
        second.schema_version().unwrap()
    );
}
