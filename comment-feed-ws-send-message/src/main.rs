use dynomite::{Item, Attribute, FromAttributes};
use lambda::lambda;
use lambda_runtime as lambda;
use log::{error, info};
use rusoto_core::Region;
use rusoto_dynamodb::{DynamoDb, DynamoDbClient, UpdateItemInput, QueryInput, GetItemInput};
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
    #[serde(rename = "domainName")]
    domain_name: String,
    stage: String,
}

#[derive(Deserialize, Clone)]
struct CustomBody {
    action: String,
    message: String,
    channel: String,
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
    let domain_name = e.request_context.domain_name;
    let stage = e.request_context.stage;
    let endpoint_url = format!("https://{}/{}", domain_name, stage);

    let body = serde_json::from_str::<CustomBody>(&e.body).expect("malformed data");
    let message = body.message;
    let channel = body.channel;

    let client = DynamoDbClient::new(Region::ApNortheast1);
    let mut rt = tokio::runtime::Runtime::new().unwrap();


    rt.block_on(async {
        let mut expression_attribute_values = HashMap::<String, rusoto_dynamodb::AttributeValue>::new();
        expression_attribute_values.insert(":channel".to_string(), channel.into_attr());

        // broadcast
        let input = QueryInput {
            table_name: "websocket.comment-feed".to_string(),
            key_condition_expression: Some("channel = :channel".to_string()),
            expression_attribute_values: Some(expression_attribute_values),
            ..QueryInput::default()
        };

        match client.query(input).await {
            Ok(output) => {
                info!("queried! {:?}", output);
                if let Some(items) = output.items {

                    let api_gateway_client = ApiGatewayManagementApiClient::new(Region::Custom{ name: "ap-northeast-1".to_string(), endpoint: endpoint_url});

                    let mut post_task = items.iter().map( |item| {
                        let item = WSConnection::from_attrs(item.clone()).expect("failed");
                        api_gateway_client.post_to_connection(PostToConnectionRequest {
                            connection_id: item.connection_id,
                            data: message.clone().into(),
                        })
                    }).collect::<FuturesUnordered<_>>();

                    while !post_task.is_empty() {
                        let (result, fut) = post_task.into_future().await;
                        post_task = fut;
                        result.expect("failed to send message").expect("I don't know");
                    }

                    info!("sent!");
                }
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
