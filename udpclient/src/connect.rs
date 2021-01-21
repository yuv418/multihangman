use std::collections::HashMap;
use crate::hangmanclient::HangmanClient;
use std::sync::Arc;
use crate::opening::OpeningScene;
use raylib::prelude::*;
use crate::raylibscene::RaylibScene;
use crate::resources::Resources;
use std::fs;
use serde::{Serialize, Deserialize};
use crate::textbox::TextBox;
use raylib::ease::*;
use std::thread;
use std::time::Duration;
use hangmanstructs::Configurable;

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Config {
    recent_ips: Vec<String>,

    #[serde(skip)]
    pub file_name: String
}

impl Configurable<Config> for Config {
    fn set_file_name(&mut self, file_name: String) {
        self.file_name = file_name;
    }
    fn file_name(&self) -> String {
        self.file_name.clone()
    }
}

impl Config {

    pub fn add(&mut self, ip: &str) {
        self.recent_ips.push(ip.to_string());

        let toml = toml::to_string(&self).unwrap();
        fs::write(&self.file_name, &toml);
    }
}

pub struct ConnectScene {
    config: Config,
    selected_ip: usize, // vec index
    give_next_scene: bool,
    add_ip: bool, // draw add box instead of other scene
    add_ip_buffer: String,
    client: Option<Arc<HangmanClient>>,
    failed_connect: bool
}

impl ConnectScene {
    pub fn new() -> Self {

        Self {
            config: Config::from_file("ClientConfiguration.toml".to_string()),
            selected_ip: 0,
            give_next_scene: false,
            add_ip: false,
            add_ip_buffer: String::new(),
            failed_connect: false,
            client: None
        }
    }
}

impl RaylibScene for ConnectScene {
    fn draw_raylib(&mut self, rl: &mut RaylibHandle, thread: &RaylibThread, res: &Resources) {
        {
            let mut d = rl.begin_drawing(thread);
            d.clear_background(raylib::core::color::Color::WHITE);
            RaylibScene::draw_text_res(&mut d, &res, "Connect", 40, 30, 24, raylib::core::color::Color::BLACK); // title text
            if !self.add_ip {

                let mut y = 150;
                for (i, ip) in self.config.recent_ips.iter().enumerate() {
                    let mut rect_color = Color::BLACK;
                    if self.selected_ip == i {
                        rect_color = Color::ORANGE;
                    }
                    RaylibScene::draw_text_box(&mut d, &res, &ip, 300, y, 24, Color::BLACK, rect_color);
                    y += 40;
                }

                let add_ip_color = if self.selected_ip == self.config.recent_ips.len() {
                    Color::ORANGE
                }
                else {
                    Color::BLACK
                };
                RaylibScene::draw_text_box(&mut d, &res, "Add IP", 300, y, 24, Color::BLACK, add_ip_color);

                if self.failed_connect {
                    RaylibScene::draw_text_box(&mut d, &res, "Couldn't connect to that server.", 300, 40, 24, Color::RED, Color::RED); // error box
                }
            }
            else {
                RaylibScene::draw_text_res(&mut d, &res, "Add IP", 290, 210, 24, Color::BLACK);
                RaylibScene::draw_input_box(&mut d, &res, &self.add_ip_buffer, 300, 250, 24);
            }
        }

        if self.failed_connect {
            thread::sleep(Duration::from_millis(500));
            self.failed_connect = false;
        }
    }
    fn handle_raylib(&mut self, rl: &mut RaylibHandle) {
        if let Some(key) = rl.get_key_pressed() {
            match key {
                KeyboardKey::KEY_UP => {
                    if self.selected_ip > 0 {
                        self.selected_ip -= 1;
                    }
                },
                KeyboardKey::KEY_DOWN => {
                    if self.selected_ip <= self.config.recent_ips.len() { // We allow user to go "equal" to the length of the array. The "equal" case is when the user wants to add something
                        self.selected_ip += 1;
                    }
                },
                KeyboardKey::KEY_ENTER => {
                    if self.add_ip {
                        // Add the IP to the config
                        self.config.add(&self.add_ip_buffer);
                        self.add_ip = false;
                    }
                    else if self.selected_ip == self.config.recent_ips.len() || self.config.recent_ips.len() == 0 { // Allow the user to add another IP
                        self.add_ip = true;
                        self.selected_ip = 0;
                    }
                    else { // Create the client here too
                        let ip = self.config.recent_ips[self.selected_ip].clone();
                        let client = HangmanClient::new(ip);

                        match client {
                            Some(c) => {
                                self.client = Some(c);
                                self.give_next_scene = true;
                            },
                            None => {
                                self.failed_connect = true
                            }
                        };

                    }
                },
                _ => {
                    let mut unicode = key as i32 as u8 as char;
                    if unicode == ';' {
                        unicode = ':'; // hack because the unicode to char thing doesn't work so well (you can't type shift and expect it to work, hah)
                    }
                    self.add_ip_buffer = TextBox::process_input_str(&mut self.add_ip_buffer, unicode);
                },
            }
        }
    }
    fn next_scene(&self) -> Box<RaylibScene> {
        let client = self.client.as_ref().unwrap();
        Box::new(OpeningScene::new(Arc::clone(client)))
    }
    fn has_next_scene(&self) -> bool {
        self.give_next_scene
    }
}