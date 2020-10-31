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

        let tooltip_class = "tooltiptext left";

        let pixel = if_html!(mods.pixel =>
            <label class="tooltip">
                {"Pixel go"}
                <span class=tooltip_class>{"You place 2x2 blobs. Overlapping stones are ignored."}</span>
            </label>
        );

        let ponnuki = if_html!(let Some(p) = mods.ponnuki_is_points =>
            <label class="tooltip">
                {format!("Ponnuki is {} points", (p as f64) / 2.0)}
                <span class=tooltip_class>{"Ponnuki requires a capture and all diagonals must be empty or different color"}</span>
            </label>
        );

        let zen_go = if_html!(mods.zen_go.is_some() =>
            <label class="tooltip">
                {"Zen go"}
                <span class=tooltip_class>{"One extra player. You get a different color on every turn. There are no winners."}</span>
            </label>
        );

        let hidden_move = if_html!(let Some(r) = &mods.hidden_move =>
            <label class="tooltip">
                {format!("{} hidden moves", r.placement_count)}
                <span class=tooltip_class>{r#"
Each team places stones before the game starts.
The opponents and viewers can't see their stones.
Stones are revealed if they cause a capture or prevent a move from being made.
If two players pick the same point, neither one gets a stone there, but they still see a marker for it."#}</span>
            </label>
        );

        let one_color = if_html!(
            let Some(VisibilityMode::OneColor) = &mods.visibility_mode =>
            <label class="tooltip">
                {"One color go"}
                <span class=tooltip_class>{"Everyone sees the stones as same color. Confusion ensues."}</span>
            </label>
        );

        let no_history = if_html!(mods.no_history =>
            <label class="tooltip">
                {"No history"}
                <span class=tooltip_class>{"No one can browse the past moves during the game."}</span>
            </label>
        );

        let n_plus_one = if_html!(let Some(r) = &mods.n_plus_one =>
            <label class="tooltip">
                {format!("{}+1 go", r.length)}
                <span class=tooltip_class>{"You get an extra turn when you make a row of exactly N stones horizontally, vertically or diagonally."}</span>
            </label>
        );

        let tetris = if_html!(mods.tetris.is_some() =>
            <label class="tooltip">
                {"Tetris go"}
                <span class=tooltip_class>{"You can't play a group of exactly 4 stones. Diagonals don't form a group."}</span>
            </label>
        );

        let toroidal = if_html!(mods.toroidal.is_some() =>
            <label class="tooltip">
                {"Toroidal go"}
                <span class=tooltip_class>{"Opposing edges are connected. First line doesn't exist."}</span>
            </label>
        );

        let phantom = if_html!(mods.phantom.is_some() =>
            <label class="tooltip">
                {"Phantom go"}
                <span class=tooltip_class>{"All stones are invisinle when placed. They become visible when they affect the game (like hidden move go). Atari also reveals."}</span>
            </label>
        );

        let captures_give_points = if_html!(
            mods.captures_give_points.is_some() =>
            <label class="tooltip">
                {"Captures give points"}
                <span class=tooltip_class>{"Only the one to remove stones from the board gets the points. Promotes aggressive play. You only get points for removed stones, not dead stones in your territory."}</span>
            </label>
        );

        html! {
            <div style="padding: 10px;">
                <div>{pixel}</div>
                <div>{ponnuki}</div>
                <div>{zen_go}</div>
                <div>{hidden_move}</div>
                <div>{one_color}</div>
                <div>{no_history}</div>
                <div>{n_plus_one}</div>
                <div>{tetris}</div>
                <div>{toroidal}</div>
                <div>{phantom}</div>
                <div>{captures_give_points}</div>
            </div>
        }
    }
}
