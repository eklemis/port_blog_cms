use crate::project::adapter::outgoing::sea_orm_entity::projects;
use crate::topic::adapter::outgoing::sea_orm_entity::topics;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Composite primary key on `(project_id, topic_id)`, matching the migration.
///
/// This previously declared an `id` primary key that the table does not have —
/// `project_topics` holds only project_id, topic_id and created_at. The one
/// existing query happens to use `.select_only()` and never asked for `id`, so
/// it worked by luck; any `Entity::find()` returning a full Model would have
/// failed at runtime against a column that does not exist.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "project_topics")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false, column_type = "Uuid")]
    pub project_id: Uuid,

    #[sea_orm(primary_key, auto_increment = false, column_type = "Uuid")]
    pub topic_id: Uuid,

    #[sea_orm(column_type = "TimestampWithTimeZone")]
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::projects::Entity",
        from = "Column::ProjectId",
        to = "super::projects::Column::Id",
        on_delete = "Cascade",
        on_update = "Cascade"
    )]
    Projects,

    #[sea_orm(
        belongs_to = "crate::topic::adapter::outgoing::sea_orm_entity::topics::Entity",
        from = "Column::TopicId",
        to = "crate::topic::adapter::outgoing::sea_orm_entity::topics::Column::Id",
        on_delete = "Cascade",
        on_update = "Cascade"
    )]
    Topics,
}

impl Related<projects::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Projects.def()
    }
}

impl Related<topics::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Topics.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::Iterable;

    /// The entity's columns must match the table exactly.
    ///
    /// This entity previously declared an `id` primary key that
    /// `project_topics` does not have. Nothing caught it because the only
    /// query used `.select_only()` and never asked for `id`; a full
    /// `Entity::find()` would have generated SQL naming a nonexistent column.
    /// Comparing the column set catches that class of drift without needing a
    /// live database.
    #[test]
    fn entity_columns_match_the_table() {
        let columns: Vec<String> = Column::iter().map(|c| c.to_string()).collect();

        assert_eq!(
            columns,
            vec!["project_id", "topic_id", "created_at"],
            "entity columns drifted from the project_topics table"
        );
    }

    /// The join row is identified by the pair, not a surrogate key, which is
    /// what makes attaching the same topic twice a no-op at the database.
    #[test]
    fn primary_key_is_the_composite_pair() {
        let pk: Vec<String> = PrimaryKey::iter().map(|c| c.to_string()).collect();
        assert_eq!(pk, vec!["project_id", "topic_id"]);
    }
}
