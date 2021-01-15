use crate::Scene;
use crate::hangmanclient::HangmanClient;
use sfml::{graphics::*, window::*};
use unicode_segmentation::UnicodeSegmentation;
use std::sync::Arc;
use hangmanstructs::*;
use std::thread;
use std::time::Duration;
use unicode_categories::UnicodeCategories;
use crate::newgamewizard::NewGameWizardScene;
use crate::opening::OpeningScene;
use crate::Scenes;
use crate::textbox::TextBox;
use crate::resources::Resources;

pub struct GameScene<'a> {
    // UI elements
    attempts_word_box: TextBox<'a>,
    guess_chars: Vec<TextBox<'a>>,
    wrong_guess_box: TextBox<'a>,
    client: Arc<HangmanClient<'a>>,
    next_scene: Scenes,
    bgcolor: Color,
}

impl<'a> GameScene<'a> {

    pub fn new(client: Arc<HangmanClient<'a>>) -> GameScene<'a> {

        let mut attempts_word_box = TextBox::new("Attempts: ", 24, (550., 40.));
        let mut wrong_guess_box = TextBox::new("Guesses:", 24, (550., 100.));

        GameScene {
            client,
            attempts_word_box,
            next_scene: Scenes::None,
            guess_chars: vec![],
            bgcolor: Color::WHITE,
            wrong_guess_box
        }


    }

    fn update_values(&mut self, window: &mut RenderWindow, resources: &Resources) {
        {

            let game = self.client.game.lock().unwrap();
            let game = game.as_ref().expect("Game doesn't exist yet in the game scene!");

            // Render guesses → they'll always be updated. (ONLY multiguess [guess together] is implemented for now)

            if self.guess_chars.len() != game.word.len() { // ONLY create all the guess chars if the two things are mismatched. Otherwise we'll just keep adding to the boxes and create a memory leak.
                self.guess_chars.clear();

                let mut xoffset = 100.;
                for i in 0..game.word.len() {
                    let mut guess_letter = TextBox::new(" ", 40, (xoffset, 280.));
                    self.guess_chars.push(guess_letter);

                    xoffset+=50.;
                }
            }

            // Implement filling the guess_chars with the respective guesses  { may put this in a separate function for multiguess/fastestguess }

            let mut attempts_remaining = game.max_guesses;
            let mut wrong_vec = vec![];

            for guess in &game.guesses {

                // Go through the guesses, find the string's position in the other string, if part of string, then get the respective guess_chars set string to guess, and rerender the word box
                let guess_indices: Vec<_> = game.word.match_indices(&guess.guess).collect();
                if guess_indices.is_empty() {
                    // Guess was wrong
                    attempts_remaining -= 1;
                    wrong_vec.push(&guess.guess);
                }

                for (guess_position, _) in guess_indices {
                    let mut guess_char = &mut self.guess_chars[guess_position];
                    guess_char.text.set_string(guess.guess.as_str());

                }
            }

            self.attempts_word_box.text.set_string(format!("Attempts: {}", attempts_remaining).as_str());

            let mut wrong_string = String::from("Wrong Guesses:\n");
            let mut rows_left = 7; // 8 letters per line
            for guess in wrong_vec {
                wrong_string += format!("{} ", guess).as_str();
                if rows_left == 0 {
                    wrong_string += "\n";
                    rows_left = 7;
                }
                rows_left -=1 ;

            }
            self.wrong_guess_box.text.set_string(wrong_string.as_str());
        } // So that the game is unlocked

        // Process client event queue (eg. if another person guesses incorrectly, flash the window red)

        // Copy the client event queue here in order to satisfy ownership rules.
        // The variable event_queue gets borrowed immutably, so we cannot handle events on client event queue.
        // Instead, we'll transfer all the events to the vec, thus clearing the event queue, and using the vec to consume the events.
        let mut local_event_queue = vec![];
        {
            let mut event_queue = self.client.event_queue.lock().unwrap();
            for _ in 0..event_queue.len() {

                local_event_queue.push(event_queue.pop_back().unwrap());
            }
        }

        for event in local_event_queue {
            self.handle_hangman_event(&event, window, resources, false); // Don't consume
            self.client.handle_event(event); // Consume
        }

    }

    fn flash_red(&mut self, window: &mut RenderWindow, resources: &Resources, from_self: bool) {
        self.bgcolor = Color::RED;
        self.draw(window, resources);

        if from_self {
            thread::sleep(Duration::from_secs(1)); // From us, penalize
        }
        else {
            println!("Not penalizing you");
            thread::sleep(Duration::from_millis(100)) // From someone else, don't wait so long/penalize them.
        }

        self.bgcolor = Color::WHITE;
    }

    fn handle_hangman_event(&mut self, event: &HangmanEvent, window: &mut RenderWindow, resources: &Resources, from_self: bool) {
        let mut wrong_guess = false;
        { // Have to use a scope since game gets borrowed here, so when we call flash_red the program doesn't know whether or not we're modifying game or something. Could be called by making bgcolor a RefCell.
            let game = self.client.game.lock().unwrap();
            let game = game.as_ref().expect("Game doesn't exist yet in the game scene!");

            match event {
                HangmanEvent::Sync(id, guess) => {
                    if let None = game.word.find(&guess.guess) { // Repeated code is not good. (from udpserver.rs)
                        wrong_guess = true;

                        // Update wrong guesses (TODO this should not be here, but rather in the update values method)

                    }
                },
                HangmanEvent::GameWon(user) => {
                    println!("{:?} won the game!", user);
                    self.next_scene = Scenes::OpeningScene;
                },
                HangmanEvent::GameDraw => {
                    println!("Draw game!");
                    self.next_scene = Scenes::OpeningScene;
                },
                _ => {}
            }

        }

        if wrong_guess {
            self.flash_red(window, resources, from_self); // Don't wait, this was someone else's failure
        }
    }
}

impl<'a> Scene<'a> for GameScene<'a> {

    fn reset_next_scene(&mut self) {
        let client = Arc::clone(&self.client);
        *self = GameScene::new(client);
    }

    fn next_scene(&self) -> Scenes {
        self.next_scene.clone()
    }

    fn draw(&mut self, window: &mut RenderWindow, resources: &Resources) {
        self.update_values(window, resources);

        window.clear(self.bgcolor);
        // window.draw(&self.attempts_remaining);

        window.draw(&self.attempts_word_box);
        window.draw(&self.wrong_guess_box);

        for guess_char in &self.guess_chars {
            window.draw(guess_char)
        }

        window.display();
    }


    fn handle_event(&mut self, event: Event, window: &mut RenderWindow, resources: &Resources) { // TODO consider moving flash_red to draw somehow

        match event {

            Event::TextEntered { unicode, .. } => if unicode.is_letter_lowercase() || unicode.is_letter_uppercase() {
                println!("Guess! {:?}", unicode.to_string());
                let (sync, sync_response) = self.client.sync(unicode.to_string());
               
                self.handle_hangman_event(&sync, window, resources, true);
            },
            _ => {}
        }
    }

}
