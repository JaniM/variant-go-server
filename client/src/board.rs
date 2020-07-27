use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::HtmlCanvasElement;
use web_sys::CanvasRenderingContext2d as Canvas2d;
use yew::services::{RenderService, Task};
use yew::{html, Component, ComponentLink, Html, NodeRef, ShouldRender};

pub struct Board {
    canvas: Option<HtmlCanvasElement>,
    canvas2d: Option<Canvas2d>,
    link: ComponentLink<Self>,
    node_ref: NodeRef,
    render_loop: Option<Box<dyn Task>>,
    mouse_pos: (f64, f64)
}

pub enum Msg {
    Render(f64),
    MouseMove((f64, f64)),
}

impl Component for Board {
    type Message = Msg;
    type Properties = ();

    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        Board {
            canvas: None,
            canvas2d: None,
            link,
            node_ref: NodeRef::default(),
            render_loop: None,
            mouse_pos: (0.0, 0.0),
        }
    }

    fn rendered(&mut self, first_render: bool) {
        // Once rendered, store references for the canvas and GL context. These can be used for
        // resizing the rendering area when the window or canvas element are resized, as well as
        // for making GL calls.

        let canvas = self.node_ref.cast::<HtmlCanvasElement>().unwrap();

        let canvas2d: Canvas2d = canvas
            .get_context("2d")
            .unwrap()
            .unwrap()
            .dyn_into()
            .unwrap();

        {
            let mouse_move = self.link.callback(Msg::MouseMove);
            let closure = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
                mouse_move.emit((event.offset_x() as f64, event.offset_y() as f64));
            }) as Box<dyn FnMut(_)>);
            canvas.add_event_listener_with_callback("mousemove", closure.as_ref().unchecked_ref()).unwrap();
            closure.forget();
        }

        self.canvas = Some(canvas);
        self.canvas2d = Some(canvas2d);

        // In a more complex use-case, there will be additional WebGL initialization that should be
        // done here, such as enabling or disabling depth testing, depth functions, face
        // culling etc.

        if first_render {
            // The callback to request animation frame is passed a time value which can be used for
            // rendering motion independent of the framerate which may vary.
            let render_frame = self.link.callback(Msg::Render);
            let handle = RenderService::request_animation_frame(render_frame);

            // A reference to the handle must be stored, otherwise it is dropped and the render won't
            // occur.
            self.render_loop = Some(Box::new(handle));
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::Render(timestamp) => {
                //self.render_gl(timestamp).unwrap();
            },
            Msg::MouseMove(p) => {
                self.mouse_pos = p;
                self.render_gl(0.0).unwrap();
            }
        }
        false
    }

    fn view(&self) -> Html {
        html! {
            <canvas ref={self.node_ref.clone()} width=800 height=800 />
        }
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        false
    }
}

impl Board {
    fn render_gl(&mut self, timestamp: f64) -> Result<(), JsValue> {
        let context = self.canvas2d.as_ref().expect("Canvas Context not initialized!");
        let canvas = self.canvas.as_ref().expect("Canvas not initialized!");

        context.clear_rect(0.0, 0.0, canvas.width().into(), canvas.height().into());

        context.set_fill_style(&JsValue::from_str("#d38139"));
        context.fill_rect(0.0, 0.0, canvas.width().into(), canvas.height().into());


        context.set_fill_style(&JsValue::from_str("#000000"));

        let size = 100i32;
        // create shape of radius 'size' around center point (size, size)
        context.begin_path();
        context.arc(
            self.mouse_pos.0,
            self.mouse_pos.1,
            size.into(),
            0.0,
            2.0 * std::f64::consts::PI,
        )?;
        context.fill();
        context.stroke();

        let render_frame = self.link.callback(Msg::Render);
        let handle = RenderService::request_animation_frame(render_frame);

        // A reference to the new handle must be retained for the next render to run.
        self.render_loop = Some(Box::new(handle));

        Ok(())
    }
}
