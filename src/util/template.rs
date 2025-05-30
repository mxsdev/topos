use std::borrow::Cow;

use handlebars::Renderable;

pub trait Templater<C> {
    type RenderError;

    fn new(context: C) -> Self;
    fn render_template<'a>(&self, template: &str) -> Result<Cow<'a, str>, Self::RenderError>;
}

pub struct HandlebarsTemplater<'a, C: serde::Serialize> {
    handlebars: handlebars::Handlebars<'a>,
    pub context: C,
}

impl<'a, C: serde::Serialize> Templater<C> for HandlebarsTemplater<'a, C> {
    type RenderError = handlebars::RenderError;

    fn new(context: C) -> Self {
        let mut templater = handlebars::Handlebars::default();

        templater.register_helper("times", Box::new(TimesHelper));
        templater.register_escape_fn(handlebars::no_escape);

        return Self {
            handlebars: templater,
            context,
        };
    }

    fn render_template<'b>(&self, template: &str) -> Result<Cow<'b, str>, Self::RenderError> {
        self.handlebars
            .render_template(template, &self.context)
            .map(Into::into)
    }
}

struct TimesHelper;
impl handlebars::HelperDef for TimesHelper {
    fn call_inner<'reg: 'rc, 'rc>(
        &self,
        helper: &handlebars::Helper,
        r: &handlebars::Handlebars,
        ctx: &handlebars::Context,
        rc: &mut handlebars::RenderContext,
    ) -> Result<handlebars::ScopedJson<'rc>, handlebars::RenderError> {
        let num_times = helper
            .param(0)
            .ok_or_else(|| {
                handlebars::RenderError::new(format!(
                    "{} Helper: Expected block parameter",
                    helper.name()
                ))
            })?
            .value()
            .as_i64()
            .ok_or_else(|| {
                handlebars::RenderError::new(format!(
                    "{} Helper: Expected integer parameter",
                    helper.name()
                ))
            })?;

        let block_template = helper.template().ok_or_else(|| {
            handlebars::RenderError::new(format!(
                "{} Helper: Expected block parameter",
                helper.name()
            ))
        })?;

        let mut out = handlebars::StringOutput::default();

        let mut new_ctx = ctx.clone();

        for i in 0..num_times {
            let mut new_render_ctx = rc.clone();

            let new_ctx_data = new_ctx.data_mut().as_object_mut().ok_or_else(|| {
                handlebars::RenderError::new(format!(
                    "{} Helper: Expected object-like context",
                    helper.name()
                ))
            })?;

            new_ctx_data.insert("index".to_string(), handlebars::JsonValue::Number(i.into()));

            block_template.render(r, &new_ctx, &mut new_render_ctx, &mut out)?;
        }

        Ok(handlebars::ScopedJson::Derived(
            handlebars::JsonValue::String(
                out.into_string().map_err(handlebars::RenderError::from)?,
            ),
        ))
    }
}
