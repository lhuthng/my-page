use crate::{
    application::commands,
    domain::{entities, errors},
};

#[async_trait::async_trait]
pub trait UserService {
    async fn me(
        &self,
        cmd: commands::user::MeCommand,
    ) -> Result<entities::user::Me, errors::user::UserError>;
}
