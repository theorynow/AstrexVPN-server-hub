use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::features::{auth::UserAuthApiDoc, user::UserApiDoc};

pub fn create_swagger_ui() -> SwaggerUi {
    SwaggerUi::new("/docs")
        .url(
            "/api-docs/user_auth/openapi.json",
            UserAuthApiDoc::openapi(),
        )
        .url("/api-docs/user/openapi.json", UserApiDoc::openapi())
}
