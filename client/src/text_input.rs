use yew::prelude::*;

pub struct TextInput {
    link: ComponentLink<Self>,
    text: String,
    props: TextInputProperties,
}

pub enum TextInputMsg {
    SetText(String),
    Submit,
    None,
}

#[derive(Properties, Clone, PartialEq)]
pub struct TextInputProperties {
    pub value: String,
    pub onsubmit: Callback<String>,
}

impl Component for TextInput {
    type Message = TextInputMsg;
    type Properties = TextInputProperties;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        TextInput {
            link,
            text: props.value.clone(),
            props,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            TextInputMsg::SetText(text) => self.text = text,
            TextInputMsg::Submit => self.props.onsubmit.emit(self.text.clone()),
            TextInputMsg::None => return false,
        }
        true
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        if self.props.value != props.value || self.text.is_empty() {
            self.text = props.value.clone();
        }
        if self.props != props {
            self.props = props;
        }
        true
    }

    fn view(&self) -> Html {
        html! {
            <input
                type="text"
                value=&self.text
                oninput=self.link.callback(|e: InputData| TextInputMsg::SetText(e.value))
                onblur=self.link.callback(|_| TextInputMsg::Submit)
                onkeydown=self.link.callback(move |e: KeyboardEvent| {
                    e.stop_propagation();
                    if e.key() == "Enter" { TextInputMsg::Submit } else { TextInputMsg::None }
                })
                />
        }
    }
}
