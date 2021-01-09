use dynomite::Item;
use lambda::lambda;
use lambda_runtime as lambda;
use log::{error, info};
use rusoto_core::Region;
use rusoto_dynamodb::{DynamoDb, DynamoDbClient, PutItemInput};
use serde_derive::{Deserialize, Serialize};
use simple_logger;

use lambda::error::HandlerError;

use std::error::Error;

#[derive(Item)]
struct WSConnection {
    #[dynomite(partition_key)]
    channel: String,
    #[dynomite(sort_key)]
    #[dynomite(rename = "connectionId")]
    connection_id: String,
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

    let item = WSConnection {
        connection_id: connection_id,
        channel: "test".to_string(),
    };

    let input = PutItemInput {
        table_name: "websocket.comment-feed".to_string(),
        item: item.into(),
        ..PutItemInput::default()
    };

    let client = DynamoDbClient::new(Region::ApNortheast1);

    let mut rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {
        match client.put_item(input).await {
            Ok(output) => {
                info!("created connection on dynamodb");
                Ok(CustomOutput {
                    status_code: 200
                })
            }
            Err(error) => {
                error!("Error: {:?}", error);
                Err(c.new_error(&format!("Error: {:?}", error)))
            }
        }
    })
}
