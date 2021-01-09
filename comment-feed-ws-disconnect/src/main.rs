use dynomite::Item;
use lambda::lambda;
use lambda_runtime as lambda;
use log::{error, info};
use rusoto_core::Region;
use rusoto_dynamodb::{Delete, DeleteItemInput, DynamoDb, DynamoDbClient};
use serde_derive::{Deserialize, Serialize};
use simple_logger;

use lambda::error::HandlerError;

use std::{collections::HashMap, error::Error};

#[derive(Item)]
struct WSConnection {
    #[dynomite(sort_key)]
    #[dynomite(rename = "connectionId")]
    connection_id: String,
    #[dynomite(partition_key)]
    channel: String,
}

#[derive(Deserialize, Clone)]
struct CustomEvent {
    #[serde(rename = "requestContext")]
    request_context: RequestContext,
}

#[derive(Deserialize, Clone)]
struct RequestContext {
    #[serde(rename = "connectionId")]
    connection_id: String,
}

#[derive(Serialize, Clone)]
struct CustomOutput {
    #[serde(rename = "statusCode")]
    status_code: u32,
}

fn main() -> Result<(), Box<dyn Error>> {
    simple_logger::init_with_level(log::Level::Info)?;
    lambda!(my_handler);

    Ok(())
}

fn my_handler(e: CustomEvent, c: lambda::Context) -> Result<CustomOutput, HandlerError> {
    let connection_id = e.request_context.connection_id;

    info!("disconnection. id: {}", connection_id);

    let item = WSConnection {
        connection_id: connection_id,
        channel: "test".to_string(),
    };

    let input = DeleteItemInput {
        table_name: "websocket.comment-feed".to_string(),
        key: item.key(),
        ..DeleteItemInput::default()
    };

    let client = DynamoDbClient::new(Region::ApNortheast1);

    let mut rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {
        match client.delete_item(input).await {
            Ok(output) => {
                info!("deleted connection on dynamodb");
                Ok(CustomOutput {
                    status_code: 200,
                })
            }
            Err(error) => {
                error!("Error: {:?}", error);
                Err(c.new_error(&format!("Error: {:?}", error)))
            }
        }
    })
}
