use lambda_http::{
    http::header::CONTENT_TYPE, run, service_fn, Body,
    Error, Request, Response,
};
use serde::Serialize;
use serde_json::json;
use sqlx::mysql::MySqlPoolOptions;
use std::env;

#[derive(Debug, sqlx::FromRow, Serialize)]
struct PokemonHp {
    name: String,
    hp: u16,
}

async fn function_handler(
    event: Request,
) -> Result<Response<Body>, Error> {
    let path = event.uri().path();
    let requested_pokemon = path.split("/").last();

    match requested_pokemon {
        None => todo!("this is a hard error, return 500"),
        Some("") => {
            let error_message =
                serde_json::to_string(&json!({
                    "error": "searched for empty pokemon"
                }))?;
            let resp = Response::builder()
                .status(400)
                .header(CONTENT_TYPE, "application/json")
                .body(Body::Text(error_message))?;
            Ok(resp)
        }
        Some(pokemon_name) => {
            let database_url = env::var("DATABASE_URL")?;

            let pool = MySqlPoolOptions::new()
                .max_connections(5)
                .connect(&database_url)
                .await?;

            let result = sqlx::query_as!(
                PokemonHp,
                r#"SELECT name, hp from pokemon where slug = ?"#,
                pokemon_name
            )
            .fetch_one(&pool)
            .await?;

            let pokemon = serde_json::to_string(&result)?;
            let resp = Response::builder()
                .status(200)
                .header(CONTENT_TYPE, "application/json")
                .body(Body::Text(pokemon))?;
            Ok(resp)
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    run(service_fn(function_handler)).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn accepts_apigw_request() {
        let input = include_str!("apigw-request.json");

        let request = lambda_http::request::from_str(input)
            .expect("failed to create request");

        let response = function_handler(request)
            .await
            .expect("failed to handle request");

        assert_eq!(
            response.body(),
            &Body::Text(
                "{\"name\":\"Bulbasaur\",\"hp\":45}"
                    .to_string()
            )
        );
    }

    #[tokio::test]
    async fn handles_empty_pokemon() {
        let input =
            include_str!("empty-pokemon-request.json");

        let request = lambda_http::request::from_str(input)
            .expect("failed to create request");

        let response = function_handler(request)
            .await
            .expect("failed to handle request");

        assert_eq!(
            response.body(),
            &Body::Text(
                "{\"error\":\"searched for empty pokemon\"}"
                    .to_string()
            )
        );
    }
}
