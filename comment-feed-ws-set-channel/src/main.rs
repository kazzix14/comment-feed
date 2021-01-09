use dynomite::{Item, Attribute, FromAttributes};
use lambda::lambda;
use lambda_runtime as lambda;
use log::{error, info};
use rusoto_core::Region;
use rusoto_dynamodb::{DynamoDb, DynamoDbClient, PutItemInput, DeleteItemInput, QueryInput, GetItemInput};
use serde_derive::{Deserialize, Serialize};
use serde_json;
use simple_logger;
use rusoto_apigatewaymanagementapi::{ApiGatewayManagementApi, ApiGatewayManagementApiClient, PostToConnectionRequest};
use futures::stream::{futures_unordered::FuturesUnordered, StreamExt};

use lambda::error::HandlerError;

use std::{error::Error, collections::HashMap};

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
    body: String,
}

#[derive(Deserialize, Clone)]
struct RequestContext {
    #[serde(rename = "connectionId")]
    connection_id: String,
}

#[derive(Deserialize, Clone)]
struct CustomBody {
    channel: String,
    new_channel: String,
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
    let body = serde_json::from_str::<CustomBody>(&e.body).expect("malformed data");
    let channel = body.channel;
    let new_channel = body.new_channel;


    // we can not update key
    // so delete the item and add it again

    let client = DynamoDbClient::new(Region::ApNortheast1);

    let mut rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {

        let mut item = WSConnection {
            channel: channel,
            connection_id: connection_id,
        };

        let input = DeleteItemInput {
            table_name: "websocket.comment-feed".to_string(),
            key: item.key(),
            ..DeleteItemInput::default()
        };

        match client.delete_item(input).await {
            Ok(output) => {
                info!("delete!");
            }
            Err(error) => {
                error!("Error: {:?}", error);
            }
        }

        item.channel = new_channel;

        let input = PutItemInput {
            table_name: "websocket.comment-feed".to_string(),
            item: item.into(),
            ..PutItemInput::default()
        };

        match client.put_item(input).await {
            Ok(output) => {
                info!("ok!! put dynamodb");
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