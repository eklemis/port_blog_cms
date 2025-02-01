use crate::modules::cv::domain::entities::{
    CVInfo, Education, Experience, HighlightedProject, Project, Screenshot, SingleProjectDetails,
};
use actix_web::{get, web, HttpResponse, Responder};
use serde::Deserialize;

#[get("/api/cv")]
pub async fn get_cv() -> impl Responder {
    // For now, return hardcoded data or fetch from DB.
    let response = CVInfo {
        bio: "I'm Jane Doe, a software engineer specializing in Rust and Svelte...".to_string(),
        photo_url: "https://example.com/images/jane.jpg".to_string(),
        educations: vec![Education {
            degree: "B.Sc. in Computer Science".to_string(),
            institution: "Tech University".to_string(),
            graduation_year: 2020,
        }],
        experiences: vec![Experience {
            company: "Acme Corp".to_string(),
            position: "Software Engineer".to_string(),
            start_date: "2020-05-01".to_string(),
            end_date: None,
            description: "Developing backend services using Rust...".to_string(),
        }],
        highlighted_projects: vec![
            HighlightedProject {
                id: "1f9c588a-2dce-4475-b878-964711d40688".to_string(),
                title: "Project Alpha".to_string(),
                slug: "project-alpha".to_string(),
                short_description: "High-level overview of the project...".to_string(),
            },
            HighlightedProject {
                id: "a2b3c4d5-e6f7-8901-2345-6789abcdef12".to_string(),
                title: "Project Beta".to_string(),
                slug: "project-beta".to_string(),
                short_description: "Another top project...".to_string(),
            },
        ],
    };

    HttpResponse::Ok().json(response)
}

#[get("/api/projects")]
pub async fn list_projects(query: web::Query<ProjectQueryParams>) -> impl Responder {
    // parse query params
    let _page = query.page;
    let _featured = query.featured;
    let _search = &query.search;

    // For now, return hardcoded or mock data
    let projects = vec![
        Project {
            id: "1f9c588a-2dce-4475-b878-964711d40688".to_string(),
            title: "Project Alpha".to_string(),
            slug: "project-alpha".to_string(),
            description: "Detailed description of Project Alpha...".to_string(),
            tech_stack: vec!["Rust".to_string(), "React".to_string()],
            featured: true,
            created_at: "2024-12-01T10:30:00Z".to_string(),
            updated_at: "2024-12-05T12:45:00Z".to_string(),
        },
        Project {
            id: "a2b3c4d5-e6f7-8901-2345-6789abcdef12".to_string(),
            title: "Project Beta".to_string(),
            slug: "project-beta".to_string(),
            description: "Another exciting project...".to_string(),
            tech_stack: vec!["SvelteKit".to_string(), "Node.js".to_string()],
            featured: false,
            created_at: "2025-01-10T08:15:00Z".to_string(),
            updated_at: "2025-01-12T09:22:00Z".to_string(),
        },
    ];

    // (Optional) Filter/paginate these results based on query parameters.

    HttpResponse::Ok().json(projects)
}

#[derive(Deserialize)]
pub struct ProjectQueryParams {
    pub page: Option<u32>,
    pub featured: Option<bool>,
    pub search: Option<String>,
}

#[get("/api/projects/{slug}")]
pub async fn get_project_by_slug(path: web::Path<String>) -> impl Responder {
    let slug = path.into_inner();
    // For now, we’ll just check if the slug matches a known project
    if slug == "project-alpha" {
        let project = SingleProjectDetails {
            id: "1f9c588a-2dce-4475-b878-964711d40688".to_string(),
            title: "Project Alpha".to_string(),
            slug: "project-alpha".to_string(),
            description: "Detailed description of Project Alpha...".to_string(),
            tech_stack: vec!["Rust".to_string(), "React".to_string()],
            featured: true,
            screenshots: vec![
                Screenshot {
                    url: "https://example.com/images/alpha1.png".to_string(),
                    order: 1,
                    featured: true,
                },
                Screenshot {
                    url: "https://example.com/images/alpha2.png".to_string(),
                    order: 2,
                    featured: false,
                },
            ],
            repo_url: "https://github.com/janedoe/project-alpha".to_string(),
            live_demo_url: "https://project-alpha.example.com".to_string(),
            created_at: "2024-12-01T10:30:00Z".to_string(),
            updated_at: "2024-12-05T12:45:00Z".to_string(),
        };
        HttpResponse::Ok().json(project)
    } else if slug == "project-beta" {
        // Return another sample, or eventually fetch from DB
        // ...
        HttpResponse::Ok().json(serde_json::json!({ "success": "A Single Project" }))
    } else {
        HttpResponse::NotFound().json(serde_json::json!({ "error": "Project not found" }))
    }
}
