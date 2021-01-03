use std::borrow::BorrowMut;
use std::convert::TryInto;

use deadpool_postgres::{Client, Config, Pool};
use deadpool_postgres::tokio_postgres::{Error, Row};
use tokio_postgres::tls::NoTls;

const INSERT_NOTIFICATION: &str = "insert into email_notifications
(title, notification_type, template, email, pwd, smtp_server, smtp_port, description)
values($1, $2, $3, $4, $5, $6, $7, $8)
returning id";

const SELECT_NOTIFICATIONS: &str = "select id, title, notification_type, template, email, pwd, smtp_server, smtp_port, description
from email_notifications";
const SELECT_NOTIFICATION: &str = "
where id=$1";

#[derive(Debug, serde::Serialize)]
pub struct DbENRecord {
    id: i32,
    title: String,
    notification_type: i32,
    template: String,
    email: String,
    pwd: String,
    smtp_server: String,
    smtp_port: i32,
    description: String,
}

impl DbENRecord {
    pub fn create_new(title: String, notification_type: i32, template: String, email: String,
                      pwd: String, smtp_server: String, smtp_port: i32, description: String) -> DbENRecord {
        DbENRecord {
            id: 0,
            title,
            notification_type,
            template,
            email,
            pwd,
            smtp_server,
            smtp_port,
            description,
        }
    }
    fn record_from_future(row: Row) -> DbENRecord {
        DbENRecord {
            id: row.get(0),
            title: row.get(1),
            notification_type: row.get(2),
            template: row.get(3),
            email: row.get(4),
            pwd: row.get(5),
            smtp_server: row.get(6),
            smtp_port: row.get(7),
            description: row.get(8),

        }
    }
}

#[derive(Clone)]
pub struct PgConnection {
    pub config: Config,
    pub pg_connection: Option<Pool>,
}


impl<'a> PgConnection
{
    fn create_config() -> Config {
        let mut cfg = Config::new();
        cfg.dbname = Some("notifications_db".to_string());
        cfg.port = Some(11001);
        cfg.host = Some("localhost".to_string());
        cfg.user = Some("local".to_string());
        cfg.password = Some("local".to_string());
        cfg
    }

    pub fn new() -> PgConnection {
        let config = PgConnection::create_config();
        PgConnection {
            config,
            pg_connection: None,
        }
    }

    fn new_connection(&self) -> Pool {
        match self.config.create_pool(NoTls) {
            Ok(pool) => {
                pool
            }
            Err(error) => {
                panic!("Connection Pool Critical Error: \n{}", error);
            }
        }
    }

    pub fn create_connection(&mut self) {
        let pool = self.new_connection();
        self.pg_connection = Some(pool);
    }

    async fn get_client_imut(& self) -> Client {
        let pool = match &self.pg_connection {
            Some(pool) => {
                pool
            }
            None => panic!("Connection doesn't exist!")
        };
        match pool.get().await {
            Ok(client) => client,
            Err(error) => {
                panic!(error);
            }
        }
    }

    async fn get_client(&mut self) -> Client {
        let pool = match &self.pg_connection {
            Some(pool) => {
                pool
            }
            None => {
                &self.create_connection();
                self.pg_connection.as_ref().unwrap()
            }
        };
        match pool.get().await {
            Ok(client) => client,
            Err(error) => {
                eprint!("Connection error: \n {}", error);
                let pool = self.new_connection();
                let client = pool.get().await.unwrap();
                self.pg_connection.replace(pool);
                client
            }
        }
    }

    pub async fn insert_notification(&'a mut self, db_notification: &DbENRecord) -> Result<i32, Error> {
        let client = self.get_client().await;
        let stmt = client.prepare(INSERT_NOTIFICATION).await.unwrap();
        let rows = client.query(&stmt,
                                &[&db_notification.title, &db_notification.notification_type, &db_notification.template,
                                    &db_notification.email, &db_notification.pwd, &db_notification.smtp_server,
                                    &db_notification.smtp_port, &db_notification.description])
            .await?;
        let value: i32 = rows[0].get(0);
        print!("New Id {}", value);
        Ok(value)
    }

    pub async fn select_notifications_imut(&self) -> Result<Vec<DbENRecord>, Error> {
        let client = self.get_client_imut().await;
        let stmt = client.prepare(SELECT_NOTIFICATIONS).await.unwrap();
        let rows = client.query(&stmt, &[]).await?;
        let mut result: Vec<DbENRecord> = Vec::new();
        {
            for row in rows {
                let record = DbENRecord::record_from_future(row);
                result.push(record);
            }
        }
        Ok(result)
    }

    pub async fn select_notifications(&mut self) -> Result<Vec<DbENRecord>, Error> {
        let client = self.get_client().await;
        let stmt = client.prepare(SELECT_NOTIFICATIONS).await.unwrap();
        let rows = client.query(&stmt, &[]).await?;
        let mut result: Vec<DbENRecord> = Vec::new();
        {
            for row in rows {
                let record = DbENRecord::record_from_future(row);
                result.push(record);
            }
        }
        Ok(result)
    }

    pub async fn select_notification(& self, id: i32) -> Result<DbENRecord, Error> {
        let client = self.get_client_imut().await;
        let select_one_query = format!("{}{}", SELECT_NOTIFICATIONS, SELECT_NOTIFICATION);
        let stmt = client.prepare(&select_one_query).await.unwrap();
        let db_record = client.query_one(&stmt, &[&id]).await?;
        let row = DbENRecord::record_from_future(db_record);
        Ok(row)
    }
}


#[cfg(test)]
mod tests {
    use std::{thread, time};
    use std::ops::AddAssign;
    use std::sync::{Mutex, MutexGuard};

    use once_cell::sync::Lazy;

    use crate::db::accessor::*;

    pub static mut CURR_ID: Lazy<Mutex<i32>> = Lazy::new(|| Mutex::new(0));

    #[tokio::test]
    async fn insert_record() -> Result<(), Error> {
        unsafe {
            let locked = CURR_ID.lock();
            let record = DbENRecord::create_new(String::from("title"), 1, String::from("template"),
                                                String::from("test@app.com"),
                                                String::from("pwd"),
                                                String::from("localhost"),
                                                80,
                                                String::from("description"));
            let mut conn = PgConnection::new();
            let res = conn.insert_notification(&record).await?;

            locked.unwrap().add_assign(res);
            println!("\n Global has been updated to {}", CURR_ID.lock().unwrap());
        }
        Ok(())
    }

    #[tokio::test]
    async fn get_records() -> Result<(), Error> {
        let mut conn = PgConnection::new();
        let records = conn.select_notifications().await;
        println!("Records for DB are {:?}", records);
        Ok(())
    }

    #[tokio::test]
    async fn get_last() -> Result<(), Error> {
        let mut conn = PgConnection::new();
        let id: i32;
        unsafe {
            id = match CURR_ID.try_lock() {
                Ok(new_id) => *new_id,
                Err(err) => {
                    eprint!("\n Error is {:?}\n", err);
                    thread::sleep(time::Duration::from_millis(100));
                    *CURR_ID.lock().unwrap()
                }
            }
        }
        println!("New Id is: {}", id);
        let record = conn.select_notification(id).await?;
        println!("Records for DB are {:?}", record);
        Ok(())
    }
}
