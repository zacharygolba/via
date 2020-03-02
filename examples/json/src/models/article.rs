use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct Article {
    pub id: Uuid,
    pub body: String,
    pub title: String,
}

#[derive(Debug, Deserialize)]
pub struct ChangeSet {
    pub body: Option<String>,
    pub title: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct NewArticle {
    pub body: String,
    pub title: String,
}
