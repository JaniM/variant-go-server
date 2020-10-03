use yew::prelude::*;
use yewtil::NeqAssign;

use crate::if_html;
use shared::game::{GameModifier, VisibilityMode};

pub struct ModeList {
    _link: ComponentLink<Self>,
    props: Props,
}

pub enum Msg {}

#[derive(Properties, PartialEq, Clone)]
pub struct Props {
    pub mods: GameModifier,
}

impl Component for ModeList {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        ModeList { _link: link, props }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {}
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        self.props.neq_assign(props)
    }

    fn view(&self) -> Html {
        let mods = &self.props.mods;

        let pixel = if_html!(mods.pixel => <li>{"Pixel go"}</li>);

        let ponnuki = if_html!(let Some(p) = mods.ponnuki_is_points =>
            <li>{format!("Ponnuki is {} points", (p as f64) / 2.0)}</li>
        );

        let zen_go = if_html!(mods.zen_go.is_some() =>
            <li>{"Zen go"}</li>
        );

        let hidden_move = if_html!(let Some(r) = &mods.hidden_move =>
            <li>{format!("{} hidden moves", r.placement_count)}</li>
        );

        let one_color = if_html!(
            let Some(VisibilityMode::OneColor) = &mods.visibility_mode =>
            <li>{"One color go"}</li>
        );

        let no_history = if_html!(mods.no_history => <li>{"No history"}</li>);

        let n_plus_one = if_html!(let Some(r) = &mods.n_plus_one =>
            <li>{format!("{}+1 go", r.length)}</li>
        );

        let tetris = if_html!(mods.tetris.is_some() =>
            <li>{"Tetris go"}</li>
        );

        let captures_give_points = if_html!(
            mods.captures_give_points.is_some() =>
            <li>{"Captures give points"}</li>
        );

        html! {
            <ul>
                {pixel}
                {ponnuki}
                {zen_go}
                {hidden_move}
                {one_color}
                {no_history}
                {n_plus_one}
                {tetris}
                {captures_give_points}
            </ul>
        }
    }
}
