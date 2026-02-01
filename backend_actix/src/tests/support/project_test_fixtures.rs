use crate::modules::project::application::ports::outgoing::project_query::{
    PageResult, ProjectCardView,
};

pub fn empty_page_result() -> PageResult<ProjectCardView> {
    PageResult {
        items: vec![],
        page: 1,
        per_page: 10,
        total: 0,
    }
}
