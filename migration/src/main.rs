use tokio_postgres::{NoTls};

type Error = Box<dyn std::error::Error>;

mod embedded {
    use refinery::embed_migrations;

    embed_migrations!("migrations");
}

async fn run_migrations() -> std::result::Result<(), Error> {
    println!("Running DB migrations...");
    let db_string =  &format!("host={} port={} user={} password={} dbname={}", "localhost", 11001, "local", "local", "notifications_db");
    let (mut client, con) =
        tokio_postgres::connect(db_string, NoTls)
            .await.expect("Connection failed");

    tokio::spawn(async move {
        if let Err(e) = con.await {
            eprintln!("connection error: {}", e);
        }
    });
    let migration_report = embedded::migrations::runner()
        .run_async(&mut client)
        .await?;
    for migration in migration_report.applied_migrations() {
        println!(
            "Migration Applied -  Name: {}, Version: {}",
            migration.name(),
            migration.version()
        );
    }
    println!("DB migrations finished!");

    Ok(())
}

#[tokio::main]
async fn main() {
    run_migrations().await.expect("can run DB migrations: {}");
}
