use game_loop::game_loop;

// For convenience, game_loop re-exports winit so you don't need to add it as
// an additional dependency of your crate.

use game_loop::winit::event::{Event, WindowEvent};
use game_loop::winit::event_loop::EventLoop;
use game_loop::winit::window::Window;

struct UserEvent;

fn main() {
    let event_loop = EventLoop::<UserEvent>::with_user_event().build().unwrap();
    let game = Game::new();

    game_loop(
        event_loop,
        game,
        240,
        0.1,
        |_, e| e.create_window(Default::default()).unwrap(),
        |g| {
            g.game.your_update_function();
        },
        |g| {
            if let Some(window) = g.window.get() {
                g.game.your_render_function(window);
            }
        },
        |g, event| {
            if !g.game.your_window_handler(event) {
                g.exit();
            }
        },
    )
    .unwrap();
}

#[derive(Default)]
struct Game {
    num_updates: u32,
    num_renders: u32,
}

impl Game {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn your_update_function(&mut self) {
        self.num_updates += 1;
    }

    pub fn your_render_function(&mut self, window: &Window) {
        self.num_renders += 1;
        window.set_title(&format!(
            "num_updates: {}, num_renders: {}",
            self.num_updates, self.num_renders
        ));
    }

    // A very simple handler that returns false when CloseRequested is detected.
    pub fn your_window_handler(&self, event: &Event<UserEvent>) -> bool {
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    return false;
                }
                _ => {}
            },
            _ => {}
        }

        true
    }
}
