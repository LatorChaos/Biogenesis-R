// Biogenesis (C) 2006-2010 Joan Queralt Molina; Color Mod (C) 2012-2022 MarcoDBAA
// SPDX-License-Identifier: GPL-2.0-or-later

use super::{XmlError, XmlVersion, limit, malformed};
use std::collections::{BTreeMap, BTreeSet};

const MAX_DTD_BYTES: usize = 1 << 20;
const MAX_DECLARATIONS: usize = 256;
const MAX_TOKENS: usize = 8192;
const MAX_NAME_BYTES: usize = 128;
const MAX_EXPANDED_BYTES: usize = 1 << 20;
const MAX_REFERENCES: usize = 100_000;
const MAX_ENTITY_DEPTH: usize = 32;
const XML10_NAME_FLAGS: &[u8; 1 << 16] = include_bytes!("xml10_name_flags.bin");

#[derive(Debug)]
enum Token {
    Word(String),
    Literal(String),
    Mark(char),
}

struct LocatedToken {
    kind: Token,
    start: usize,
    end: usize,
}

struct Tokens {
    version: XmlVersion,
    values: std::iter::Peekable<std::vec::IntoIter<LocatedToken>>,
    source: String,
    end: usize,
}

impl Tokens {
    fn optional_next(&mut self) -> Option<Token> {
        let token = self.values.next()?;
        self.end = token.end;
        Some(token.kind)
    }
    fn next(&mut self) -> Result<Token, XmlError> {
        self.optional_next()
            .ok_or_else(|| malformed("Incomplete DTD declaration"))
    }
    fn space(&mut self) -> Result<(), XmlError> {
        let next = self
            .values
            .peek()
            .ok_or_else(|| malformed("Incomplete DTD declaration"))?;
        let gap = &self.source[self.end..next.start];
        if !gap.is_empty() && gap.chars().all(xml_space) {
            Ok(())
        } else {
            Err(malformed("Missing required DTD whitespace"))
        }
    }
    fn adjacent(&mut self) -> Result<(), XmlError> {
        if self
            .values
            .peek()
            .is_some_and(|token| token.start == self.end)
        {
            Ok(())
        } else {
            Err(malformed("Forbidden DTD whitespace"))
        }
    }
    fn word(&mut self) -> Result<String, XmlError> {
        match self.next()? {
            Token::Word(value) => Ok(value),
            _ => Err(malformed("Expected a DTD name")),
        }
    }
    fn name(&mut self) -> Result<String, XmlError> {
        let value = self.word()?;
        if valid_name(&value, self.version) {
            Ok(value)
        } else {
            Err(malformed("Invalid XML name"))
        }
    }
    fn mark(&mut self, value: char) -> Result<(), XmlError> {
        if matches!(self.next()?, Token::Mark(actual) if actual == value) {
            Ok(())
        } else {
            Err(malformed("Unexpected DTD punctuation"))
        }
    }
    fn take_mark(&mut self, value: char) -> bool {
        if matches!(self.values.peek(), Some(LocatedToken { kind: Token::Mark(actual), .. }) if *actual == value)
        {
            self.optional_next();
            true
        } else {
            false
        }
    }
    fn suffix(&mut self) -> Result<Option<char>, XmlError> {
        let mark = match self.values.peek() {
            Some(LocatedToken {
                kind: Token::Mark(value @ ('?' | '*' | '+')),
                ..
            }) => Some(*value),
            _ => None,
        };
        if let Some(value) = mark {
            self.adjacent()?;
            self.take_mark(value);
        }
        Ok(mark)
    }
    fn literal(&mut self) -> Result<String, XmlError> {
        match self.next()? {
            Token::Literal(value) => Ok(value),
            _ => Err(malformed("Expected a quoted DTD value")),
        }
    }
}

fn tokens(version: XmlVersion, text: &str, count: &mut usize) -> Result<Tokens, XmlError> {
    if text.len() > MAX_DTD_BYTES {
        return Err(limit("DTD bytes", MAX_DTD_BYTES));
    }
    let mut values = Vec::new();
    let mut cursor = 0;
    let mut declaration = false;
    while cursor < text.len() {
        let rest = &text[cursor..];
        let ch = rest.chars().next().unwrap();
        if xml_space(ch) {
            cursor += ch.len_utf8();
            continue;
        }
        if rest.starts_with("<!--") {
            if declaration {
                return Err(malformed("Comment inside DTD declaration"));
            }
            let end = rest
                .find("-->")
                .ok_or_else(|| malformed("Unclosed DTD comment"))?;
            if end < 4 || rest[4..end].contains("--") || rest[4..end].ends_with('-') {
                return Err(malformed("Invalid DTD comment"));
            }
            cursor += end + 3;
            continue;
        }
        if rest.starts_with("<?") {
            if declaration {
                return Err(malformed("Processing instruction inside DTD declaration"));
            }
            let end = rest.find("?>").ok_or_else(|| malformed("Unclosed DTD processing instruction"))?;
            if end < 2 {
                return Err(malformed("Invalid DTD processing instruction"));
            }
            let body = &rest[2..end];
            let target = body.split(xml_space).next().unwrap_or("");
            if target.len() > MAX_NAME_BYTES {
                return Err(limit("DTD name bytes", MAX_NAME_BYTES));
            }
            if !valid_name(target, version) || target.eq_ignore_ascii_case("xml") {
                return Err(malformed("Invalid DTD processing instruction target"));
            }
            cursor += end + 2;
            continue;
        }
        let start = cursor;
        let kind = if ch == '\'' || ch == '"' {
            let content = &rest[ch.len_utf8()..];
            let end = content
                .find(ch)
                .ok_or_else(|| malformed("Unclosed DTD literal"))?;
            cursor += ch.len_utf8() + end + ch.len_utf8();
            Token::Literal(content[..end].to_owned())
        } else if "<>![]()|*+?%#;,".contains(ch) {
            cursor += ch.len_utf8();
            if ch == '<' {
                declaration = true;
            } else if ch == '>' {
                declaration = false;
            }
            Token::Mark(ch)
        } else {
            let end = rest
                .char_indices()
                .find_map(|(index, value)| {
                    (xml_space(value) || "<>![]()|*+?%#,;\"'".contains(value)).then_some(index)
                })
                .unwrap_or(rest.len());
            if end == 0 || end > MAX_NAME_BYTES {
                return Err(limit("DTD name bytes", MAX_NAME_BYTES));
            }
            let word = &rest[..end];
            if !word.chars().all(|ch| name_char(ch, version)) {
                return Err(malformed("Invalid DTD token"));
            }
            cursor += end;
            Token::Word(word.to_owned())
        };
        *count = count.checked_add(1).filter(|value| *value <= MAX_TOKENS)
            .ok_or_else(|| limit("DTD tokens", MAX_TOKENS))?;
        values.push(LocatedToken {
            kind,
            start,
            end: cursor,
        });
    }
    Ok(Tokens {
        version,
        values: values.into_iter().peekable(),
        source: text.to_owned(),
        end: 0,
    })
}

struct DtdFrame {
    input: Tokens,
    parameter: Option<String>,
}

#[derive(Debug)]
enum ParameterEntity {
    Internal(String),
    External,
}

#[derive(Debug)]
enum Model {
    Empty,
    Any,
    Mixed(BTreeSet<String>),
    General(Program),
    Children {
        name: String,
        minimum: usize,
        maximum: Option<usize>,
        block_width: usize,
    },
}

fn mixed_model(input: &mut Tokens) -> Result<Model, XmlError> {
    input.adjacent()?;
    if input.word()? != "PCDATA" {
        return Err(malformed("Expected PCDATA in mixed content model"));
    }
    let mut names = BTreeSet::new();
    while input.take_mark('|') {
        if !names.insert(input.name()?) {
            return Err(malformed("Duplicate name in mixed content model"));
        }
    }
    input.mark(')')?;
    match input.suffix()? {
        Some('*') => {}
        None if names.is_empty() => {}
        _ => return Err(malformed("Mixed child content requires an adjacent star")),
    }
    Ok(Model::Mixed(names))
}

#[derive(Debug)]
enum Particle {
    Name(String),
    Sequence(Vec<Particle>),
    Choice(Vec<Particle>),
    Repeat(Box<Particle>, char),
}

fn compact_model(particle: &Particle) -> Option<Model> {
    fn fixed(particle: &Particle) -> Option<(&str, usize)> {
        match particle {
            Particle::Name(name) => Some((name, 1)),
            Particle::Sequence(items) => {
                let mut parts = items.iter();
                let (name, mut width) = fixed(parts.next()?)?;
                for item in parts {
                    let (next, count) = fixed(item)?;
                    if next != name { return None; }
                    width = width.checked_add(count)?;
                }
                Some((name, width))
            }
            _ => None,
        }
    }
    let (body, outer) = match particle {
        Particle::Repeat(body, suffix) => (body.as_ref(), Some(*suffix)),
        body => (body, None),
    };
    let single = match body {
        Particle::Sequence(items) if items.len() == 1 => match &items[0] {
            Particle::Repeat(name, suffix) => match name.as_ref() {
                Particle::Name(name) => Some((name.as_str(), *suffix)),
                _ => None,
            },
            _ => None,
        },
        _ => None,
    };
    let (name, width, mut minimum, mut maximum) = if let Some((name, suffix)) = single {
        let (minimum, maximum) = match suffix {
            '?' => (0, Some(1)), '*' => (0, None), '+' => (1, None), _ => unreachable!(),
        };
        (name, 1, minimum, maximum)
    } else {
        let (name, width) = fixed(body)?;
        (name, width, width, Some(width))
    };
    match outer {
        Some('*') => { minimum = 0; maximum = None; }
        Some('+') => maximum = None,
        Some('?') => minimum = 0,
        _ => {}
    }
    Some(Model::Children { name: name.to_owned(), minimum, maximum, block_width: width })
}

const MAX_MODEL_STATES: usize = 2 * MAX_TOKENS + 2 * MAX_DECLARATIONS;
const MAX_MODEL_EDGES: usize = 4 * MAX_TOKENS + 2 * MAX_DECLARATIONS;

#[derive(Debug)]
enum Step {
    Name(String, usize),
    Jump(usize),
    Split(usize, usize),
    Accept,
}

#[derive(Default)]
struct ModelBudget { states: usize, edges: usize }

impl ModelBudget {
    fn charge(&mut self, step: &Step, state: bool) -> Result<(), XmlError> {
        if state {
            self.states = self.states.checked_add(1).filter(|count| *count <= MAX_MODEL_STATES)
                .ok_or_else(|| limit("DTD model states", MAX_MODEL_STATES))?;
        }
        let edges = match step { Step::Split(_, _) => 2, Step::Accept => 0, _ => 1 };
        self.edges = self.edges.checked_add(edges).filter(|count| *count <= MAX_MODEL_EDGES)
            .ok_or_else(|| limit("DTD model edges", MAX_MODEL_EDGES))?;
        Ok(())
    }
}

#[derive(Debug)]
struct Program {
    steps: Vec<Step>,
    start: usize,
}

impl Program {
    fn emit(&mut self, step: Step, budget: &mut ModelBudget) -> Result<usize, XmlError> {
        budget.charge(&step, true)?;
        self.steps.try_reserve(1).map_err(|_| limit("DTD model states", MAX_MODEL_STATES))?;
        let index = self.steps.len();
        self.steps.push(step);
        Ok(index)
    }

    fn connect(&mut self, index: usize, step: Step, budget: &mut ModelBudget) -> Result<(), XmlError> {
        debug_assert!(matches!(self.steps[index], Step::Accept));
        budget.charge(&step, false)?;
        self.steps[index] = step;
        Ok(())
    }

    fn fragment(&mut self, particle: Particle, budget: &mut ModelBudget) -> Result<(usize, usize), XmlError> {
        match particle {
            Particle::Name(name) => {
                let end = self.emit(Step::Accept, budget)?;
                Ok((self.emit(Step::Name(name, end), budget)?, end))
            }
            Particle::Sequence(items) => {
                let mut parts = items.into_iter();
                let (start, mut end) = self.fragment(parts.next().unwrap(), budget)?;
                for part in parts {
                    let (next, last) = self.fragment(part, budget)?;
                    self.connect(end, Step::Jump(next), budget)?;
                    end = last;
                }
                Ok((start, end))
            }
            Particle::Choice(items) => {
                let mut parts = items.into_iter();
                let (mut start, mut end) = self.fragment(parts.next().unwrap(), budget)?;
                for part in parts {
                    let (right, last) = self.fragment(part, budget)?;
                    let joined = self.emit(Step::Accept, budget)?;
                    let branch = self.emit(Step::Split(start, right), budget)?;
                    self.connect(end, Step::Jump(joined), budget)?;
                    self.connect(last, Step::Jump(joined), budget)?;
                    start = branch;
                    end = joined;
                }
                Ok((start, end))
            }
            Particle::Repeat(body, suffix) => {
                let (child, last) = self.fragment(*body, budget)?;
                let end = self.emit(Step::Accept, budget)?;
                let start = self.emit(match suffix {
                    '?' | '*' => Step::Split(child, end),
                    '+' => Step::Jump(child),
                    _ => unreachable!(),
                }, budget)?;
                self.connect(last, match suffix {
                    '?' => Step::Jump(end),
                    '*' => Step::Jump(start),
                    '+' => Step::Split(child, end),
                    _ => unreachable!(),
                }, budget)?;
                Ok((start, end))
            }
        }
    }

    fn compile(particle: Particle, budget: &mut ModelBudget) -> Result<Self, XmlError> {
        let mut program = Self { steps: Vec::new(), start: 0 };
        program.start = program.fragment(particle, budget)?.0;
        Ok(program)
    }

    #[cfg(test)]
    fn edges(&self) -> usize {
        self.steps.iter().map(|step| match step { Step::Split(_, _) => 2, Step::Accept => 0, _ => 1 }).sum()
    }

    fn close(&self, scratch: &mut ModelScratch) -> bool {
        let mut accepts = false;
        while let Some(index) = scratch.pending.pop() {
            match &self.steps[index] {
                Step::Name(_, _) => scratch.next.push(index),
                Step::Jump(next) => scratch.schedule(*next),
                Step::Split(left, right) => { scratch.schedule(*left); scratch.schedule(*right); }
                Step::Accept => accepts = true,
            }
        }
        for index in scratch.touched.drain(..) { scratch.marked[index] = false; }
        accepts
    }

    fn cursor(&self, scratch: &mut ModelScratch) -> Result<ChildCursor, XmlError> {
        scratch.begin(self.steps.len())?;
        scratch.schedule(self.start);
        let accepts = self.close(scratch);
        Ok(ChildCursor { active: std::mem::take(&mut scratch.next), accepts })
    }

    fn advance(&self, cursor: &mut ChildCursor, name: &str, scratch: &mut ModelScratch) -> Result<(), XmlError> {
        scratch.begin(self.steps.len())?;
        for index in &cursor.active {
            if let Step::Name(label, next) = &self.steps[*index] {
                if label == name { scratch.schedule(*next); }
            }
        }
        cursor.accepts = self.close(scratch);
        std::mem::swap(&mut cursor.active, &mut scratch.next);
        if cursor.active.is_empty() && !cursor.accepts {
            Err(malformed("Element violates its DTD content model"))
        } else { Ok(()) }
    }
}

pub(super) struct ChildCursor {
    active: Vec<usize>,
    accepts: bool,
}

#[derive(Default)]
pub(super) struct ModelScratch {
    marked: Vec<bool>,
    touched: Vec<usize>,
    pending: Vec<usize>,
    next: Vec<usize>,
}

impl ModelScratch {
    fn begin(&mut self, states: usize) -> Result<(), XmlError> {
        debug_assert!(self.touched.is_empty() && self.pending.is_empty());
        if states > self.marked.len() {
            self.marked.try_reserve(states - self.marked.len()).map_err(|_| limit("DTD model states", MAX_MODEL_STATES))?;
            self.marked.resize(states, false);
        }
        self.next.clear();
        Ok(())
    }

    fn schedule(&mut self, index: usize) {
        if !self.marked[index] {
            self.marked[index] = true;
            self.touched.push(index);
            self.pending.push(index);
        }
    }
}

fn children_model(input: &mut Tokens, budget: &mut ModelBudget) -> Result<Model, XmlError> {
    if input.take_mark('#') { return mixed_model(input); }
    #[derive(Default)]
    struct Group { items: Vec<Particle>, separator: Option<char>, after_child: bool }
    fn push(group: &mut Group, mut particle: Particle, input: &mut Tokens) -> Result<(), XmlError> {
        if let Some(suffix) = input.suffix()? { particle = Particle::Repeat(Box::new(particle), suffix); }
        group.items.push(particle);
        group.after_child = true;
        Ok(())
    }
    let mut groups = vec![Group::default()];
    loop {
        let depth = groups.len();
        let group = groups.last_mut().unwrap();
        match input.next()? {
            Token::Word(name) if !group.after_child => {
                if !valid_name(&name, input.version) { return Err(malformed("Invalid XML name")); }
                push(group, Particle::Name(name), input)?;
            }
            Token::Mark('(') if !group.after_child => {
                if depth >= super::MAX_DEPTH { return Err(limit("DTD content-model depth", super::MAX_DEPTH)); }
                groups.push(Group::default());
            }
            Token::Mark(separator @ (',' | '|')) if group.after_child => {
                if group.separator.is_some_and(|prior| prior != separator) { return Err(malformed("Mixed DTD group separators")); }
                group.separator = Some(separator);
                group.after_child = false;
            }
            Token::Mark(')') if group.after_child => {
                let closed = groups.pop().unwrap();
                let mut particle = if closed.separator == Some('|') { Particle::Choice(closed.items) } else { Particle::Sequence(closed.items) };
                if let Some(suffix) = input.suffix()? { particle = Particle::Repeat(Box::new(particle), suffix); }
                if let Some(parent) = groups.last_mut() {
                    parent.items.push(particle);
                    parent.after_child = true;
                } else {
                    return if let Some(compact) = compact_model(&particle) { Ok(compact) }
                        else { Ok(Model::General(Program::compile(particle, budget)?)) };
                }
            }
            _ => return Err(malformed("Invalid DTD child sequence")),
        }
    }
}

#[derive(Debug)]
enum AttributeType {
    Cdata,
    Id,
    IdRef,
    IdRefs,
    NmToken,
    NmTokens,
    Entity,
    Entities,
    Notation(BTreeSet<String>),
    Enum(BTreeSet<String>),
}

#[derive(Debug)]
enum DefaultValue {
    Required,
    Implied,
    Value(String),
    Fixed(String),
}

pub(super) enum ContentReference<'a> {
    Character(char),
    Replacement { name: &'a str, value: &'a str },
}

pub(super) fn enter_entity<'a>(name: &'a str, trail: &mut Vec<&'a str>) -> Result<(), XmlError> {
    enter_entity_at_depth(name, trail, 0)
}

fn enter_entity_at_depth<'a>(name: &'a str, trail: &mut Vec<&'a str>, parameter_depth: usize) -> Result<(), XmlError> {
    if trail.contains(&name) {
        return Err(malformed("Recursive entity definition"));
    }
    if parameter_depth.checked_add(trail.len()).is_none_or(|depth| depth >= MAX_ENTITY_DEPTH) {
        return Err(limit("entity recursion", MAX_ENTITY_DEPTH));
    }
    trail.push(name);
    Ok(())
}

#[derive(Debug)]
struct Attribute {
    kind: AttributeType,
    default: DefaultValue,
}

#[derive(Debug)]
enum Entity {
    Internal(String),
    ParsedExternal,
    Unparsed(String),
}

fn external_identifier(input: &mut Tokens, kind: &str, notation: bool) -> Result<(), XmlError> {
    input.space()?;
    let literal = input.literal()?;
    if kind == "PUBLIC" {
        if !literal.chars().all(|ch| ch.is_ascii_alphanumeric()
            || " \r\n-'()+,./:=?;!*#@$_%".contains(ch)) {
            return Err(malformed("Invalid public identifier"));
        }
        if matches!(input.values.peek(), Some(LocatedToken { kind: Token::Literal(_), .. })) {
            input.space()?;
            input.literal()?;
        } else if !notation {
            return Err(malformed("PUBLIC entity requires a system identifier"));
        }
    }
    Ok(())
}

#[derive(Debug, Default)]
pub(super) struct Dtd {
    version: XmlVersion,
    root: String,
    elements: BTreeMap<String, Model>,
    attributes: BTreeMap<String, BTreeMap<String, Attribute>>,
    entities: BTreeMap<String, Entity>,
    notations: BTreeSet<String>,
}

// Count retained string payload, not container or allocator overhead
pub(super) const MAX_RETAINED_PAYLOAD_BYTES: usize = super::MAX_DOCUMENT_BYTES + MAX_EXPANDED_BYTES;

#[derive(Default)]
pub(super) struct RetainedPayloadBudget {
    bytes: usize,
}

impl RetainedPayloadBudget {
    pub(super) fn charge(&mut self, amount: usize) -> Result<(), XmlError> {
        self.bytes = self.bytes.checked_add(amount)
            .filter(|value| *value <= MAX_RETAINED_PAYLOAD_BYTES)
            .ok_or_else(|| limit("retained XML payload bytes", MAX_RETAINED_PAYLOAD_BYTES))?;
        Ok(())
    }
}

#[derive(Default)]
pub(super) struct DocumentIds {
    ids: BTreeSet<String>,
    references: BTreeSet<String>,
}

impl DocumentIds {
    pub(super) fn finish(&self) -> Result<(), XmlError> {
        if self.references.iter().any(|name| !self.ids.contains(name)) {
            return Err(malformed("Unresolved IDREF"));
        }
        Ok(())
    }
}

#[derive(Default)]
pub(super) struct ExpansionBudget {
    bytes: usize,
    references: usize,
}

impl ExpansionBudget {
    fn reference(&mut self) -> Result<(), XmlError> {
        self.references += 1;
        if self.references > MAX_REFERENCES {
            Err(limit("entity references", MAX_REFERENCES))
        } else {
            Ok(())
        }
    }
    pub(super) fn generated(&mut self, bytes: usize, copies: usize) -> Result<(), XmlError> {
        let bytes = bytes.checked_mul(copies)
            .ok_or_else(|| limit("expanded entity bytes", MAX_EXPANDED_BYTES))?;
        self.bytes = self.bytes.checked_add(bytes)
            .ok_or_else(|| limit("expanded entity bytes", MAX_EXPANDED_BYTES))?;
        if self.bytes > MAX_EXPANDED_BYTES {
            return Err(limit("expanded entity bytes", MAX_EXPANDED_BYTES));
        }
        Ok(())
    }

    fn append(&mut self, output: &mut String, text: &str, generated: bool) -> Result<(), XmlError> {
        if generated {
            self.generated(text.len(), 1)?;
        }
        if output.len().saturating_add(text.len()) > super::MAX_DOCUMENT_BYTES {
            return Err(limit("expanded value bytes", super::MAX_DOCUMENT_BYTES));
        }
        output.push_str(text);
        Ok(())
    }
}

impl Dtd {
    #[cfg(test)]
    pub(super) fn parse(text: &str) -> Result<Self, XmlError> {
        Self::parse_bounded(XmlVersion::Xml10, text, &mut ExpansionBudget::default(), &mut RetainedPayloadBudget::default())
    }

    pub(super) fn parse_bounded(
        version: XmlVersion,
        text: &str,
        budget: &mut ExpansionBudget,
        retained: &mut RetainedPayloadBudget,
    ) -> Result<Self, XmlError> {
        let mut token_count = 0;
        let mut input = tokens(version, text, &mut token_count)?;
        let mut dtd = Self {
            version,
            root: input.name()?,
            ..Self::default()
        };
        if !input.take_mark('[') {
            return match input.optional_next() {
                Some(Token::Word(value)) if value == "SYSTEM" || value == "PUBLIC" => {
                    Err(XmlError::ExternalResource)
                }
                None => Err(malformed("DTD has no declared document grammar")),
                _ => Err(malformed("Invalid DOCTYPE")),
            };
        }
        let mut declarations = 0usize;
        let mut model_budget = ModelBudget::default();
        let mut parameters = BTreeMap::new();
        let mut frames = vec![DtdFrame { input, parameter: None }];
        loop {
            let parameter_depth = frames.len() - 1;
            let input = &mut frames.last_mut().unwrap().input;
            if input.values.peek().is_none() {
                if parameter_depth == 0 {
                    return Err(malformed("Unclosed DTD internal subset"));
                }
                frames.pop();
                continue;
            }
            if input.take_mark(']') {
                if parameter_depth != 0 {
                    return Err(malformed("Parameter entity cannot close the DTD subset"));
                }
                break;
            }
            if input.take_mark('%') {
                input.adjacent()?;
                let name = input.name()?;
                input.adjacent()?;
                input.mark(';')?;
                budget.reference()?;
                let value = match parameters.get(&name) {
                    Some(ParameterEntity::Internal(value)) => value,
                    Some(ParameterEntity::External) => return Err(XmlError::ExternalResource),
                    None => return Err(malformed("Undeclared parameter entity")),
                };
                if frames.iter().any(|frame| frame.parameter.as_deref() == Some(name.as_str())) {
                    return Err(malformed("Recursive parameter entity definition"));
                }
                if parameter_depth >= MAX_ENTITY_DEPTH {
                    return Err(limit("entity recursion", MAX_ENTITY_DEPTH));
                }
                budget.generated(value.len(), 1)?;
                retained.charge(value.len())?;
                retained.charge(name.len())?;
                let input = tokens(version, value, &mut token_count)?;
                frames.push(DtdFrame { input, parameter: Some(name) });
                continue;
            }
            declarations = declarations.checked_add(1).filter(|value| *value <= MAX_DECLARATIONS)
                .ok_or_else(|| limit("DTD declarations", MAX_DECLARATIONS))?;
            // A declaration consumes only its owning frame, including its closing delimiter
            input.mark('<')?;
            input.adjacent()?;
            input.mark('!')?;
            input.adjacent()?;
            let keyword = input.word()?;
            input.space()?;
            match keyword.as_str() {
                "ELEMENT" => {
                    let name = input.name()?;
                    input.space()?;
                    let model = match input.next()? {
                        Token::Word(value) if value == "EMPTY" => Model::Empty,
                        Token::Word(value) if value == "ANY" => Model::Any,
                        Token::Mark('(') => children_model(input, &mut model_budget)?,
                        Token::Mark('%') => return Err(malformed("Parameter reference inside DTD declaration")),
                        _ => return Err(XmlError::UnsupportedDtd("element content model")),
                    };
                    if dtd.elements.insert(name, model).is_some() {
                        return Err(malformed("Duplicate element declaration"));
                    }
                    input.mark('>')?;
                }
                "ATTLIST" => {
                    let owner = input.name()?;
                    let mut attributes = dtd.attributes.remove(&owner).unwrap_or_default();
                    let mut first = true;
                    while !input.take_mark('>') {
                        if first {
                            input.space()?;
                            first = false;
                        }
                        // Java accepts adjacent attribute names after quoted defaults

                        if attributes.len() >= 128 {
                            return Err(limit("declared attributes per element", 128));
                        }
                        let name = input.name()?;
                        input.space()?;
                        let kind = if input.take_mark('(') {
                            let mut values = BTreeSet::new();
                            loop {
                                let value = input.word()?;
                                if value.is_empty() || !value.chars().all(|ch| name_char(ch, version)) {
                                    return Err(malformed("Invalid attribute enumeration token"));
                                }
                                if !values.insert(value) {
                                    return Err(malformed("Duplicate attribute enumeration token"));
                                }
                                if input.take_mark(')') {
                                    break;
                                }
                                input.mark('|')?;
                            }
                            AttributeType::Enum(values)
                        } else {
                            match input.word()?.as_str() {
                                "CDATA" => AttributeType::Cdata,
                                "ID" => AttributeType::Id,
                                "IDREF" => AttributeType::IdRef,
                                "IDREFS" => AttributeType::IdRefs,
                                "NMTOKEN" => AttributeType::NmToken,
                                "NMTOKENS" => AttributeType::NmTokens,
                                "ENTITY" => AttributeType::Entity,
                                "ENTITIES" => AttributeType::Entities,
                                "NOTATION" => {
                                    input.space()?;
                                    input.mark('(')?;
                                    let mut names = BTreeSet::new();
                                    loop {
                                        if !names.insert(input.name()?) {
                                            return Err(malformed("Duplicate NOTATION list name"));
                                        }
                                        if input.take_mark(')') { break; }
                                        input.mark('|')?;
                                    }
                                    if attributes.values().any(|value: &Attribute| matches!(value.kind, AttributeType::Notation(_))) {
                                        return Err(malformed("Only one NOTATION attribute is permitted per element"));
                                    }
                                    AttributeType::Notation(names)
                                }
                                _ => return Err(XmlError::UnsupportedDtd("attribute type")),
                            }
                        };
                        input.space()?;
                        let default = if input.take_mark('#') {
                            input.adjacent()?;
                            match input.word()?.as_str() {
                                "REQUIRED" => DefaultValue::Required,
                                "IMPLIED" => DefaultValue::Implied,
                                "FIXED" => {
                                    input.space()?;
                                    DefaultValue::Fixed(input.literal()?)
                                }
                                _ => {
                                    return Err(malformed("Unknown attribute default declaration"));
                                }
                            }
                        } else {
                            DefaultValue::Value(input.literal()?)
                        };
                        if let DefaultValue::Value(value) | DefaultValue::Fixed(value) = &default {
                            validate_literal(value, false, version)?;
                        }
                        if matches!(kind, AttributeType::Entity | AttributeType::Entities | AttributeType::Notation(_)) {
                            if let DefaultValue::Value(raw) | DefaultValue::Fixed(raw) = &default {
                                let expanded = normalize(&kind, dtd.expand_with_parameter_depth(raw, budget, true, true, &mut Vec::new(), parameter_depth)?);
                                // Source permits an empty ENTITIES default until it is applied
                                if !(matches!(kind, AttributeType::Entities) && expanded.is_empty())
                                    && !valid_attribute(&kind, &expanded, version) {
                                    return Err(malformed("Invalid lexical attribute default"));
                                }
                            }
                        }
                        if matches!(kind, AttributeType::Id) {
                            if !matches!(default, DefaultValue::Required | DefaultValue::Implied) {
                                return Err(malformed("ID defaults must be REQUIRED or IMPLIED"));
                            }
                            if attributes.values().any(|value| matches!(value.kind, AttributeType::Id)) {
                                return Err(malformed("Only one ID attribute is permitted per element"));
                            }
                        }
                        if attributes
                            .insert(name, Attribute { kind, default })
                            .is_some()
                        {
                            return Err(malformed("Duplicate attribute declaration"));
                        }
                    }
                    dtd.attributes.insert(owner, attributes);
                }
                "ENTITY" => {
                    let parameter = input.take_mark('%');
                    if parameter {
                        input.space()?;
                    }
                    let name = input.name()?;
                    input.space()?;
                    let entity = match input.next()? {
                        Token::Word(value) if value == "SYSTEM" || value == "PUBLIC" => {
                            external_identifier(input, &value, false)?;
                            if !parameter && matches!(input.values.peek(), Some(LocatedToken { kind: Token::Word(_), .. })) {
                                input.space()?;
                                if input.word()? != "NDATA" { return Err(malformed("Invalid unparsed entity declaration")); }
                                input.space()?;
                                Entity::Unparsed(input.name()?)
                            } else { Entity::ParsedExternal }
                        }
                        Token::Literal(value) => Entity::Internal(entity_literal(&value, version)?),
                        _ => return Err(malformed("Invalid entity declaration")),
                    };
                    if parameter {
                        input.mark('>')?;
                        if !parameters.contains_key(&name) {
                            retained.charge(name.len())?;
                            let value = match entity {
                                Entity::Internal(value) => {
                                    retained.charge(value.len())?;
                                    ParameterEntity::Internal(value)
                                }
                                Entity::ParsedExternal => ParameterEntity::External,
                                Entity::Unparsed(_) => unreachable!(),
                            };
                            parameters.insert(name, value);
                        }
                        continue;
                    }
                    if predefined(&name).is_some() {
                        return Err(malformed("Reserved entity declaration"));
                    }
                    input.mark('>')?;
                    dtd.entities.entry(name).or_insert(entity);
                }
                "NOTATION" => {
                    let name = input.name()?;
                    input.space()?;
                    let kind = input.word()?;
                    if kind != "SYSTEM" && kind != "PUBLIC" {
                        return Err(malformed("Invalid notation identifier"));
                    }
                    external_identifier(input, &kind, true)?;
                    input.mark('>')?;
                    if !dtd.notations.insert(name) { return Err(malformed("Duplicate notation declaration")); }
                }
                _ => return Err(XmlError::UnsupportedDtd("declaration kind")),
            }
        }
        if frames[0].input.optional_next().is_some() {
            return Err(malformed("Trailing DOCTYPE data"));
        }
        if !dtd.elements.contains_key(&dtd.root) {
            return Err(malformed("Undeclared document element"));
        }
        for (owner, attributes) in &dtd.attributes {
            for declaration in attributes.values() {
                if let AttributeType::Notation(names) = &declaration.kind {
                    if matches!(dtd.elements.get(owner), Some(Model::Empty)) {
                        return Err(malformed("NOTATION attribute on EMPTY element"));
                    }
                    if names.iter().any(|name| !dtd.notations.contains(name)) {
                        return Err(malformed("Undeclared NOTATION list name"));
                    }
                }
                if matches!(declaration.kind, AttributeType::Cdata | AttributeType::Enum(_)
                    | AttributeType::Entity | AttributeType::Entities | AttributeType::Notation(_)) {
                    continue;
                }
                if let DefaultValue::Value(raw) | DefaultValue::Fixed(raw) = &declaration.default {
                    let expanded = dtd.expand(raw, budget, true, true, &mut Vec::new())?;
                    if !valid_attribute(&declaration.kind, &normalize(&declaration.kind, expanded), version) {
                        return Err(malformed("Invalid lexical attribute default"));
                    }
                }
            }
        }
        for entity in dtd.entities.values() {
            if let Entity::Unparsed(name) = entity {
                if !dtd.notations.contains(name) { return Err(malformed("Undeclared unparsed entity notation")); }
            }
        }
        retained.charge(dtd.root.len())?;
        for (name, model) in &dtd.elements {
            retained.charge(name.len())?;
            match model {
                Model::Children { name, .. } => retained.charge(name.len())?,
                Model::Mixed(names) => for name in names { retained.charge(name.len())?; },
                Model::General(program) => for step in &program.steps {
                    if let Step::Name(name, _) = step { retained.charge(name.len())?; }
                },
                _ => {}
            }
        }
        for (owner, attributes) in &dtd.attributes {
            retained.charge(owner.len())?;
            for (name, declaration) in attributes {
                retained.charge(name.len())?;
                if let AttributeType::Enum(values) | AttributeType::Notation(values) = &declaration.kind {
                    for value in values { retained.charge(value.len())?; }
                }
                if let DefaultValue::Value(value) | DefaultValue::Fixed(value) = &declaration.default {
                    retained.charge(value.len())?;
                }
            }
        }
        for name in &dtd.notations { retained.charge(name.len())?; }
        for (name, entity) in &dtd.entities {
            retained.charge(name.len())?;
            match entity {
                Entity::Internal(value) | Entity::Unparsed(value) => retained.charge(value.len())?,
                Entity::ParsedExternal => {}
            }
        }
        Ok(dtd)
    }

    pub(super) fn root(&self) -> &str {
        &self.root
    }

    pub(super) fn ignorable_whitespace(&self, owner: &str, value: &str) -> bool {
        matches!(self.elements.get(owner), Some(Model::General(_) | Model::Children { .. }))
            && value.chars().all(xml_space)
    }

    pub(super) fn attributes(
        &self,
        owner: &str,
        raw: BTreeMap<String, String>,
        budget: &mut ExpansionBudget,
    ) -> Result<BTreeMap<String, String>, XmlError> {
        self.attributes_with_entities(owner, raw, budget, &mut Vec::new())
    }

    pub(super) fn attributes_with_entities<'a>(
        &'a self,
        owner: &str,
        raw: BTreeMap<String, String>,
        budget: &mut ExpansionBudget,
        trail: &mut Vec<&'a str>,
    ) -> Result<BTreeMap<String, String>, XmlError> {
        if !self.elements.contains_key(owner) {
            return Err(malformed("Undeclared element"));
        }
        let empty = BTreeMap::new();
        let declared = self.attributes.get(owner).unwrap_or(&empty);
        for key in raw.keys() {
            if !declared.contains_key(key) {
                return Err(malformed("Undeclared attribute"));
            }
        }
        let mut result = BTreeMap::new();
        for (name, declaration) in declared {
            let (value, generated) = if let Some(value) = raw.get(name) {
                (value, false)
            } else {
                match &declaration.default {
                    DefaultValue::Required => return Err(XmlError::MissingAttribute(name.clone())),
                    DefaultValue::Implied => continue,
                    DefaultValue::Value(value) | DefaultValue::Fixed(value) => (value, true),
                }
            };
            let expanded = normalize(&declaration.kind, self.expand(value, budget, true, generated, trail)?);
            if !valid_attribute(&declaration.kind, &expanded, self.version) {
                return if matches!(declaration.kind, AttributeType::Enum(_)) {
                    Err(XmlError::InvalidAttribute(name.clone()))
                } else {
                    Err(malformed("Invalid tokenized attribute value"))
                };
            }
            if matches!(declaration.kind, AttributeType::Entity | AttributeType::Entities)
                && expanded.split(' ').any(|name| !matches!(self.entities.get(name), Some(Entity::Unparsed(_)))) {
                return Err(malformed("Attribute requires a declared unparsed entity"));
            }
            if let DefaultValue::Fixed(fixed) = &declaration.default {
                let fixed = normalize(&declaration.kind, self.expand(fixed, budget, true, true, trail)?);
                if fixed != expanded {
                    return Err(XmlError::InvalidAttribute(name.clone()));
                }
            }
            result.insert(name.clone(), expanded);
        }
        Ok(result)
    }

    pub(super) fn identities(
        &self,
        owner: &str,
        attributes: &BTreeMap<String, String>,
        document: &mut DocumentIds,
        retained: &mut RetainedPayloadBudget,
    ) -> Result<(), XmlError> {
        let Some(declarations) = self.attributes.get(owner) else { return Ok(()); };
        for (name, value) in attributes {
            match declarations[name].kind {
                AttributeType::Id => {
                    if document.ids.contains(value) { return Err(malformed("Duplicate XML ID")); }
                    retained.charge(value.len())?;
                    document.ids.insert(value.clone());
                }
                AttributeType::IdRef | AttributeType::IdRefs => {
                    for reference in value.split(' ') {
                        if !document.references.contains(reference) {
                            retained.charge(reference.len())?;
                            document.references.insert(reference.to_owned());
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub(super) fn cursor(&self, name: &str, scratch: &mut ModelScratch) -> Result<Option<ChildCursor>, XmlError> {
        match self.elements.get(name) {
            Some(Model::General(program)) => program.cursor(scratch).map(Some),
            _ => Ok(None),
        }
    }

    pub(super) fn child(&self, parent: &str, child: &str, cursor: &mut Option<ChildCursor>, scratch: &mut ModelScratch) -> Result<(), XmlError> {
        match self.elements.get(parent) {
            Some(Model::Any) => Ok(()),
            Some(Model::General(program)) => program.advance(cursor.as_mut().expect("General model cursor"), child, scratch),
            Some(Model::Mixed(names)) if names.contains(child) => Ok(()),
            Some(Model::Children { name, .. }) if name == child => Ok(()),
            _ => Err(malformed("Element violates its DTD content model")),
        }
    }

    pub(super) fn finish(
        &self,
        name: &str,
        children: usize,
        text: bool,
        nonspace: bool,
        cursor: Option<&ChildCursor>,
    ) -> Result<(), XmlError> {
        let valid = match self.elements.get(name) {
            Some(Model::Any | Model::Mixed(_)) => true,
            Some(Model::General(_)) => !nonspace && cursor.is_some_and(|cursor| cursor.accepts),
            Some(Model::Empty) => children == 0 && !text,
            Some(Model::Children {
                minimum, maximum, block_width, ..
            }) => !nonspace && children >= *minimum
                && maximum.is_none_or(|max| children <= max)
                && children % *block_width == 0,
            None => false,
        };
        if valid {
            Ok(())
        } else {
            Err(malformed("Element violates its DTD content model"))
        }
    }

    pub(super) fn content_reference<'a>(
        &'a self,
        name: &str,
        budget: &mut ExpansionBudget,
    ) -> Result<ContentReference<'a>, XmlError> {
        budget.reference()?;
        if let Some(value) = predefined(name) {
            return Ok(ContentReference::Character(value.chars().next().unwrap()));
        }
        if let Some(number) = name.strip_prefix('#') {
            return Ok(ContentReference::Character(character_reference(number, self.version)?));
        }
        if !valid_name(name, self.version) {
            return Err(malformed("Invalid entity reference name"));
        }
        let (name, value) = self.entities.get_key_value(name)
            .ok_or_else(|| malformed("Undeclared entity"))?;
        match value {
            Entity::Internal(value) => Ok(ContentReference::Replacement { name, value }),
            Entity::ParsedExternal => Err(XmlError::ExternalResource),
            Entity::Unparsed(_) => Err(malformed("Unparsed entity cannot be expanded")),
        }
    }

    pub(super) fn text(
        &self,
        value: &str,
        budget: &mut ExpansionBudget,
    ) -> Result<String, XmlError> {
        self.expand(value, budget, false, false, &mut Vec::new())
    }

    fn expand<'a>(
        &'a self,
        raw: &str,
        budget: &mut ExpansionBudget,
        attribute: bool,
        generated: bool,
        trail: &mut Vec<&'a str>,
    ) -> Result<String, XmlError> {
        self.expand_with_parameter_depth(raw, budget, attribute, generated, trail, 0)
    }

    fn expand_with_parameter_depth<'a>(
        &'a self,
        raw: &str,
        budget: &mut ExpansionBudget,
        attribute: bool,
        generated: bool,
        trail: &mut Vec<&'a str>,
        parameter_depth: usize,
    ) -> Result<String, XmlError> {
        let mut output = String::new();
        let mut rest = raw;
        loop {
            let split = rest.find('&').unwrap_or(rest.len());
            let literal = &rest[..split];
            if attribute && literal.contains('<') {
                return Err(malformed("Literal less-than in attribute"));
            }
            let literal = if attribute {
                literal.replace(['\n', '\r', '\t'], " ")
            } else {
                literal.to_owned()
            };
            budget.append(&mut output, &literal, generated)?;
            if split == rest.len() {
                break;
            }
            rest = &rest[split + 1..];
            let end = rest
                .find(';')
                .ok_or_else(|| malformed("Unterminated entity reference"))?;
            let name = &rest[..end];
            budget.reference()?;
            if let Some(value) = predefined(name) {
                budget.append(&mut output, value, true)?;
            } else if let Some(number) = name.strip_prefix('#') {
                let value = character_reference(number, self.version)?;
                budget.append(&mut output, value.encode_utf8(&mut [0; 4]), true)?;
            } else {
                if !valid_name(name, self.version) {
                    return Err(malformed("Invalid entity reference name"));
                }
                let (name, value) = self.entities.get_key_value(name)
                    .ok_or_else(|| malformed("Undeclared entity"))?;
                let value = match value {
                    Entity::Internal(value) => value,
                    Entity::ParsedExternal => return Err(XmlError::ExternalResource),
                    Entity::Unparsed(_) => return Err(malformed("Unparsed entity cannot be expanded")),
                };
                enter_entity_at_depth(name, trail, parameter_depth)?;
                let expanded = self.expand_with_parameter_depth(value, budget, attribute, true, trail, parameter_depth);
                trail.pop();
                let expanded = expanded?;
                // Check stored replacement markup, not less-than text produced by escaped references
                if !attribute && value.contains('<') {
                    return Err(XmlError::UnsupportedDtd("entity markup in element text"));
                }
                budget.append(&mut output, &expanded, true)?;
            }
            rest = &rest[end + 1..];
        }
        Ok(output)
    }
}

fn normalize(kind: &AttributeType, value: String) -> String {
    if matches!(kind, AttributeType::Cdata) { value }
    else { value.split(' ').filter(|part| !part.is_empty()).collect::<Vec<_>>().join(" ") }
}

fn valid_attribute(kind: &AttributeType, value: &str, version: XmlVersion) -> bool {
    let token = |text: &str| !text.is_empty() && text.chars().all(|ch| name_char(ch, version));
    match kind {
        AttributeType::Cdata => true,
        AttributeType::Enum(values) | AttributeType::Notation(values) => values.contains(value),
        AttributeType::Id | AttributeType::IdRef | AttributeType::Entity => valid_name(value, version),
        AttributeType::IdRefs | AttributeType::Entities => !value.is_empty() && value.split(' ').all(|name| valid_name(name, version)),
        AttributeType::NmToken => token(value),
        AttributeType::NmTokens => !value.is_empty() && value.split(' ').all(token),
    }
}

pub(super) fn xml_space(ch: char) -> bool {
    matches!(ch, ' ' | '\t' | '\r' | '\n')
}
pub(super) fn valid_xml_char(ch: char) -> bool {
    matches!(ch as u32, 0x9 | 0xa | 0xd | 0x20..=0xd7ff | 0xe000..=0xfffd | 0x10000..=0x10ffff)
}
fn xml10_name_flag(ch: char, flag: u8) -> bool {
    XML10_NAME_FLAGS.get(ch as usize).is_some_and(|value| *value & flag != 0)
}
fn name_start(ch: char, version: XmlVersion) -> bool {
    if version == XmlVersion::Xml10 {
        return xml10_name_flag(ch, 1);
    }
    matches!(ch as u32,
        0x3a | 0x41..=0x5a | 0x5f | 0x61..=0x7a | 0xc0..=0xd6 | 0xd8..=0xf6
        | 0xf8..=0x2ff | 0x370..=0x37d | 0x37f..=0x1fff | 0x200c..=0x200d
        | 0x2070..=0x218f | 0x2c00..=0x2fef | 0x3001..=0xd7ff | 0xf900..=0xfdcf
        | 0xfdf0..=0xfffd | 0x10000..=0xeffff)
}
fn name_char(ch: char, version: XmlVersion) -> bool {
    if version == XmlVersion::Xml10 {
        return xml10_name_flag(ch, 2);
    }
    name_start(ch, version)
        || matches!(ch as u32, 0x2d | 0x2e | 0x30..=0x39 | 0xb7 | 0x300..=0x36f | 0x203f..=0x2040)
}
pub(super) fn valid_name(name: &str, version: XmlVersion) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|ch| name_start(ch, version)) && chars.all(|ch| name_char(ch, version))
}
fn character_reference(number: &str, version: XmlVersion) -> Result<char, XmlError> {
    let (digits, radix) = number
        .strip_prefix('x')
        .map_or((number, 10), |hex| (hex, 16));
    if digits.is_empty()
        || !digits.bytes().all(|byte| {
            if radix == 16 {
                byte.is_ascii_hexdigit()
            } else {
                byte.is_ascii_digit()
            }
        })
    {
        return Err(malformed("Invalid XML character reference"));
    }
    u32::from_str_radix(digits, radix)
        .ok()
        .and_then(char::from_u32)
        .filter(|ch| version.reference_character(*ch))
        .ok_or_else(|| malformed("Invalid XML character reference"))
}
fn validate_literal(raw: &str, entity: bool, version: XmlVersion) -> Result<(), XmlError> {
    if !entity && raw.contains('<') {
        return Err(malformed("Literal less-than in attribute default"));
    }
    if entity && raw.contains('%') {
        return Err(malformed("Parameter reference inside entity value"));
    }
    let mut rest = raw;
    while let Some(at) = rest.find('&') {
        rest = &rest[at + 1..];
        let end = rest
            .find(';')
            .ok_or_else(|| malformed("Unterminated entity reference"))?;
        let name = &rest[..end];
        if let Some(number) = name.strip_prefix('#') {
            character_reference(number, version)?;
        } else if !valid_name(name, version) {
            return Err(malformed("Invalid entity reference name"));
        }
        rest = &rest[end + 1..];
    }
    Ok(())
}
fn entity_literal(raw: &str, version: XmlVersion) -> Result<String, XmlError> {
    validate_literal(raw, true, version)?;
    let mut output = String::new();
    let mut rest = raw;
    while let Some(at) = rest.find('&') {
        output.push_str(&rest[..at]);
        rest = &rest[at + 1..];
        let end = rest.find(';').unwrap();
        if let Some(number) = rest[..end].strip_prefix('#') {
            output.push(character_reference(number, version)?);
        } else {
            output.push('&');
            output.push_str(&rest[..=end]);
        }
        rest = &rest[end + 1..];
    }
    output.push_str(rest);
    Ok(output)
}
fn predefined(name: &str) -> Option<&'static str> {
    match name {
        "amp" => Some("&"),
        "lt" => Some("<"),
        "gt" => Some(">"),
        "apos" => Some("'"),
        "quot" => Some("\""),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_expand_without_changing_character_reference_whitespace() {
        let dtd = Dtd::parse(
            "r [<!ELEMENT r EMPTY><!ENTITY n '2.5'><!ATTLIST r a CDATA '&n;' b CDATA '&#10;'>]",
        )
        .unwrap();
        let result = dtd
            .attributes("r", BTreeMap::new(), &mut ExpansionBudget::default())
            .unwrap();
        assert_eq!(result["a"], "2.5");
        assert_eq!(result["b"], "\n");
    }

    #[test]
    fn external_and_recursive_entities_never_resolve_resources() {
        for text in [
            "r SYSTEM 'owned.dtd'",
            "r [<!ENTITY % x SYSTEM 'owned.dtd'>%x;<!ELEMENT r EMPTY>]",
        ] {
            assert_eq!(Dtd::parse(text).unwrap_err(), XmlError::ExternalResource);
        }
        assert!(Dtd::parse("r [<!ELEMENT r EMPTY><!ENTITY % x SYSTEM 'owned.dtd'>]").is_ok());
        let external = Dtd::parse("r [<!ELEMENT r ANY><!ENTITY x SYSTEM 'owned.txt'>]").unwrap();
        assert_eq!(external.text("&x;", &mut ExpansionBudget::default()).unwrap_err(), XmlError::ExternalResource);
        let external = Dtd::parse("r [<!ELEMENT r EMPTY><!ENTITY x SYSTEM 'owned.txt'><!ATTLIST r a CDATA '&x;'>]").unwrap();
        assert_eq!(external.attributes("r", BTreeMap::new(), &mut ExpansionBudget::default()).unwrap_err(), XmlError::ExternalResource);
        let dtd = Dtd::parse(
            "r [<!ELEMENT r EMPTY><!ENTITY x '&y;'><!ENTITY y '&x;'><!ATTLIST r a CDATA '&x;'>]",
        )
        .unwrap();
        assert!(matches!(
            dtd.attributes("r", BTreeMap::new(), &mut ExpansionBudget::default()),
            Err(XmlError::Malformed(_))
        ));
    }

    #[test]
    fn entity_character_references_are_resolved_before_replacement() {
        let dtd = Dtd::parse("r [<!ELEMENT r EMPTY><!ENTITY n '2.5'><!ENTITY i '&#38;n;'><!ATTLIST r a CDATA '&i;'>]").unwrap();
        assert_eq!(
            dtd.attributes("r", BTreeMap::new(), &mut ExpansionBudget::default())
                .unwrap()["a"],
            "2.5"
        );
        for bad in ["&#0;", "&#x110000;", "&#X41;", "&#65", "&1bad;"] {
            assert!(
                Dtd::parse(&format!("r [<!ELEMENT r EMPTY><!ENTITY unused '{bad}'>]")).is_err()
            );
        }
    }

    #[test]
    fn required_fixed_enumerated_and_empty_content_constraints_remain_effective() {
        let dtd = Dtd::parse(
            "r [<!ELEMENT r EMPTY><!ATTLIST r a (yes|no) #FIXED 'yes' b CDATA #REQUIRED>]",
        )
        .unwrap();
        assert!(matches!(
            dtd.attributes("r", BTreeMap::new(), &mut ExpansionBudget::default()),
            Err(XmlError::MissingAttribute(_))
        ));
        let attributes = BTreeMap::from([
            ("a".to_owned(), "no".to_owned()),
            ("b".to_owned(), "1".to_owned()),
        ]);
        assert!(matches!(
            dtd.attributes("r", attributes, &mut ExpansionBudget::default()),
            Err(XmlError::InvalidAttribute(_))
        ));
        assert!(dtd.finish("r", 0, true, false, None).is_err());
    }

    #[test]
    fn declaration_spacing_preserves_required_and_forbidden_separators() {
        let bad = [
            "r [<!ELEMENT r EMPTY><!ATTLIST r a CDATA#REQUIRED>]",
            "r [<!ELEMENT r EMPTY><!ATTLIST r a (green|red)#REQUIRED>]",
            "r [<!ELEMENT r EMPTY><!ATTLIST r a CDATA'value'>]",
            "r [<!ELEMENT r EMPTY><!ATTLIST r a CDATA #FIXED'value'>]",
            "r [<!ELEMENT r EMPTY><!ENTITY x'value'>]",
            "r [<!ELEMENT r EMPTY><!ATTLIST r a(green|red) #IMPLIED>]",
            "r [<!ELEMENT r(leaf*)><!ELEMENT leaf EMPTY>]",
            "r [< !ELEMENT r EMPTY>]",
            "r [<! ELEMENT r EMPTY>]",
            "r [<!ELEMENT r EMPTY><!ATTLIST r a CDATA # REQUIRED>]",
            "r [<!ELEMENT r (leaf *)><!ELEMENT leaf EMPTY>]",
            "r [<!ELEMENT r (leaf) +><!ELEMENT leaf EMPTY>]",
        ];
        let accepted: Vec<_> = bad
            .iter()
            .copied()
            .filter(|text| Dtd::parse(text).is_ok())
            .collect();
        assert!(
            accepted.is_empty(),
            "Malformed declarations accepted: {accepted:?}"
        );
        for good in [
            "r [<!ELEMENT r EMPTY><!ATTLIST r a CDATA 'x'b CDATA 'y'>]",
            "r [<!ELEMENT r ( leaf+ ) ><!ELEMENT leaf EMPTY>]",
            "r [<!ELEMENT r EMPTY><!ATTLIST r a \t CDATA \n #FIXED 'x' >]",
            "r [<!ELEMENT r EMPTY><!ATTLIST r a ( green | red ) #IMPLIED >]",
            "r [<!ELEMENT r EMPTY><!-- between declarations --><!ENTITY \u{e9} '1.0'>]",
        ] {
            assert!(Dtd::parse(good).is_ok(), "{good}");
        }
    }

    #[test]
    fn names_reject_digit_starts_but_enumerations_allow_them() {
        assert!(Dtd::parse("r [<!ELEMENT r EMPTY><!ATTLIST r 1bad CDATA #IMPLIED>]").is_err());
        let dtd = Dtd::parse("r [<!ELEMENT r EMPTY><!ATTLIST r a (1|2) '1'>]").unwrap();
        assert_eq!(
            dtd.attributes("r", BTreeMap::new(), &mut ExpansionBudget::default())
                .unwrap()["a"],
            "1"
        );
    }

    #[test]
    fn empty_expansion_bombs_are_bounded_by_reference_work() {
        let mut source = "r [<!ELEMENT r EMPTY><!ENTITY e0 ''>".to_owned();
        for level in 1..=18 {
            source.push_str(&format!(
                "<!ENTITY e{level} '&e{};&e{};'>",
                level - 1,
                level - 1
            ));
        }
        source.push_str("<!ATTLIST r a CDATA '&e18;'>]");
        let dtd = Dtd::parse(&source).unwrap();
        assert!(matches!(
            dtd.attributes("r", BTreeMap::new(), &mut ExpansionBudget::default()),
            Err(XmlError::ResourceLimit { .. })
        ));
    }

    #[test]
    fn fixed_blocks_preserve_source_cardinality_and_existing_single_particles() {
        let models: [(&str, &[usize]); 15] = [
            ("(gene,gene)", &[2]),
            ("(gene,gene,gene)", &[3]),
            ("((gene,gene))", &[2]),
            ("(gene,gene)?", &[0, 2]),
            ("(gene,gene)*", &[0, 2, 4]),
            ("(gene,gene)+", &[2, 4]),
            ("(gene+)", &[1, 2, 3, 4]),
            ("(gene*)", &[0, 1, 2, 3, 4]),
            ("(gene?)", &[0, 1]),
            ("(gene)+", &[1, 2, 3, 4]),
            ("(gene)*", &[0, 1, 2, 3, 4]),
            ("(gene)?", &[0, 1]),
            ("(gene+)?", &[0, 1, 2, 3, 4]),
            ("(gene?)+", &[0, 1, 2, 3, 4]),
            ("(gene*)?", &[0, 1, 2, 3, 4]),
        ];
        for (model, allowed) in models {
            let dtd = Dtd::parse(&format!("r [<!ELEMENT r {model}><!ELEMENT gene EMPTY>]")).unwrap();
            assert!(dtd.child("r", "gene", &mut None, &mut ModelScratch::default()).is_ok());
            assert!(matches!(dtd.child("r", "other", &mut None, &mut ModelScratch::default()), Err(XmlError::Malformed(_))));
            for count in 0..=4 {
                assert_eq!(dtd.finish("r", count, false, false, None).is_ok(), allowed.contains(&count), "{model}: {count}");
            }
            assert!(matches!(dtd.finish("r", allowed[0], true, true, None), Err(XmlError::Malformed(_))));
        }
    }

    #[test]
    fn malformed_sequences_and_unsupported_particles_keep_typed_failures() {
        for model in [
            "(gene,)", "(,gene)", "(gene,,gene)", "((gene,gene)", "(gene,gene))", "()",
            "(gene,gene)**", "(gene,gene)?+", "(gene,gene){2}",
        ] {
            assert!(matches!(Dtd::parse(&format!("r [<!ELEMENT r {model}><!ELEMENT gene EMPTY>]")), Err(XmlError::Malformed(_))), "{model}");
        }
        for model in ["(gene|gene)", "(gene,other)", "(gene*,gene)", "(gene,gene?)", "((gene,gene)*)"] {
            assert!(Dtd::parse(&format!("r [<!ELEMENT r {model}><!ELEMENT gene EMPTY><!ELEMENT other EMPTY>]")).is_ok(), "{model}");
        }
    }

    #[test]
    fn content_model_nesting_and_token_work_stay_bounded() {
        let model = format!("{}gene{}", "(".repeat(super::super::MAX_DEPTH), ")".repeat(super::super::MAX_DEPTH));
        let dtd = Dtd::parse(&format!("r [<!ELEMENT r {model}><!ELEMENT gene EMPTY>]")).unwrap();
        assert!(dtd.finish("r", 1, false, false, None).is_ok());
        let model = format!("({model})");
        assert!(matches!(Dtd::parse(&format!("r [<!ELEMENT r {model}><!ELEMENT gene EMPTY>]")), Err(XmlError::ResourceLimit { resource: "DTD content-model depth", .. })));
        let model = vec!["gene"; MAX_TOKENS].join(",");
        assert!(matches!(Dtd::parse(&format!("r [<!ELEMENT r ({model})><!ELEMENT gene EMPTY>]")), Err(XmlError::ResourceLimit { resource: "DTD tokens", .. })));
    }

    #[test]
    fn mixed_models_validate_names_suffixes_and_declared_children() {
        for model in ["(#PCDATA)", "(#PCDATA)*", "(#PCDATA|leaf|unused)*"] {
            let dtd = Dtd::parse(&format!("r [<!ELEMENT r {model}><!ELEMENT leaf EMPTY>]")).unwrap();
            assert!(dtd.finish("r", 0, true, true, None).is_ok());
            assert_eq!(dtd.child("r", "leaf", &mut None, &mut ModelScratch::default()).is_ok(), model.contains("leaf"));
            assert!(matches!(dtd.child("r", "other", &mut None, &mut ModelScratch::default()), Err(XmlError::Malformed(_))));
            if model.contains("unused") {
                assert!(dtd.child("r", "unused", &mut None, &mut ModelScratch::default()).is_ok());
                assert!(matches!(dtd.attributes("unused", BTreeMap::new(), &mut ExpansionBudget::default()), Err(XmlError::Malformed(_))));
            }
        }
        for model in [
            "(#PCDATA|leaf)", "(#PCDATA|leaf) *", "(#PCDATA|leaf)**",
            "(#PCDATA)?", "(#PCDATA)+", "(#PCDATA|leaf|leaf)*",
            "(leaf|#PCDATA)*", "(#PCDATA|#PCDATA)*", "(#PCDATA,leaf)*",
            "(#PCDATA|(leaf))*", "(#PCDATA|leaf?)*", "(#PCDATA|)*", "(# PCDATA)",
        ] {
            assert!(matches!(Dtd::parse(&format!("r [<!ELEMENT r {model}><!ELEMENT leaf EMPTY>]")), Err(XmlError::Malformed(_))), "{model}");
        }
    }

    #[test]
    fn mixed_models_and_escaped_entities_keep_resource_bounds() {
        let names = (0..MAX_TOKENS).map(|index| format!("n{index}")).collect::<Vec<_>>().join("|");
        assert!(matches!(Dtd::parse(&format!("r [<!ELEMENT r (#PCDATA|{names})*>]")), Err(XmlError::ResourceLimit { resource: "DTD tokens", .. })));
        let dtd = Dtd::parse("r [<!ELEMENT r (#PCDATA)><!ENTITY e0 '&lt;'>]").unwrap();
        assert!(matches!(dtd.text(&"&e0;".repeat(MAX_REFERENCES + 1), &mut ExpansionBudget::default()), Err(XmlError::ResourceLimit { .. })));
    }

    #[test]
    fn escaped_text_entities_differ_from_stored_markup_replacements() {
        let dtd = Dtd::parse("r [<!ELEMENT r (#PCDATA)><!ENTITY safe '&lt;'><!ENTITY nested '&safe;'><!ENTITY numeric '&#60;'><!ENTITY literal '<'>]").unwrap();
        for text in ["&lt;", "&#60;", "&safe;", "&nested;"] {
            assert_eq!(dtd.text(text, &mut ExpansionBudget::default()).unwrap(), "<", "{text}");
        }
        for text in ["&numeric;", "&literal;"] {
            assert!(matches!(dtd.text(text, &mut ExpansionBudget::default()), Err(XmlError::UnsupportedDtd("entity markup in element text"))));
        }
    }

    #[test]
    fn declaration_defaults_and_applications_share_expansion_limits() {
        for fixed in [false, true] {
            let default = if fixed { "#FIXED '&n;'" } else { "'&n;'" };
            let source = format!("r [<!ELEMENT r EMPTY><!ENTITY n 'xy'><!ATTLIST r a NMTOKEN {default}>]");
            let raw = if fixed { BTreeMap::from([("a".to_owned(), "xy".to_owned())]) } else { BTreeMap::new() };
            let mut references = ExpansionBudget { bytes: 0, references: MAX_REFERENCES - 1 };
            let dtd = Dtd::parse_bounded(XmlVersion::Xml10, &source, &mut references, &mut RetainedPayloadBudget::default()).unwrap();
            assert!(matches!(dtd.attributes("r", raw.clone(), &mut references), Err(XmlError::ResourceLimit { resource: "entity references", .. })));
            let mut bytes = ExpansionBudget { bytes: MAX_EXPANDED_BYTES - 4, references: 0 };
            let dtd = Dtd::parse_bounded(XmlVersion::Xml10, &source, &mut bytes, &mut RetainedPayloadBudget::default()).unwrap();
            assert!(matches!(dtd.attributes("r", raw.clone(), &mut bytes), Err(XmlError::ResourceLimit { resource: "expanded entity bytes", .. })));
            let mut fresh = ExpansionBudget::default();
            let dtd = Dtd::parse_bounded(XmlVersion::Xml10, &source, &mut fresh, &mut RetainedPayloadBudget::default()).unwrap();
            assert_eq!(dtd.attributes("r", raw, &mut fresh).unwrap()["a"], "xy");
        }
    }


    #[test]
    fn entity_notation_declarations_validate_roles_defaults_and_first_bindings() {
        let catalog = "<!NOTATION media PUBLIC '-//BGR//EN'><!ENTITY first SYSTEM 'urn:first#part' NDATA media>";
        let source = format!("r [<!ELEMENT r ANY>{catalog}<!ATTLIST r a ENTITIES ''>]");
        let dtd = Dtd::parse(&source).unwrap();
        assert_eq!(dtd.attributes("r", BTreeMap::from([("a".into(), " first  first ".into())]), &mut ExpansionBudget::default()).unwrap()["a"], "first first");
        assert!(matches!(dtd.attributes("r", BTreeMap::new(), &mut ExpansionBudget::default()), Err(XmlError::Malformed(_))));
        let unused = format!("r [<!ELEMENT r ANY><!ELEMENT aux ANY>{catalog}<!ATTLIST aux a ENTITIES ''>]");
        assert!(Dtd::parse(&unused).is_ok());
        for declarations in [
            "<!ENTITY e SYSTEM 'urn:e' NDATA unknown>",
            "<!NOTATION p SYSTEM 'urn:p'><!NOTATION p SYSTEM 'urn:q'>",
            "<!ATTLIST aux a NOTATION (unknown) #IMPLIED>",
            "<!ATTLIST r a ENTITY '&later;'><!ENTITY later 'first'>",
            "<!NOTATION p PUBLIC '-//BGR\t//EN'>",
            "<!ENTITY e PUBLIC '-//BGR//EN'>",
        ] {
            assert!(matches!(Dtd::parse(&format!("r [<!ELEMENT r ANY><!ELEMENT aux ANY>{declarations}]")), Err(XmlError::Malformed(_))), "{declarations}");
        }
        for first_unparsed in [false, true] {
            let internal = "<!ENTITY first 'text'>";
            let declarations = if first_unparsed { format!("{catalog}{internal}") } else { format!("{internal}{catalog}") };
            let dtd = Dtd::parse(&format!("r [<!ELEMENT r ANY>{declarations}<!ATTLIST r a ENTITY #REQUIRED>]")).unwrap();
            let result = dtd.attributes("r", BTreeMap::from([("a".into(), "first".into())]), &mut ExpansionBudget::default());
            assert_eq!(result.is_ok(), first_unparsed);
        }
        let dtd = Dtd::parse("r [<!ELEMENT r ANY><!ENTITY words 'first'><!ENTITY words 'second'>]").unwrap();
        assert_eq!(dtd.text("&words;", &mut ExpansionBudget::default()).unwrap(), "first");
    }

    #[test]
    fn entity_notation_metadata_and_defaults_preserve_resource_bounds() {
        let source = "r [<!ELEMENT r ANY><!NOTATION p SYSTEM 'urn:p'><!ENTITY e SYSTEM 'urn:e' NDATA p><!ENTITY words 'e'><!ATTLIST r a ENTITIES '&words;' b NOTATION (p) #IMPLIED>]";
        let mut retained = RetainedPayloadBudget::default();
        Dtd::parse_bounded(XmlVersion::Xml10, source, &mut ExpansionBudget::default(), &mut retained).unwrap();
        assert!(retained.bytes >= "rpepewordseabpe&words;".len());
        let cost = retained.bytes;
        let mut exact = RetainedPayloadBudget { bytes: MAX_RETAINED_PAYLOAD_BYTES - cost };
        Dtd::parse_bounded(XmlVersion::Xml10, source, &mut ExpansionBudget::default(), &mut exact).unwrap();
        assert_eq!(exact.bytes, MAX_RETAINED_PAYLOAD_BYTES);
        let mut exceeded = RetainedPayloadBudget { bytes: MAX_RETAINED_PAYLOAD_BYTES - cost + 1 };
        assert!(matches!(Dtd::parse_bounded(XmlVersion::Xml10, source, &mut ExpansionBudget::default(), &mut exceeded), Err(XmlError::ResourceLimit { resource: "retained XML payload bytes", .. })));
        let mut budget = ExpansionBudget { bytes: 0, references: MAX_REFERENCES - 1 };
        let dtd = Dtd::parse_bounded(XmlVersion::Xml10, source, &mut budget, &mut RetainedPayloadBudget::default()).unwrap();
        assert!(matches!(dtd.attributes("r", BTreeMap::new(), &mut budget), Err(XmlError::ResourceLimit { resource: "entity references", .. })));
        let dtd = Dtd::parse(source).unwrap();
        assert_eq!(dtd.attributes("r", BTreeMap::new(), &mut ExpansionBudget::default()).unwrap()["a"], "e");
    }


    fn model_accepts(dtd: &Dtd, names: &[&str], scratch: &mut ModelScratch) -> bool {
        let mut cursor = dtd.cursor("r", scratch).unwrap();
        for name in names {
            if dtd.child("r", name, &mut cursor, scratch).is_err() { return false; }
        }
        dtd.finish("r", names.len(), false, false, cursor.as_ref()).is_ok()
    }

    #[test]
    fn general_models_match_ordered_ambiguous_and_undeclared_name_languages() {
        let cases: [(&str, &[&[&str]], &[&[&str]]); 8] = [
            ("((leaf,other)|(other,leaf))", &[&["leaf","other"], &["other","leaf"]], &[&[], &["leaf"], &["leaf","leaf"]]),
            ("(leaf,leaf?)", &[&["leaf"], &["leaf","leaf"]], &[&[], &["leaf","leaf","leaf"]]),
            ("(leaf|leaf)", &[&["leaf"]], &[&[], &["leaf","leaf"]]),
            ("(leaf?,leaf)", &[&["leaf"], &["leaf","leaf"]], &[&[], &["leaf","leaf","leaf"]]),
            ("(leaf*,leaf)", &[&["leaf"], &["leaf","leaf","leaf"]], &[&[], &["other"]]),
            ("(leaf+,leaf)", &[&["leaf","leaf"], &["leaf","leaf","leaf"]], &[&[], &["leaf"]]),
            ("((leaf?,leaf?)*)", &[&[], &["leaf"], &["leaf","leaf","leaf"]], &[&["other"]]),
            ("(leaf|ghost)*", &[&[], &["leaf"], &["ghost","leaf"]], &[&["other"]]),
        ];
        let mut scratch = ModelScratch::default();
        for (model, accepted, rejected) in cases {
            let dtd = Dtd::parse(&format!("r [<!ELEMENT r {model}><!ELEMENT leaf EMPTY><!ELEMENT other EMPTY>]")).unwrap();
            for names in accepted { assert!(model_accepts(&dtd, names, &mut scratch), "{model}: {names:?}"); }
            for names in rejected { assert!(!model_accepts(&dtd, names, &mut scratch), "{model}: {names:?}"); }
            assert!(matches!(dtd.attributes("ghost", BTreeMap::new(), &mut ExpansionBudget::default()), Err(XmlError::Malformed(_))));
        }
    }

    #[test]
    fn general_model_cursors_terminate_nullable_cycles_and_remain_independent() {
        let dtd = Dtd::parse("r [<!ELEMENT r ((leaf?)*)><!ELEMENT leaf EMPTY>]").unwrap();
        let mut scratch = ModelScratch::default();
        for count in 0..=64 {
            assert!(model_accepts(&dtd, &vec!["leaf"; count], &mut scratch));
            assert!(scratch.touched.is_empty() && scratch.pending.is_empty());
            assert!(scratch.marked.iter().all(|value| !*value));
        }
        let alternatives = (0..512).map(|index| format!("n{index}")).collect::<Vec<_>>().join("|");
        let dtd = Dtd::parse(&format!("r [<!ELEMENT r (leaf,({alternatives}))><!ELEMENT leaf EMPTY>]")).unwrap();
        let mut first = dtd.cursor("r", &mut scratch).unwrap();
        let mut second = dtd.cursor("r", &mut scratch).unwrap();
        assert_eq!(first.as_ref().unwrap().active.len(), 1);
        let initial = second.as_ref().unwrap().active.clone();
        dtd.child("r", "leaf", &mut first, &mut scratch).unwrap();
        assert_eq!(second.as_ref().unwrap().active, initial);
        assert_eq!(first.as_ref().unwrap().active.len(), 512);
        assert!(dtd.child("r", "wrong", &mut second, &mut scratch).is_err());
        dtd.child("r", "n123", &mut first, &mut scratch).unwrap();
        assert!(dtd.finish("r", 2, false, false, first.as_ref()).is_ok());
        assert!(model_accepts(&dtd, &["leaf","n0"], &mut scratch));
        assert!(scratch.touched.is_empty() && scratch.pending.is_empty());
    }

    #[test]
    fn general_models_bound_program_storage_and_retained_payload() {
        let mut states = ModelBudget { states: MAX_MODEL_STATES, edges: 0 };
        assert!(matches!(Program::compile(Particle::Name("leaf".into()), &mut states), Err(XmlError::ResourceLimit { resource: "DTD model states", .. })));
        let mut edges = ModelBudget { states: 0, edges: MAX_MODEL_EDGES };
        assert!(matches!(Program::compile(Particle::Name("leaf".into()), &mut edges), Err(XmlError::ResourceLimit { resource: "DTD model edges", .. })));
        let mut budget = ModelBudget::default();
        let program = Program::compile(Particle::Choice(vec![Particle::Name("leaf".into()), Particle::Name("other".into())]), &mut budget).unwrap();
        assert_eq!(budget.states, program.steps.len());
        assert_eq!(budget.edges, program.edges());
        let compact = "r [<!ELEMENT r (leaf,leaf)><!ELEMENT leaf EMPTY>]";
        let general = "r [<!ELEMENT r ((leaf,other)|(leaf,ghost))><!ELEMENT leaf EMPTY><!ELEMENT other EMPTY>]";
        for (source, payload) in [(compact, "rrleafleaf".len()), (general, "rrleafotherleafotherleafghost".len())] {
            let mut fresh = RetainedPayloadBudget::default();
            Dtd::parse_bounded(XmlVersion::Xml10, source, &mut ExpansionBudget::default(), &mut fresh).unwrap();
            assert_eq!(fresh.bytes, payload);
            let mut exact = RetainedPayloadBudget { bytes: MAX_RETAINED_PAYLOAD_BYTES - payload };
            Dtd::parse_bounded(XmlVersion::Xml10, source, &mut ExpansionBudget::default(), &mut exact).unwrap();
            assert_eq!(exact.bytes, MAX_RETAINED_PAYLOAD_BYTES);
            let mut excess = RetainedPayloadBudget { bytes: MAX_RETAINED_PAYLOAD_BYTES - payload + 1 };
            assert!(matches!(Dtd::parse_bounded(XmlVersion::Xml10, source, &mut ExpansionBudget::default(), &mut excess), Err(XmlError::ResourceLimit { resource: "retained XML payload bytes", .. })));
            assert!(Dtd::parse(source).is_ok());
        }
    }

    #[test]
    fn content_accounting_preserves_text_ancestry_and_shared_recursion() {
        let source = "r [<!ELEMENT r (#PCDATA)><!ENTITY leaf '&#xe9;&amp;'><!ENTITY outer 'a&leaf;b'><!ATTLIST r probe CDATA #IMPLIED>]";
        let dtd = Dtd::parse(source).unwrap();
        let mut scalar = ExpansionBudget::default();
        assert_eq!(dtd.text("&outer;", &mut scalar).unwrap(), "a\u{e9}&b");
        assert_eq!((scalar.bytes, scalar.references), (13, 3));
        let mut replay = ExpansionBudget::default();
        let mut trail = Vec::new();
        let ContentReference::Replacement { name, .. } = dtd.content_reference("outer", &mut replay).unwrap() else { panic!("Internal replacement required") };
        enter_entity(name, &mut trail).unwrap();
        replay.generated(1, trail.len() + 1).unwrap();
        let ContentReference::Replacement { name, .. } = dtd.content_reference("leaf", &mut replay).unwrap() else { panic!("Internal replacement required") };
        enter_entity(name, &mut trail).unwrap();
        replay.generated("\u{e9}".len(), trail.len() + 1).unwrap();
        let ContentReference::Character(ch) = dtd.content_reference("amp", &mut replay).unwrap() else { panic!("Character data required") };
        assert_eq!(ch, '&');
        replay.generated(ch.len_utf8(), trail.len() + 1).unwrap();
        trail.pop();
        replay.generated(1, trail.len() + 1).unwrap();
        assert_eq!((replay.bytes, replay.references), (13, 3));
        assert!(matches!(dtd.attributes_with_entities("r", BTreeMap::from([("probe".into(), "&outer;".into())]), &mut replay, &mut trail), Err(XmlError::Malformed(_))));
        assert_eq!(trail.as_slice(), &["outer"]);
        trail.pop();
        assert_eq!(dtd.attributes_with_entities("r", BTreeMap::from([("probe".into(), "&outer;".into())]), &mut ExpansionBudget::default(), &mut trail).unwrap()["probe"], "a\u{e9}&b");

        let mut exact = ExpansionBudget { bytes: 0, references: MAX_REFERENCES - 1 };
        assert!(matches!(dtd.content_reference("#65", &mut exact), Ok(ContentReference::Character('A'))));
        assert!(matches!(dtd.content_reference("#65", &mut exact), Err(XmlError::ResourceLimit { resource: "entity references", .. })));
        let declarations = (0..=MAX_ENTITY_DEPTH).map(|index| format!("<!ENTITY n{index} ''>")).collect::<Vec<_>>().join("");
        let catalog = Dtd::parse(&format!("r [<!ELEMENT r EMPTY>{declarations}]")).unwrap();
        let mut shared = Vec::new();
        for index in 0..MAX_ENTITY_DEPTH {
            let ContentReference::Replacement { name, .. } = catalog.content_reference(&format!("n{index}"), &mut ExpansionBudget::default()).unwrap() else { panic!("Internal replacement required") };
            enter_entity(name, &mut shared).unwrap();
        }
        let ContentReference::Replacement { name, .. } = catalog.content_reference(&format!("n{MAX_ENTITY_DEPTH}"), &mut ExpansionBudget::default()).unwrap() else { panic!("Internal replacement required") };
        assert!(matches!(enter_entity(name, &mut shared), Err(XmlError::ResourceLimit { resource: "entity recursion", maximum: MAX_ENTITY_DEPTH })));
        assert_eq!(shared.len(), MAX_ENTITY_DEPTH);
    }

    #[test]
    fn parameter_entities_preserve_declaration_frames_and_lexical_context() {
        let source = "r [<!ELEMENT r EMPTY><!ENTITY same 'value'><!ENTITY % same \"<!ATTLIST r a CDATA 'first'>\"><!ENTITY % same \"<!ATTLIST r a CDATA 'second'>\">%same;<!ATTLIST r b CDATA '&same;'>]";
        let dtd = Dtd::parse(source).unwrap();
        let values = dtd.attributes("r", BTreeMap::new(), &mut ExpansionBudget::default()).unwrap();
        assert_eq!(values["a"], "first");
        assert_eq!(values["b"], "value");
        let source = "r [<!ELEMENT r EMPTY><!ENTITY % inner \"<!ATTLIST r a NMTOKENS ' alpha  beta '>\"><!ENTITY % outer '&#37;inner;'>%outer;]";
        assert_eq!(Dtd::parse(source).unwrap().attributes("r", BTreeMap::new(), &mut ExpansionBudget::default()).unwrap()["a"], "alpha beta");
        assert!(Dtd::parse("r [<!ELEMENT r EMPTY><!ENTITY % unused '<!ELEMENT aux '>]").is_ok());
        assert!(Dtd::parse("r [<!ELEMENT r EMPTY><!-- %ghost; --><?owned %ghost;?>]").is_ok());
        let source = "r [<!ELEMENT r EMPTY><!ENTITY % gap '<!-- %ghost; --><?owned data?>'>]";
        assert!(matches!(Dtd::parse(source), Err(XmlError::Malformed(_))));
        for replacement in ["<!-->", "<!--->", "<?>", "<??>", "<?xml?>", "<!--x--->", "<!ELEMENT aux ", "]", "<!ELEMENT aux <?owned?>EMPTY>"] {
            let source = format!("r [<!ELEMENT r EMPTY><!ENTITY % piece '{replacement}'>%piece;]");
            assert!(matches!(Dtd::parse(&source), Err(XmlError::Malformed(_))), "{replacement}");
            assert!(Dtd::parse("r [<!ELEMENT r EMPTY><?owned data?>]").is_ok());
        }
        for source in [
            "r [<!ENTITY % model 'EMPTY'><!ELEMENT r %model;>]",
            "r [<!ENTITY % open '<!ELEMENT r '>%open;EMPTY>]",
            "r [<!ELEMENT r EMPTY><!ENTITY % inner ''><!ENTITY % outer '%inner;'>]",
            "r [<!ELEMENT r EMPTY><!ENTITY % loop '&#37;loop;'>%loop;]",
            "r [<!ELEMENT r EMPTY>%later;<!ENTITY % later ''>]",
            "r [<!ELEMENT r EMPTY><!ENTITY % gap ''>%gap]",
        ] {
            assert!(matches!(Dtd::parse(source), Err(XmlError::Malformed(_))), "{source}");
        }
        assert_eq!(Dtd::parse("r [<!ELEMENT r EMPTY><!ENTITY % x SYSTEM 'urn:denied'>%x;]").unwrap_err(), XmlError::ExternalResource);
    }

    #[test]
    fn parameter_entities_share_cumulative_budgets_and_recover() {
        let piece = "<!--x--><?p?>";
        let source = format!("r [<!ELEMENT r EMPTY><!ENTITY % p '{piece}'>%p;%p;]");
        let mut exact = ExpansionBudget { bytes: MAX_EXPANDED_BYTES - 2 * piece.len(), references: MAX_REFERENCES - 2 };
        Dtd::parse_bounded(XmlVersion::Xml10, &source, &mut exact, &mut RetainedPayloadBudget::default()).unwrap();
        assert_eq!(exact.bytes, MAX_EXPANDED_BYTES);
        assert_eq!(exact.references, MAX_REFERENCES);
        let mut bytes = ExpansionBudget { bytes: MAX_EXPANDED_BYTES - 2 * piece.len() + 1, references: 0 };
        assert!(matches!(Dtd::parse_bounded(XmlVersion::Xml10, &source, &mut bytes, &mut RetainedPayloadBudget::default()), Err(XmlError::ResourceLimit { resource: "expanded entity bytes", .. })));
        let mut references = ExpansionBudget { bytes: 0, references: MAX_REFERENCES - 1 };
        assert!(matches!(Dtd::parse_bounded(XmlVersion::Xml10, &source, &mut references, &mut RetainedPayloadBudget::default()), Err(XmlError::ResourceLimit { resource: "entity references", .. })));
        let payload = 2 + 3 * (1 + piece.len());
        let mut retained = RetainedPayloadBudget { bytes: MAX_RETAINED_PAYLOAD_BYTES - payload };
        Dtd::parse_bounded(XmlVersion::Xml10, &source, &mut ExpansionBudget::default(), &mut retained).unwrap();
        assert_eq!(retained.bytes, MAX_RETAINED_PAYLOAD_BYTES);
        let mut retained = RetainedPayloadBudget { bytes: MAX_RETAINED_PAYLOAD_BYTES - payload + 1 };
        assert!(matches!(Dtd::parse_bounded(XmlVersion::Xml10, &source, &mut ExpansionBudget::default(), &mut retained), Err(XmlError::ResourceLimit { resource: "retained XML payload bytes", .. })));

        // The fixed source has 26 tokens; each outer and inner reference adds six
        let repetitions = (MAX_TOKENS - 26) / 6;
        assert_eq!(26 + 6 * repetitions, MAX_TOKENS);
        let source = |count| format!("r [<!ELEMENT r EMPTY><!ENTITY % q ''><!ENTITY % p '&#37;q;'>%q;{}]", "%p;".repeat(count));
        assert!(Dtd::parse(&source(repetitions)).is_ok());
        assert!(matches!(Dtd::parse(&source(repetitions + 1)), Err(XmlError::ResourceLimit { resource: "DTD tokens", .. })));
        let source = |count| format!("r [<!ELEMENT r EMPTY><!ENTITY % p \"<!ENTITY same 'x'>\">{}]", "%p;".repeat(count));
        assert!(Dtd::parse(&source(MAX_DECLARATIONS - 2)).is_ok());
        assert!(matches!(Dtd::parse(&source(MAX_DECLARATIONS - 1)), Err(XmlError::ResourceLimit { resource: "DTD declarations", .. })));

        let piece = "<!ATTLIST r a CDATA '&g;'>";
        let source = format!("r [<!ELEMENT r EMPTY><!ENTITY g 'x'><!ENTITY % p \"{piece}\">%p;]");
        let mut references = ExpansionBudget { bytes: 0, references: MAX_REFERENCES - 1 };
        let dtd = Dtd::parse_bounded(XmlVersion::Xml10, &source, &mut references, &mut RetainedPayloadBudget::default()).unwrap();
        assert!(matches!(dtd.attributes("r", BTreeMap::new(), &mut references), Err(XmlError::ResourceLimit { resource: "entity references", .. })));
        let mut bytes = ExpansionBudget { bytes: MAX_EXPANDED_BYTES - piece.len(), references: 0 };
        let dtd = Dtd::parse_bounded(XmlVersion::Xml10, &source, &mut bytes, &mut RetainedPayloadBudget::default()).unwrap();
        assert!(matches!(dtd.attributes("r", BTreeMap::new(), &mut bytes), Err(XmlError::ResourceLimit { resource: "expanded entity bytes", .. })));
        let mut fresh = ExpansionBudget::default();
        let dtd = Dtd::parse_bounded(XmlVersion::Xml10, &source, &mut fresh, &mut RetainedPayloadBudget::default()).unwrap();
        assert_eq!(dtd.attributes("r", BTreeMap::new(), &mut fresh).unwrap()["a"], "x");
        assert_eq!(fresh.bytes, piece.len() + 2);
        assert_eq!(fresh.references, 2);
    }

    #[test]
    fn parameter_and_general_entities_share_invocation_depth() {
        fn source(parameters: usize, general: usize) -> String {
            let mut text = "r [<!ELEMENT r EMPTY><!NOTATION n SYSTEM 'urn:n'><!ENTITY item SYSTEM 'urn:item' NDATA n>".to_owned();
            for index in 0..general {
                let value = if index + 1 == general { "item".to_owned() } else { format!("&g{};", index + 1) };
                text.push_str(&format!("<!ENTITY g{index} '{value}'>"));
            }
            for index in (0..parameters).rev() {
                let value = if index + 1 == parameters {
                    let value = if general == 0 { "item" } else { "&g0;" };
                    format!("<!ATTLIST r probe ENTITY '{value}'>")
                } else { format!("&#37;p{};", index + 1) };
                text.push_str(&format!("<!ENTITY % p{index} \"{value}\">"));
            }
            text.push_str("%p0;]");
            text
        }
        let dtd = Dtd::parse(&source(MAX_ENTITY_DEPTH - 1, 1)).unwrap();
        assert_eq!(dtd.attributes("r", BTreeMap::new(), &mut ExpansionBudget::default()).unwrap()["probe"], "item");
        assert!(Dtd::parse(&source(MAX_ENTITY_DEPTH, 0)).is_ok());
        for (parameters, general) in [(MAX_ENTITY_DEPTH, 1), (MAX_ENTITY_DEPTH - 1, 2), (MAX_ENTITY_DEPTH + 1, 0)] {
            assert!(matches!(Dtd::parse(&source(parameters, general)), Err(XmlError::ResourceLimit { resource: "entity recursion", maximum: MAX_ENTITY_DEPTH })));
            assert!(Dtd::parse(&source(MAX_ENTITY_DEPTH - 1, 1)).is_ok());
        }
    }

}
