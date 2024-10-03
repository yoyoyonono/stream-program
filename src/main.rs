use brainrot::{twitch, youtube};
use chrono;
use eframe::egui;
use futures_util::StreamExt;
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

#[tokio::main]
async fn main() {
    let yt_live_id = "7WBJlWc9NX4";
    let twitch_id = "yoyoyonono";

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1280.0, 720.0]),
        ..Default::default()
    };

    let mut tts_voice = Tts::new(tts::Backends::WinRt).unwrap();
    tts_voice.speak("hello, world", false).unwrap();
    thread::sleep(Duration::from_secs(2));

    let chat_history = String::new();
    let chat_history_arc = Arc::new(Mutex::new(chat_history));
    let chat_history_add = Arc::clone(&chat_history_arc);
    let chat_history_remove = Arc::clone(&chat_history_arc);

    let tts_queue = queue![];
    let tts_queue_arc = Arc::new(Mutex::new(tts_queue));
    let tts_queue_yt = Arc::clone(&tts_queue_arc);
    let tts_queue_twitch = Arc::clone(&tts_queue_arc);

    let program_start_time = chrono::Utc::now();

    let yt_handler = tokio::spawn(async move {
        let context = youtube::ChatContext::new_from_live(yt_live_id)
            .await
            .unwrap();
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
                let author = message_renderer_base.author_name.unwrap().simple_text;
                let message_text = message
                    .unwrap()
                    .runs
                    .into_iter()
                    .map(|c| c.to_chat_string())
                    .collect::<String>();
                let message_1 = ChatMessage {
                    author: author.clone(),
                    text: message_text.clone(),
                };
                let mut lock = tts_queue_yt.lock().unwrap();
                lock.add(message_1).unwrap();
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
                let author = user.display_name.clone();
                let message_text = contents.iter().map(|c| c.to_string()).collect::<String>();
                let message_1 = ChatMessage {
                    author: author.clone(),
                    text: message_text.clone(),
                };
                let mut lock = tts_queue_twitch.lock().unwrap();
                lock.add(message_1).unwrap();
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
            let mut lock_text = chat_history_add.lock().unwrap();
            *lock_text += &format!("{}: {}\n", &text.author, &text.text);
        }
        drop(lock);
        thread::sleep(Duration::from_millis(100));
    });

    let _ = eframe::run_simple_native("My egui App", options, move |ctx, _frame| {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut chat_history_remove.lock().unwrap().clone())
                    .desired_rows(40),
            );
        });
    });

    yt_handler.await.unwrap();
    twitch_handler.await.unwrap();
}
