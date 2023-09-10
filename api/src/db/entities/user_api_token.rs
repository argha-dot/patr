//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.2

use sea_orm::entity::prelude::*;

use super::sea_orm_active_enums::UserLoginType;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "user_api_token")]
pub struct Model {
	#[sea_orm(primary_key, auto_increment = false)]
	pub token_id: Uuid,
	#[sea_orm(column_type = "Text")]
	pub name: String,
	pub user_id: Uuid,
	#[sea_orm(column_type = "Text")]
	pub token_hash: String,
	pub token_nbf: Option<TimeDateTimeWithTimeZone>,
	pub token_exp: Option<TimeDateTimeWithTimeZone>,
	pub allowed_ips: Option<Vec<String>>,
	pub created: TimeDateTimeWithTimeZone,
	pub revoked: Option<TimeDateTimeWithTimeZone>,
	pub login_type: Option<UserLoginType>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
	#[sea_orm(
		belongs_to = "super::user::Entity",
		from = "Column::UserId",
		to = "super::user::Column::Id",
		on_update = "NoAction",
		on_delete = "NoAction"
	)]
	User,
	#[sea_orm(has_many = "super::user_api_token_workspace_permission_type::Entity")]
	UserApiTokenWorkspacePermissionType,
	#[sea_orm(has_many = "super::user_api_token_workspace_super_admin::Entity")]
	UserApiTokenWorkspaceSuperAdmin,
	#[sea_orm(
		belongs_to = "super::user_login::Entity",
		from = "Column::TokenId",
		to = "super::user_login::Column::UserId",
		on_update = "NoAction",
		on_delete = "NoAction"
	)]
	UserLogin,
}

impl Related<super::user::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::User.def()
	}
}

impl Related<super::user_api_token_workspace_permission_type::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::UserApiTokenWorkspacePermissionType.def()
	}
}

impl Related<super::user_api_token_workspace_super_admin::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::UserApiTokenWorkspaceSuperAdmin.def()
	}
}

impl Related<super::user_login::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::UserLogin.def()
	}
}

impl ActiveModelBehavior for ActiveModel {}
