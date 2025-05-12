use brainrot::{twitch, youtube};
use chrono;
use eframe::egui;
use egui::{ecolor, Align2, Color32, FontId, Pos2, Rect};
use futures_util::StreamExt;
use queues::*;
use rand::Rng;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use std::{thread, time::Duration};
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
        viewport: egui::ViewportBuilder::default().with_inner_size([300.0, 720.0]),
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

    let danmaku_queue = queue![];
    let danmaku_queue_arc = Arc::new(Mutex::new(danmaku_queue));
    let danmaku_queue_yt = Arc::clone(&danmaku_queue_arc);
    let danmaku_queue_twitch = Arc::clone(&danmaku_queue_arc);
    let danmaku_queue_window = Arc::clone(&danmaku_queue_arc);

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

                let mut danmaku_lock = danmaku_queue_yt.lock().unwrap();
                danmaku_lock.add(message_text.clone()).unwrap();
                drop(danmaku_lock);
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

                let mut danmaku_lock = danmaku_queue_twitch.lock().unwrap();
                danmaku_lock.add(message_text.clone()).unwrap();
            }
        }
    });

    thread::spawn(move || loop {
        let mut queue_lock = tts_queue_arc.lock().unwrap();
        if queue_lock.size() > 0 {
            let text = queue_lock.remove().unwrap();
            let mut lock_text = chat_history_add.lock().unwrap();
            *lock_text = format!("\n{}: {}", &text.author, &text.text) + &lock_text;
            tts_voice
                .speak(format!("{}, {}", &text.author, &text.text), false)
                .unwrap();
        }
        drop(queue_lock);
        thread::sleep(Duration::from_millis(100));
    });

    eframe::run_native(
        "Chat window",
        options,
        Box::new(|cc| {
            Ok(Box::<AppState>::new(AppState::new(
                chat_history_remove,
                danmaku_queue_window,
            )))
        }),
    )
    .unwrap();

    yt_handler.await.unwrap();
    twitch_handler.await.unwrap();
}

struct AppState {
    chat: Arc<Mutex<String>>,
    current_danmaku: Vec<DanmakuMessage>,
    danmaku_queue: Arc<Mutex<Queue<String>>>,
    last_frame_time: Instant,
}

impl AppState {
    fn new(chat: Arc<Mutex<String>>, danmaku_queue: Arc<Mutex<Queue<String>>>) -> AppState {
        AppState {
            chat: chat,
            current_danmaku: vec![],
            danmaku_queue: danmaku_queue,
            last_frame_time: Instant::now(),
        }
    }
}

impl eframe::App for AppState {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut self.chat.lock().unwrap().clone()).desired_rows(40),
            );
        });
        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("viewport"),
            egui::ViewportBuilder::default()
                .with_title("Immediate viewport")
                .with_inner_size([1920.0, 1080.0])
                .with_resizable(false),
            |ctx, class| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let painter = ui.painter();

                    let mut danmaku_lock = self.danmaku_queue.lock().unwrap();
                    while let Ok(message) = danmaku_lock.remove() {
                        let mut y_pos = 0;
                        while self.current_danmaku.iter().any(|d| (d.position[1] == y_pos as f32 && d.position[0] + d.width + 20.0 > 1920.0)) {
                            y_pos += 30;
                        }

                        let color = ecolor::rgb_from_hsv([rand::rng().random_range(0.0..1.0), 0.5, 1.0].into());
                        let converted_color = Color32::from_rgb(
                                (color[0] * 255.0) as u8, 
                                (color[1] * 255.0) as u8, 
                                (color[2] * 255.0) as u8,
                            );
                        let text_width = painter.text(
                            Pos2::new(0.0, 0.0),
                            Align2::LEFT_TOP,
                            &message,
                            FontId::proportional(30.0),
                            converted_color,
                        ).width();
                        self.current_danmaku.push(DanmakuMessage {
                            content: message,
                            position: [1920.0, y_pos as f32],
                            color: converted_color,
                            width: text_width,
                        });
                    }

                    let move_distance = 100.0
                        * Instant::now()
                            .duration_since(self.last_frame_time)
                            .as_secs_f32();
                    self.last_frame_time = Instant::now();
                    for danmaku in &mut self.current_danmaku {
                        let pos = Pos2::new(danmaku.position[0] as f32, danmaku.position[1] as f32);
                        painter.text(
                            pos,
                            Align2::LEFT_TOP,
                            &danmaku.content,
                            FontId::proportional(30.0),
                            danmaku.color, 
                        );
                        danmaku.position[0] -= move_distance;
                    }

                    self.current_danmaku.retain(|danmaku| {
                        danmaku.position[0] > 0.0 - danmaku.width
                    });
                });
            },
        );
        ctx.request_repaint_after(Duration::from_millis(25));
    }
}

struct DanmakuMessage {
    content: String,
    position: [f32; 2],
    color: Color32,
    width: f32,
}
