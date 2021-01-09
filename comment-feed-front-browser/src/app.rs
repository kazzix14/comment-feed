use chrono::{DateTime, Local, NaiveDateTime, NaiveTime};
use futures::{prelude::*, stream::StreamFuture, StreamExt};
use js_sys::JsString;
use log::*;
use serde_derive::{Deserialize, Serialize};
use serde_json::json;
use std::{
    io::BufReader,
    pin::Pin,
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc,
    },
    task::Poll,
    time::SystemTime,
};
use strum::IntoEnumIterator;
use tokio;
use wasm_bindgen::{prelude::*, JsCast, UnwrapThrowExt};
use wasm_bindgen_futures::spawn_local;
use web_sys::MessageEvent;
use ws_stream_wasm::*;
use yew::format::Json;
use yew::prelude::*;
use yew::services::storage::{Area, StorageService};

const KEY: &str = "yew.todomvc.self";

pub struct App {
    link: ComponentLink<Self>,
    storage: StorageService,
    state: State,
    ws_meta: Option<Arc<WsMeta>>,
    ws_stream: Option<Arc<WsStream>>,
}

#[derive(Serialize, Clone)]
struct SetChannelBody {
    action: String,
    channel: String,
    new_channel: String,
}

#[derive(Serialize, Clone)]
struct SendMessageBody {
    action: String,
    channel: String,
    message: String,
}

#[derive(Serialize, Deserialize)]
pub struct State {
    channel: String,
    channel_input: String,
    connected: bool,
    comments: Vec<Comment>,
    comment_input: String,
}

#[derive(Serialize, Deserialize)]
struct Comment {
    body: String,
    time: DateTime<Local>,
}

pub enum Message {
    UpdateCommentField(String),
    PushComment,
    UpdateChannelField(String),
    Connected(WsMeta, WsStream),
    Disconnected,
    SetChannel,
    CommentReceived(Comment),
    Nope,
}

impl Component for App {
    type Message = Message;
    type Properties = ();

    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        let storage = StorageService::new(Area::Local).unwrap();
        let comments = {
            if let Json(Ok(restored_comments)) = storage.restore(KEY) {
                restored_comments
            } else {
                Vec::<Comment>::new()
            }
        };

        let state = State {
            channel: "".into(),
            channel_input: "".into(),
            connected: false,
            comments,
            comment_input: "".into(),
        };

        info!("try connect!");
        let cloned_link = link.clone();
        spawn_local(async move {
            let (ws_meta, ws_stream) = WsMeta::connect(
                "wss://7ht6ij8i09.execute-api.ap-northeast-1.amazonaws.com/production",
                None,
            )
            .await
            .expect_throw("failed to connect");

            if ws_meta.ready_state() == WsState::Open {
                cloned_link.send_message(Message::Connected(ws_meta, ws_stream));
            } else {
                unreachable!();
            }
        });

        App {
            link,
            storage,
            state,
            ws_meta: None,
            ws_stream: None,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Message::UpdateCommentField(body) => {
                self.state.comment_input = body;
            }
            Message::PushComment => {
                if !self.state.comment_input.is_empty() {
                    info!("pushing comment");
                    self.ws_stream
                        .as_ref()
                        .expect("connection naiyo!")
                        .wrapped()
                        .send_with_str(
                            &serde_json::to_string(&SendMessageBody {
                                action: "sendmessage".to_string(),
                                channel: self.state.channel.clone(),
                                message: self.state.comment_input.clone(),
                            })
                            .unwrap(),
                        )
                        .expect("failed to send");

                    self.state.comment_input = "".to_string();
                    return true;
                }
            }
            Message::UpdateChannelField(body) => {
                self.state.channel_input = body;
            }
            Message::Connected(ws_meta, ws_stream) => {
                self.ws_meta = Some(Arc::new(ws_meta));
                self.ws_stream = Some(Arc::new(ws_stream));

                let link = self.link.clone();

                let callback = Closure::wrap(Box::new(move |e: MessageEvent| {
                    info!("callbacked");
                    if let Ok(message) = e.data().dyn_into::<JsString>() {
                        info!("message event, received Text: {:?}", message);

                        let comment = Comment {
                            body: message.into(),
                            time: Local::now(),
                        };
                        link.send_message(Message::CommentReceived(comment))
                    }
                }) as Box<dyn FnMut(MessageEvent)>);

                self.ws_stream
                    .as_ref()
                    .expect("connection naiyo!")
                    .wrapped()
                    .add_event_listener_with_callback("message", callback.as_ref().unchecked_ref())
                    .unwrap();

                callback.forget();

                self.state.connected = true;
                info!("connected!");
                return true;
            }
            Message::Disconnected => {
                self.ws_meta = None;
                self.ws_stream = None;
                self.state.connected = false;
                info!("disconnected!");
                return true;
            }
            Message::CommentReceived(comment) => {
                self.state.comments.push(comment);
                return true;
            }
            Message::SetChannel => {
                info!("pushing channel");
                self.ws_stream
                    .as_ref()
                    .expect("connection naiyo!")
                    .wrapped()
                    .send_with_str(
                        &serde_json::to_string(&SetChannelBody {
                            action: "setchannel".to_string(),
                            channel: self.state.channel.clone(),
                            new_channel: self.state.channel_input.clone(),
                        })
                        .unwrap(),
                    )
                    .expect("failed to send");

                self.state.channel = self.state.channel_input.clone();
                return true;
            }
            Message::Nope => (),
        }
        false
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        false
    }

    fn destroy(&mut self) {
        info!("try disconnect!");
        let link = self.link.clone();
        let ws_meta = self.ws_meta.as_ref().expect("not connected");
        let ws_meta = Arc::clone(ws_meta);
        spawn_local(async move {
            ws_meta.close().await.expect_throw("failed to close");
            if ws_meta.ready_state() == WsState::Closed {
                link.send_message(Message::Disconnected);
            } else {
                unreachable!();
            }
        });
    }

    fn view(&self) -> Html {
        info!("rendered!");
        html! {
            <div>
                <div class="ui masthead">
                    <div class="ui container">
                        <h1 class="ui header">
                            { "Comment Feed" }
                        </h1>
                    </div>
                </div>
                <div class="ui divider"/>
                <div class="main ui container">
                    <div class="ui vertical segment">
                        <div class="ui masthead basic vertical segment">
                            { self.view_channel_connection() }
                        </div>

                        <div class="ui container">
                            <ul class="ui comments">
                                {
                                    for self.state.comments.iter().map( |comment| self.view_comment(comment) )
                                }
                            </ul>
                        </div>

                        <div class="ui vertical segment">
                            { self.view_comment_input() }
                        </div>
                    </div>
                </div>
            </div>
        }
    }
}

impl App {
    fn view_comment(&self, comment: &Comment) -> Html {
        html! {
            <div class="comment">
                <div class="content">
                    <div class="metadata">
                        { &comment.time }
                    </div>
                    <div class="text">
                        { &comment.body }
                    </div>
                </div>
            </div>
        }
    }

    fn view_comment_input(&self) -> Html {
        html! {
            <div class="ui fluid action input">
                <input
                    type="text"
                    value=&self.state.comment_input
                    oninput=self.link.callback(move |e: InputData| Message::UpdateCommentField(e.value))
                    onkeypress=self.link.callback(move |e: KeyboardEvent| {
                        match e.key().as_ref() {
                            "Enter" => Message::PushComment,
                            _ => Message::Nope,
                        }
                    })
                />
                <button class="ui button" onclick=self.link.callback(move |_| Message::PushComment)>
                    { "送信" }
                </button>
            </div>
        }
    }

    fn view_channel_connection(&self) -> Html {
        html! {
            <div class="ui fluid action input">
                <input
                    type="text"
                    value=&self.state.channel_input
                    oninput=self.link.callback(move |e: InputData| Message::UpdateChannelField(e.value))
                />
                <button class="ui button" onclick=self.link.callback(move |_| Message::SetChannel)>
                    { "接続" }
                </button>
            </div>
        }
    }
}
