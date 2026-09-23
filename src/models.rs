use sea_orm::entity::prelude::*;
use serde::Serialize;

pub mod andromeda_users {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "andromeda_users")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub user_id: i64,
        pub username: String,
        pub password_hash: String,
        pub created_at: DateTimeWithTimeZone,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod andromeda_stars {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "andromeda_stars")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub star_id: i64,
        pub created_by: i64,
        pub star_name: String,
        pub star_catalog_id: String,
        pub distance_kpc: Decimal,
        pub velocity_kms: i32,
        pub star_status: String,
        pub star_description: String,
        pub image_url: String,
        pub video_url: String,
        pub created_at: DateTimeWithTimeZone,
        pub updated_at: DateTimeWithTimeZone,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod andromeda_likes {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "andromeda_likes")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub user_id: i64,
        #[sea_orm(primary_key, auto_increment = false)]
        pub star_id: i64,
        pub created_at: DateTimeWithTimeZone,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

#[derive(Debug, Serialize)]
pub struct StarTemplate {
    pub id: i64,
    pub name: String,
    pub catalog_id: String,
    pub distance_kpc: String,
    pub velocity_kms: i32,
    pub status: String,
    pub description: String,
    pub image_url: String,
    pub video_url: String,
    pub likes_count: i64,
}
