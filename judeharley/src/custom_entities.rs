pub mod songs {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "songs")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub file_path: String,
        pub title: String,
        pub artist: String,
        pub album: String,
        pub played: i32,
        pub requested: i32,
        #[sea_orm(ignore)]
        pub tsvector: Option<String>,
        #[sea_orm(column_type = "Double")]
        pub duration: f64,
        #[sea_orm(unique)]
        pub file_hash: String,
        pub bitrate: i32,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}
