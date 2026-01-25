use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create the table
        manager
            .create_table(
                Table::create()
                    .table(Users::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Users::Id).uuid().not_null().primary_key())
                    .col(
                        ColumnDef::new(Users::Username)
                            .string_len(50) // Add length constraint
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(Users::Email)
                            .string_len(255) // Add length constraint
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(Users::PasswordHash)
                            .string_len(255)
                            .not_null(),
                    )
                    .col(ColumnDef::new(Users::FullName).string_len(100).not_null())
                    .col(
                        ColumnDef::new(Users::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Users::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Users::IsVerified)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Users::IsDeleted)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .to_owned(),
            )
            .await?;

        // ============================================
        // PERFORMANCE INDEXES
        // ============================================

        // 1. Partial unique index on email for active users only
        //    This allows soft-deleted users to "free up" their email
        //    and speeds up email lookups for active users
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE UNIQUE INDEX idx_users_email_active
                ON users (email)
                WHERE is_deleted = false;
                "#,
            )
            .await?;

        // 2. Partial unique index on username for active users only
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE UNIQUE INDEX idx_users_username_active
                ON users (username)
                WHERE is_deleted = false;
                "#,
            )
            .await?;

        // 3. Index for soft-delete queries (find deleted users by email)
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE INDEX idx_users_email_deleted
                ON users (email, is_deleted);
                "#,
            )
            .await?;

        // 4. Index on is_verified for filtering unverified users
        //    Partial index - only index unverified users (smaller index)
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE INDEX idx_users_unverified
                ON users (id)
                WHERE is_verified = false AND is_deleted = false;
                "#,
            )
            .await?;

        // 5. Index on created_at for sorting/pagination
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE INDEX idx_users_created_at
                ON users (created_at DESC);
                "#,
            )
            .await?;

        // 6. Composite index for common query patterns
        //    (listing active, verified users)
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE INDEX idx_users_active_verified
                ON users (is_deleted, is_verified, created_at DESC)
                WHERE is_deleted = false;
                "#,
            )
            .await?;

        // ============================================
        // TRIGGER FOR updated_at
        // ============================================

        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE OR REPLACE FUNCTION update_updated_at_column()
                RETURNS TRIGGER AS $$
                BEGIN
                    NEW.updated_at = CURRENT_TIMESTAMP;
                    RETURN NEW;
                END;
                $$ language 'plpgsql';
                "#,
            )
            .await?;

        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE TRIGGER update_users_updated_at
                BEFORE UPDATE ON users
                FOR EACH ROW
                EXECUTE FUNCTION update_updated_at_column();
                "#,
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop trigger and function
        manager
            .get_connection()
            .execute_unprepared("DROP TRIGGER IF EXISTS update_users_updated_at ON users")
            .await?;

        manager
            .get_connection()
            .execute_unprepared("DROP FUNCTION IF EXISTS update_updated_at_column")
            .await?;

        // Drop all indexes (they'll be dropped with the table, but explicit is clearer)
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                DROP INDEX IF EXISTS idx_users_email_active;
                DROP INDEX IF EXISTS idx_users_username_active;
                DROP INDEX IF EXISTS idx_users_email_deleted;
                DROP INDEX IF EXISTS idx_users_unverified;
                DROP INDEX IF EXISTS idx_users_created_at;
                DROP INDEX IF EXISTS idx_users_active_verified;
                "#,
            )
            .await?;

        manager
            .drop_table(Table::drop().table(Users::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
    Username,
    Email,
    PasswordHash,
    FullName,
    CreatedAt,
    UpdatedAt,
    IsVerified,
    IsDeleted,
}
