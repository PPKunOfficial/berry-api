use ::entity::user;
use sea_orm::*;

pub struct Mutation;

impl Mutation {
    pub async fn create_user(
        &self,
        db: &DbConn,
        form_data: user::ActiveModel,
    ) -> Result<user::ActiveModel, DbErr> {
        form_data.save(db).await
    }
    pub async fn delete_user(&self, db: &DbConn, id: i32) -> Result<(), DbErr> {
        user::ActiveModel {
            id: Set(id),
            ..Default::default()
        }
        .delete(db)
        .await?;
        Ok(())
    }
}
