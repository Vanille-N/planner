use std::collections::{HashMap, HashSet};

use crate::lib::{
    parse::ast::*,
    error::{Result, Error},
    entry::{Entry, fields::*},
    date::Date,
};

pub mod models {
    pub use super::{
        Template,
        TagTemplate,
        AmountTemplate,
        AmountTemplateItem,
    };
}

#[derive(Debug)]
pub struct Instance<'i> {
    pub label: &'i str,
    pub pos: Vec<Arg<'i>>,
    pub named: Vec<(&'i str, Arg<'i>)>,
}

#[derive(Debug, Clone, Copy)]
pub enum Arg<'i> {
    Amount(Amount),
    Tag(&'i str),
}
#[derive(Debug)]
pub struct Template<'i> {
    pub positional: Vec<&'i str>,
    pub named: Vec<(&'i str, Arg<'i>)>,
    pub value: AmountTemplate<'i>,
    pub cat: Category,
    pub span: Span,
    pub tag: TagTemplate<'i>,
}

#[derive(Debug)]
pub struct TagTemplate<'i>(pub Vec<TagTemplateItem<'i>>);

#[derive(Debug)]
pub enum TagTemplateItem<'i> {
    Day,
    Month,
    Year,
    Date,
    Weekday,
    Raw(&'i str),
    Arg(&'i str),
}

#[derive(Debug)]
pub struct AmountTemplate<'i> {
    pub sign: bool,
    pub sum: Vec<AmountTemplateItem<'i>>,
}

#[derive(Debug)]
pub enum AmountTemplateItem<'i> {
    Cst(Amount),
    Arg(&'i str),
}

pub fn instanciate(ast: Ast<'_>) -> Result<Vec<(Date, Entry)>> {
    let mut entries = Vec::new();
    let mut templates = HashMap::new();
    for item in ast {
        match item {
            AstItem::Entry(date, entry) => entries.push((date, entry)),
            AstItem::Template(name, loc, body) => {
                templates.insert(name.to_string(), (loc, body));
            }
            AstItem::Instance(date, loc, instance) => {
                let inst = instanciate_item(instance, date, loc, &templates)?;
                entries.push((date, inst));
            }
        }
    }
    Ok(entries)
}

fn instanciate_item(
    instance: Instance<'_>,
    date: Date,
    loc: pest::Span,
    templates: &HashMap<String, (pest::Span, Template)>,
) -> Result<Entry> {
    let templ = match templates.get(instance.label) {
        None => {
            return Err(Error::new("Undeclared template")
                .with_span(&loc, format!("attempt to instanciate {}", instance.label))
                .with_message("Maybe a typo ?"));
        }
        Some(t) => t,
    };
    let args = build_arguments(&instance, &loc, templ)?;
    perform_replacements(&instance.label, &loc, templ, args, date)
}

fn build_arguments<'i>(
    instance: &Instance<'i>,
    loc: &pest::Span,
    template: &(pest::Span<'i>, Template<'i>),
) -> Result<HashMap<String, Arg<'i>>> {
    // check number of positional arguments
    if instance.pos.len() != template.1.positional.len() {
        let err = Error::new("Argcount mismatch")
            .with_span(loc, format!("instanciation provides {} arguments", instance.pos.len()))
            .with_span(&template.0, format!("template expects {} arguments", template.1.positional.len()))
            .with_message("Fix the count mismatch");
        return Err(err);
    }
    let mut args = HashMap::new();
    for (name, val) in template.1.positional.iter().zip(instance.pos.iter()) {
        args.insert(name.to_string(), *val);
    }
    // template first so that instance overrides them
    for (name, val) in template.1.named.iter() {
        args.insert(name.to_string(), *val);
    }
    for (name, val) in instance.named.iter() {
        args.insert(name.to_string(), *val);
    }
    Ok(args)
}

fn perform_replacements(
    name: &str,
    loc: &pest::Span,
    templ: &(pest::Span, Template),
    args: HashMap<String, Arg>,
    date: Date,
) -> Result<Entry> {
    let (value, used_val) = instantiate_amount(name, loc, &templ.0, &templ.1.value, &args)?;
    let (tag, used_tag) = instanciate_tag(name, loc, &templ.0, &templ.1.tag, &args, date)?;
    for (argname, argval) in args.iter() {
        let use_v = used_val.contains(argname);
        let use_t = used_tag.contains(argname);
        match (argval, use_v, use_t) {
            (_, false, false) => {
                let err = Error::new("Unused argument")
                    .nonfatal()
                    .with_span(loc, format!("in instanciation of '{}'", name))
                    .with_message(format!("Argument {} is provided but not used", argname))
                    .with_span(&templ.0, "defined here")
                    .with_message("Remove argument or use in template");
                println!("{}", err);
            }
            (Arg::Amount(_), false, true) => {
                let err = Error::new("Needless amount")
                    .nonfatal()
                    .with_span(loc, format!("in instanciation of '{}'", name))
                    .with_message(format!("Argument '{}' has type 'amount' but could be a 'tag'", argname))
                    .with_span(&templ.0, "defined here")
                    .with_message("Change to string or use in amount calculation");
                println!("{}", err);
            }
            _ => (),
        }
    }
    Ok(Entry {
        value,
        cat: templ.1.cat,
        span: templ.1.span,
        tag,
    })
}

fn instantiate_amount(
    name: &str,
    loc_inst: &pest::Span,
    loc_templ: &pest::Span,
    templ: &AmountTemplate,
    args: &HashMap<String, Arg>,
) -> Result<(Amount, HashSet<String>)> {
    let mut sum = 0;
    let mut used = HashSet::new();
    for item in &templ.sum {
        match item {
            AmountTemplateItem::Cst(Amount(n)) => sum += n,
            AmountTemplateItem::Arg(a) => {
                used.insert(a.to_string());
                match args.get(*a) {
                    None => {
                        let err = Error::new("Missing argument")
                            .with_span(loc_inst, format!("in instanciation of '{}'", name))
                            .with_message(format!("Argument '{}' is not provided", a))
                            .with_span(loc_templ, "defined here")
                            .with_message("Remove argument from template body or provide a default value");
                        return Err(err);
                    }
                    Some(Arg::Amount(Amount(n))) => sum += n,
                    Some(Arg::Tag(_)) => {
                        let err = Error::new("Type mismatch")
                            .with_span(loc_inst, format!("in instanciation of '{}'", name))
                            .with_message(format!("Cannot treat tag as a monetary value"))
                            .with_span(loc_templ, "defined here")
                            .with_message("Make it a value");
                        return Err(err);
                    }
                }
            }
        }
    }
    Ok((Amount(if templ.sign { sum } else { -sum }), used))
}

fn instanciate_tag(
    name: &str,
    loc_inst: &pest::Span,
    loc_templ: &pest::Span,
    templ: &TagTemplate,
    args: &HashMap<String, Arg>,
    date: Date,
) -> Result<(Tag, HashSet<String>)> {
    let mut tag = String::new();
    let mut used = HashSet::new();
    for item in &templ.0 {
        match item {
            TagTemplateItem::Day => tag.push_str(&date.day().to_string()),
            TagTemplateItem::Month => tag.push_str(&date.month().to_string()),
            TagTemplateItem::Year => tag.push_str(&date.year().to_string()),
            TagTemplateItem::Date => tag.push_str(&date.to_string()),
            TagTemplateItem::Weekday => tag.push_str(&date.weekday().to_string()),
            TagTemplateItem::Raw(s) => tag.push_str(s),
            TagTemplateItem::Arg(a) => {
                used.insert(a.to_string());
                match args.get(*a) {
                    None => {
                        let err = Error::new("Missing argument")
                            .with_span(loc_inst, format!("in instanciation of '{}'", name))
                            .with_message(format!("Argument '{}' is not provided", a))
                            .with_span(loc_templ, "defined here")
                            .with_message("Remove argument from template body or provide a default value");
                        return Err(err);
                    }
                    Some(Arg::Amount(amount)) => tag.push_str(&amount.to_string()),
                    Some(Arg::Tag(t)) => tag.push_str(t),
                }
            }
        }
    }
    Ok((Tag(tag), used))
}