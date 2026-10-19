use utoipa::{
    Modify, OpenApi,
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
};

use crate::handlers::{api, api_dto::*, users_api};

#[derive(OpenApi)]
#[openapi(
    info(title = "Andromeda Stars API", version = "0.4.0",
         description = "Лабораторная 4. JWT + blacklist Redis. Регистрация, вход, список и лента доступны гостю. Выполните POST /api/auth/login, скопируйте access_token из ответа, нажмите Authorize и вставьте токен без префикса Bearer. Защищённые методы принимают Authorization: Bearer <JWT>. Токен действует один час; logout отзывает его через blacklist Redis до истечения exp."),
    paths(api::api_get_stars, api::api_get_feed, api::api_get_draft,
          api::api_create_star, api::api_publish_star, api::api_delete_star, api::api_like_star,
          users_api::api_register_user, users_api::api_login, users_api::api_logout),
    components(schemas(ApiStar, ApiStarsList, ApiCreateStarRequest, ApiPublishRequest,
                       ApiLikeRequest, ApiRegisterRequest, ApiAuthRequest, ApiUser, ApiAuthResponse)),
    modifiers(&BearerSecurity),
    tags((name = "Пользователи"), (name = "Звёзды"), (name = "Лайки"))
)]
pub struct ApiDoc;

struct BearerSecurity;

impl Modify for BearerSecurity {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearerAuth",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build(),
                ),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swagger_distinguishes_public_read_and_protected_methods() {
        let document = ApiDoc::openapi();
        let paths = &document.paths.paths;
        assert!(document.security.is_none());
        let schemes = &document.components.as_ref().unwrap().security_schemes;
        assert!(schemes.contains_key("bearerAuth"));
        assert!(!schemes.contains_key("sessionCookie"));
        for path in ["/api/andromeda-stars", "/api/andromeda-stars/feed"] {
            assert!(paths[path].get.as_ref().unwrap().security.is_none());
        }
        for operation in [
            paths["/api/andromeda-stars"].post.as_ref().unwrap(),
            paths["/api/andromeda-stars/draft"].get.as_ref().unwrap(),
            paths["/api/andromeda-stars/{id}/publish"]
                .put
                .as_ref()
                .unwrap(),
            paths["/api/andromeda-stars/{id}"].delete.as_ref().unwrap(),
            paths["/api/andromeda-stars/{id}/like"]
                .post
                .as_ref()
                .unwrap(),
        ] {
            assert!(
                operation
                    .security
                    .as_ref()
                    .is_some_and(|items| !items.is_empty())
            );
        }
    }
}
