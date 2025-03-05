use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table) // ✅ Fix: Reference the table explicitly
                    .add_column(
                        ColumnDef::new(Users::IsVerified)
                            .boolean()
                            .not_null()
                            .default(false), // Default: Unverified
                    )
                    .add_column(
                        ColumnDef::new(Users::IsDeleted)
                            .boolean()
                            .not_null()
                            .default(false), // Default: Not deleted
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table) // ✅ Fix: Reference the table explicitly
                    .drop_column(Users::IsVerified)
                    .drop_column(Users::IsDeleted)
                    .to_owned(),
            )
            .await
    }
}

/// ✅ Define the `Users` table with an `Iden` struct
#[derive(Iden)]
enum Users {
    Table,
    IsVerified,
    IsDeleted,
}
