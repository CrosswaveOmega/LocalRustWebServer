use password_auth::generate_hash;
use sqlx::SqlitePool;
use std::error::Error;
use std::io::{self, Write};

///Add a new user with this access level
pub async fn adduser(
    username: &str,
    password: &str,
    access_level: i32,
) -> Result<(), Box<dyn Error>> {
    let db = SqlitePool::connect("./thisbackend.db").await?;

    let existing_user: Option<(String,)> = sqlx::query_as(
        r#"
        SELECT username FROM users WHERE username = ?
        "#,
    )
    .bind(username)
    .fetch_optional(&db)
    .await?;

    if existing_user.is_some() {
        return Err("Username already exists".into());
    }

    let hash = generate_hash(password);
    sqlx::query(
        r#"
        INSERT INTO users (username, password, access_level)
        VALUES (?, ?,?)
        "#,
    )
    .bind(username)
    .bind(hash)
    .bind(access_level)
    .execute(&db)
    .await?;

    println!("User '{}' added successfully.", username);
    Ok(())
}

pub async fn adduser_from_prompt() -> Result<(), Box<dyn Error>> {
    let mut is_ok = false;

    while !is_ok {
        let mut username = String::new();
        let mut password = String::new();
        let mut access_level_input = String::new();

        print!("Enter username: ");
        io::stdout().flush()?;
        io::stdin().read_line(&mut username)?;
        let username = username.trim().to_string();

        print!("Enter password: ");
        io::stdout().flush()?;
        io::stdin().read_line(&mut password)?;
        let password = password.trim().to_string();

        print!("Enter access level: ");
        io::stdout().flush()?;
        io::stdin().read_line(&mut access_level_input)?;
        let access_level: i32 = match access_level_input.trim().parse() {
            Ok(level) => level,
            Err(_) => {
                println!("Invalid access level. Please enter a valid number.");
                continue;
            }
        };

        print!(
            "Accept username '{}', password '******', access level {}? (y/n): ",
            username, access_level
        );
        io::stdout().flush()?;
        let mut confirmation = String::new();
        io::stdin().read_line(&mut confirmation)?;

        if confirmation.trim().eq_ignore_ascii_case("y") {
            is_ok = true;

            // Call function with parsed values
            return adduser(&username, &password, access_level).await;
        }
    }

    Ok(())
}
