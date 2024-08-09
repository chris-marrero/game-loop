use crate::*;

pub use helper::*;

#[cfg(all(
    not(target_arch = "wasm32"),
    not(feature = "winit"),
    not(feature = "tao")
))]
mod helper {
    use super::*;

    pub fn game_loop<G, U, R>(
        game: G,
        updates_per_second: u32,
        max_frame_time: f64,
        mut update: U,
        mut render: R,
    ) -> GameLoop<G, Time, ()>
    where
        U: FnMut(&mut GameLoop<G, Time, ()>),
        R: FnMut(&mut GameLoop<G, Time, ()>),
    {
        let mut game_loop = GameLoop::new(game, updates_per_second, max_frame_time, ());

        while game_loop.next_frame(&mut update, &mut render) {}

        game_loop
    }
}

#[cfg(all(target_arch = "wasm32", not(feature = "winit")))]
mod helper {
    use super::*;
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::JsCast;
    use web_sys::window;

    pub fn game_loop<G, U, R>(
        game: G,
        updates_per_second: u32,
        max_frame_time: f64,
        update: U,
        render: R,
    ) where
        G: 'static,
        U: FnMut(&mut GameLoop<G, Time, ()>) + 'static,
        R: FnMut(&mut GameLoop<G, Time, ()>) + 'static,
    {
        let game_loop = GameLoop::new(game, updates_per_second, max_frame_time, ());

        animation_frame(game_loop, update, render);
    }

    fn animation_frame<G, U, R>(mut g: GameLoop<G, Time, ()>, mut update: U, mut render: R)
    where
        G: 'static,
        U: FnMut(&mut GameLoop<G, Time, ()>) + 'static,
        R: FnMut(&mut GameLoop<G, Time, ()>) + 'static,
    {
        if g.next_frame(&mut update, &mut render) {
            let next_frame = move || animation_frame(g, update, render);
            let closure = Closure::once_into_js(next_frame);
            let js_func = closure.as_ref().unchecked_ref();

            window().unwrap().request_animation_frame(js_func).unwrap();
        }
    }
}

#[cfg(feature = "winit")]
mod helper {
    use super::*;
    use std::cell::OnceCell;
    use std::sync::Arc;

    pub use winit;
    use winit::{
        application::ApplicationHandler,
        error::EventLoopError,
        event::{DeviceEvent, DeviceId, Event, WindowEvent},
        event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
        window::{Window, WindowId},
    };

    type Loop<G> = GameLoop<G, Time, Arc<OnceCell<Window>>>;

    pub trait UpdateFn<G>: FnMut(&mut Loop<G>) + 'static {}
    impl<T, G> UpdateFn<G> for T where T: FnMut(&mut Loop<G>) + 'static {}

    pub trait RenderFn<G>: FnMut(&mut Loop<G>) + 'static {}
    impl<T, G> RenderFn<G> for T where T: FnMut(&mut Loop<G>) + 'static {}

    pub trait HandlerFn<G>: FnMut(&mut Loop<G>, &Event<()>) + 'static {}
    impl<T, G> HandlerFn<G> for T where T: FnMut(&mut Loop<G>, &Event<()>) + 'static {}

    pub trait InitFn<G>: FnMut(&mut Loop<G>, &ActiveEventLoop) -> Window + 'static {}
    impl<T, G> InitFn<G> for T where T: FnMut(&mut Loop<G>, &ActiveEventLoop) -> Window + 'static {}

    struct App<G, U: UpdateFn<G>, R: RenderFn<G>, H: HandlerFn<G>, I: InitFn<G>> {
        game_loop: Loop<G>,
        init: I,
        update: U,
        render: R,
        handler: H,
    }

    impl<G, U: UpdateFn<G>, R: RenderFn<G>, H: HandlerFn<G>, I: InitFn<G>, T: 'static>
        ApplicationHandler<T> for App<G, U, R, H, I>
    {
        fn resumed(&mut self, event_loop: &ActiveEventLoop) {
            let window = self.game_loop.window.clone();
            if window.get().is_none() {
                window
                    .set((self.init)(&mut self.game_loop, event_loop))
                    .unwrap();
            }
        }

        fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
            if let Some(w) = self.game_loop.window.get() {
                w.request_redraw();
            }
        }

        fn window_event(
            &mut self,
            event_loop: &ActiveEventLoop,
            window_id: WindowId,
            event: WindowEvent,
        ) {
            match event {
                WindowEvent::Occluded(occluded) => self.game_loop.window_occluded = occluded,
                WindowEvent::RedrawRequested => {
                    if !self
                        .game_loop
                        .next_frame(&mut self.update, &mut self.render)
                    {
                        event_loop.exit();
                    }
                }
                _ => {
                    (self.handler)(
                        &mut self.game_loop,
                        &Event::WindowEvent { window_id, event },
                    );
                }
            }
        }

        fn device_event(
            &mut self,
            _event_loop: &ActiveEventLoop,
            device_id: DeviceId,
            event: DeviceEvent,
        ) {
            (self.handler)(
                &mut self.game_loop,
                &Event::DeviceEvent { device_id, event },
            );
        }
    }

    pub fn game_loop<G: 'static, U: UpdateFn<G>, R: RenderFn<G>, H: HandlerFn<G>, I: InitFn<G>>(
        event_loop: EventLoop<()>,
        game: G,
        updates_per_second: u32,
        max_frame_time: f64,
        init: I,
        update: U,
        render: R,
        handler: H,
    ) -> Result<(), EventLoopError>
    where
        G: 'static,
    {
        let mut app = App {
            game_loop: GameLoop::new(game, updates_per_second, max_frame_time, Default::default()),
            init,
            update,
            render,
            handler,
        };

        event_loop.set_control_flow(ControlFlow::Poll);
        event_loop.run_app(&mut app)
    }
}

#[cfg(feature = "tao")]
mod helper {
    use super::*;
    use std::sync::Arc;
    use tao::event::Event;
    use tao::event_loop::{ControlFlow, EventLoop};
    use tao::window::Window;

    pub use tao;

    pub fn game_loop<G, U, R, H, T>(
        event_loop: EventLoop<T>,
        window: Arc<Window>,
        game: G,
        updates_per_second: u32,
        max_frame_time: f64,
        mut update: U,
        mut render: R,
        mut handler: H,
    ) -> !
    where
        G: 'static,
        U: FnMut(&mut GameLoop<G, Time, Arc<Window>>) + 'static,
        R: FnMut(&mut GameLoop<G, Time, Arc<Window>>) + 'static,
        H: FnMut(&mut GameLoop<G, Time, Arc<Window>>, &Event<'_, T>) + 'static,
        T: 'static,
    {
        let mut game_loop = GameLoop::new(game, updates_per_second, max_frame_time, window);

        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Poll;

            // Forward events to existing handlers.
            handler(&mut game_loop, &event);

            match event {
                Event::RedrawRequested(_) => {
                    if !game_loop.next_frame(&mut update, &mut render) {
                        *control_flow = ControlFlow::Exit;
                    }
                }
                Event::MainEventsCleared => {
                    game_loop.window.request_redraw();
                }
                _ => {}
            }
        })
    }
}
