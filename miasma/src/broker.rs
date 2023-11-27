use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use serde_with::DisplayFromStr;
use std::net::IpAddr;
use surrealdb::{
    self,
    engine::remote::ws::{Client, Ws},
    opt::{auth::Root, PatchOp},
    sql::{Datetime, Thing},
    Surreal,
};

#[derive(Debug)]
pub enum BrokerError {
    ClientDoesNotExist,
}

pub type Result<T> = std::result::Result<T, BrokerError>;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    identifier: u64,
    content: Vec<u8>,
}

#[serde_as()]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Node {
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub ip: Option<IpAddr>,
    pub registed_at: Datetime,
    pub last_seen: Datetime,
    pub messages: Vec<Message>,
}

impl Node {
    pub fn new(ip: Option<IpAddr>) -> Node {
        Node {
            ip,
            last_seen: Datetime::default(),
            registed_at: Datetime::default(),
            messages: vec![],
        }
    }
}

pub struct SurrealBroker {
    client: Surreal<Client>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Record {
    id: Thing,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SurrealId(pub String);

impl SurrealId {
    fn qualified(&self) -> String {
        format!("node:{}", self.0)
    }
}

impl From<String> for SurrealId {
    fn from(value: String) -> Self {
        SurrealId(value)
    }
}

impl SurrealBroker {
    pub async fn new(
        endpoint: &str,
        namespace: &str,
        database: &str,
        username: &str,
        password: &str,
    ) -> SurrealBroker {
        let client = surrealdb::Surreal::new::<Ws>(endpoint)
            .await
            .expect(&format!(
                "Failed connecting to surrealdb endpoint :{}",
                &endpoint
            ));

        client
            .signin(Root {
                username: &username,
                password: &password,
            })
            .await
            .expect("Sign in failed");

        client.use_ns(namespace).use_db(database).await.unwrap();

        SurrealBroker { client }
    }

    pub async fn add_node(&self, node: Node) -> SurrealId {
        let mut dota: Vec<Record> = self.client.create("node").content(node).await.unwrap();

        match dota.remove(0).id.id {
            surrealdb::sql::Id::Number(_) => todo!(),
            surrealdb::sql::Id::String(id) => SurrealId(id),
            surrealdb::sql::Id::Array(_) => todo!(),
            surrealdb::sql::Id::Object(_) => todo!(),
            surrealdb::sql::Id::Generate(_) => todo!(),
        }
    }

    pub async fn refresh_client(&self, id: SurrealId) -> Result<()> {
        let result: Option<Record> = self
            .client
            .update(("node", id.0))
            .patch(PatchOp::replace("last_seen", Datetime::default()))
            .await
            .unwrap();

        match result {
            Some(_) => Ok(()),
            None => Err(BrokerError::ClientDoesNotExist),
        }
    }

    pub async fn pop_message(&self, id: SurrealId) -> Option<Message> {
        let message: Option<Message> = self
            .client
            .query(format!(
                "
                    BEGIN TRANSACTION; 
                    let $message = SELECT array::pop(messages) 
                    FROM {};
                    if $message != null {{
                        UPDATE {} SET 
                        messages = array::remove(messages, 
                        array::len(messages)-1);
                    }};
                    return $message[0][\"array::pop\"];
                    COMMIT TRANSACTION;
                    ",
                id.qualified(),
                id.qualified()
            ))
            .await
            .unwrap()
            .take(0)
            .unwrap();

        println!("{:?}", message);

        message
    }

    pub async fn client_exist(&self, id: SurrealId) -> bool {
        self.client
            .select::<Option<Record>>(("node", id.0))
            .await
            .unwrap()
            .is_some()
    }
}

#[cfg(test)]
mod tests {
    use std::{net::Ipv4Addr, str::FromStr};

    use super::*;

    async fn test_db() -> SurrealBroker {
        let mut db = SurrealBroker::new("192.168.0.103:9090", "test", "test", "root", "root").await;
        return db;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn client_insertion() {
        let mut db = test_db().await;

        let client = Node::new(Ipv4Addr::from_str("192.176.231.232").unwrap());

        db.add_node(client).await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn client_refresh() {
        let mut db = test_db().await;
        let past_node = Node::new(Ipv4Addr::from_str("192.176.231.232").unwrap());

        let id = db.add_node(past_node.clone()).await.unwrap();
        db.refresh_client(id.clone()).await;

        std::thread::sleep(std::time::Duration::new(2, 0));

        let future_node: Option<Node> = db.client.select(("node", id)).await.unwrap().unwrap();
        dbg!(future_node);
        dbg!(past_node);
        //println!("{}:{}", future_node.last_seen, past_node.last_seen)
    }
}
