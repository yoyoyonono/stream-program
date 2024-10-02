use brainrot::{twitch, youtube};
use chrono;
use futures_util::StreamExt;
use iced;
use iced::widget;
use queues::*;
use std::sync::{Arc, Mutex};
use std::{thread, time::Duration};
use tokio::{task, time};
use tts::Tts;

#[derive(Debug, Clone)]
struct ChatMessage {
    author: String,
    text: String,
}

#[derive(Default)]
struct AppState {
    value: i32,
}

impl AppState {
    fn update(&mut self, message: AppMessage) {
        match message {
            AppMessage::Increment => {
                self.value += 1;
            }
            AppMessage::Decrement => {
                self.value -= 1;
            }
        }
    }

    fn view(&self) -> widget::Column<AppMessage> {
        widget::column![
            widget::button("Increment").on_press(AppMessage::Increment),
            widget::text(self.value).size(50),
            widget::button("Decrement").on_press(AppMessage::Decrement)
        ]
        .padding(20)
        .align_x(iced::Center)
    }
}

#[derive(Debug, Clone, Copy)]
enum AppMessage {
    Increment,
    Decrement,
}

#[tokio::main]
async fn main() {
    let yt_live_id = "7WBJlWc9NX4";
    let twitch_id = "yoyoyonono";

    let mut tts_voice = Tts::new(tts::Backends::WinRt).unwrap();
    tts_voice.speak("hello, world", false).unwrap();
    thread::sleep(Duration::from_secs(2));

    let tts_queue = queue![];
    let tts_queue_arc = Arc::new(Mutex::new(tts_queue));
    let tts_queue_yt = Arc::clone(&tts_queue_arc);
    let tts_queue_twitch = Arc::clone(&tts_queue_arc);

    let program_start_time = chrono::Utc::now();

    let yt_handler = tokio::spawn(async move {
        let context = youtube::ChatContext::new_from_live(yt_live_id).await.unwrap();
        let mut stream = youtube::stream(&context).await.unwrap();
        println!("Youtube connected");
        while let Some(Ok(c)) = stream.next().await {
            if let youtube::Action::AddChatItem {
                item:
                    youtube::ChatItem::TextMessage {
                        message_renderer_base,
                        message,
                    },
                ..
            } = c
            {
                if message_renderer_base.timestamp_usec < program_start_time {
                    continue;
                }
                let text = ChatMessage {
                    author: message_renderer_base.author_name.unwrap().simple_text,
                    text: message
                        .unwrap()
                        .runs
                        .into_iter()
                        .map(|c| c.to_chat_string())
                        .collect::<String>(),
                };
                let mut lock = tts_queue_yt.lock().unwrap();
                lock.add(text).unwrap();
                drop(lock);
            }
        }
    });

    let twitch_handler = tokio::spawn(async move {
        let mut client = brainrot::TwitchChat::new(twitch_id, twitch::Anonymous)
            .await
            .unwrap();

        println!("Twitch connected");

        while let Some(message) = client.next().await.transpose().unwrap() {
            if let brainrot::TwitchChatEvent::Message { user, contents, .. } = message {
                let text = ChatMessage {
                    author: user.display_name,
                    text: contents.iter().map(|c| c.to_string()).collect::<String>(),
                };
                let mut lock = tts_queue_twitch.lock().unwrap();
                lock.add(text).unwrap();
                drop(lock);
            }
        }
    });

    thread::spawn(move || loop {
        let mut lock = tts_queue_arc.lock().unwrap();
        println!("{:?}", lock);
        if lock.size() > 0 {
            let text = lock.remove().unwrap();
            tts_voice
                .speak(format!("{}, {}", &text.author, &text.text), false)
                .unwrap();
        }
        drop(lock);
        thread::sleep(Duration::from_millis(100));
    });

    iced::run("A cool counter", AppState::update, AppState::view);
    yt_handler.await.unwrap();
    twitch_handler.await.unwrap();
}
