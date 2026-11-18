// Biogenesis (C) 2006-2010 Joan Queralt Molina; Color Mod (C) 2012-2022 MarcoDBAA
// SPDX-License-Identifier: GPL-2.0-or-later

//! Bounded detached GeneticCode/Gene[]/Gene conversion; actor and World publication stay separate
use crate::legacy_color::{ColorRoot, ColorStager, emit_color_documents_charged};
use crate::legacy_graph::{GraphError, GraphErrorKind, LegacyGraph};
use crate::object_stream::*;
use crate::organism::GenomeRef;
use crate::{
    Gene, GeneArrayRef, GeneRef, Genome, GenomeModifiers, GenomeProperties, JavaText,
    REACTION_NAMES,
};
use std::{cell::RefCell, collections::HashMap, mem::size_of, rc::Rc};

const CODE: &str = "biogenesis.GeneticCode";
const GENE: &str = "biogenesis.Gene";
const ARRAY: &str = "[Lbiogenesis.Gene;";
fn error(kind: GraphErrorKind) -> GraphError {
    GraphError {
        node: None,
        field: None,
        kind,
    }
}
fn product(a: usize, b: usize) -> Result<usize, GraphError> {
    a.checked_mul(b).ok_or(error(GraphErrorKind::Budget))
}

struct Budget {
    used: usize,
    maximum: usize,
    work: usize,
    work_limit: usize,
}
impl Budget {
    fn charge(&mut self, bytes: usize) -> Result<(), GraphError> {
        let next = self
            .used
            .checked_add(bytes)
            .ok_or(error(GraphErrorKind::Budget))?;
        if next > self.maximum {
            return Err(error(GraphErrorKind::Budget));
        }
        self.used = next;
        Ok(())
    }
    fn visit(&mut self, count: usize) -> Result<(), GraphError> {
        self.work = self
            .work
            .checked_add(count)
            .ok_or(error(GraphErrorKind::Budget))?;
        if self.work > self.work_limit {
            return Err(error(GraphErrorKind::Budget));
        }
        Ok(())
    }
    fn reserve<T>(&mut self, vec: &mut Vec<T>, count: usize) -> Result<(), GraphError> {
        let desired = vec
            .len()
            .checked_add(count)
            .ok_or(error(GraphErrorKind::Budget))?;
        if desired <= vec.capacity() {
            return Ok(());
        }
        let previous = vec.capacity();
        self.charge(product(desired - previous, size_of::<T>())?)?;
        vec.try_reserve_exact(count)
            .map_err(|_| error(GraphErrorKind::Allocation))?;
        self.charge(product(vec.capacity() - desired, size_of::<T>())?)
    }
    fn push<T>(&mut self, vec: &mut Vec<T>, value: T) -> Result<(), GraphError> {
        self.reserve(vec, 1)?;
        vec.push(value);
        Ok(())
    }
    fn reserve_index(&mut self, index: &mut HashMap<usize, usize>, count: usize) -> Result<(), GraphError> {
        let desired = index.len().checked_add(count).ok_or(error(GraphErrorKind::Budget))?;
        if desired <= index.capacity() { return Ok(()); }
        let previous = index.capacity();
        // Include bucket/control overhead in addition to the reported entry capacity
        let width = product(2, size_of::<(usize, usize)>())?;
        self.charge(product(desired - previous, width)?)?;
        self.visit(index.len())?;
        index.try_reserve(count).map_err(|_| error(GraphErrorKind::Allocation))?;
        self.charge(product(index.capacity() - desired, width)?)
    }
    fn index(&mut self, index: &mut HashMap<usize, usize>, key: usize, value: usize) -> Result<(), GraphError> {
        self.reserve_index(index, 1)?;
        self.visit(1)?;
        index.insert(key, value);
        Ok(())
    }
}

/// Per-Document identity tables own every staged target until the batch is published
pub(crate) struct GeneticsStager<'graph, 'document> {
    graph: &'graph LegacyGraph<'document>,
    codes: Vec<Option<GenomeRef>>,
    arrays: Vec<Option<GeneArrayRef>>,
    genes: Vec<Option<GeneRef>>,
    texts: Vec<Option<JavaText>>,
    colors: ColorStager<'graph, 'document>,
    budget: Budget,
    elements: usize,
}
impl<'g, 'd> GeneticsStager<'g, 'd> {
    pub(crate) fn new(
        graph: &'g LegacyGraph<'d>,
        additional_bytes: usize,
    ) -> Result<Self, GraphError> {
        let maximum = additional_bytes.min(graph.limits().allocated_bytes);
        let mut budget = Budget {
            used: 0,
            maximum,
            work: 0,
            work_limit: graph.limits().work,
        };
        let count = graph.arena().nodes.len();
        graph.charge(product(count, 4)?)?;
        graph.allocate(product(count,
            size_of::<Option<GenomeRef>>() + size_of::<Option<GeneArrayRef>>()
                + size_of::<Option<GeneRef>>() + size_of::<Option<JavaText>>()
        )?)?;
        let (mut codes, mut arrays, mut genes, mut texts) =
            (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        budget.reserve(&mut codes, count)?;
        codes.resize_with(count, || None);
        budget.reserve(&mut arrays, count)?;
        arrays.resize_with(count, || None);
        budget.reserve(&mut genes, count)?;
        genes.resize_with(count, || None);
        budget.reserve(&mut texts, count)?;
        texts.resize_with(count, || None);
        let colors = ColorStager::new(graph, maximum - budget.used)?;
        Ok(Self {
            graph,
            codes,
            arrays,
            genes,
            texts,
            colors,
            budget,
            elements: 0,
        })
    }
    pub(crate) fn uses_graph(&self, graph: &LegacyGraph<'_>) -> bool {
        std::ptr::eq(self.graph, graph)
    }

    pub(crate) fn allocated_bytes(&self) -> usize {
        self.budget.used + self.colors.allocated_bytes()
    }
    fn charge(&mut self, bytes: usize) -> Result<(), GraphError> {
        self.budget.maximum = self.budget.maximum.min(self.graph.limits().allocated_bytes);
        let total = self
            .allocated_bytes()
            .checked_add(bytes)
            .ok_or(error(GraphErrorKind::Budget))?;
        if total > self.budget.maximum {
            return Err(error(GraphErrorKind::Budget));
        }
        self.budget.charge(bytes)?;
        self.graph.allocate(bytes)
    }
    /// Use the same Color owner table for genotype, phenotype, and temporary holders
    pub(crate) fn color(&mut self, value: Value) -> Result<Option<crate::JavaColor>, GraphError> {
        self.colors
            .restrict_budget(self.budget.maximum - self.budget.used)?;
        self.colors.color(value)
    }
    pub(crate) fn color_array(
        &mut self,
        value: Value,
    ) -> Result<Option<crate::ColorArrayRef>, GraphError> {
        self.colors
            .restrict_budget(self.budget.maximum - self.budget.used)?;
        self.colors.colors(value)
    }
    pub(crate) fn genomes(
        &mut self,
        values: &[Value],
    ) -> Result<Vec<Option<GenomeRef>>, GraphError> {
        if values.len() > self.graph.limits().records {
            return Err(error(GraphErrorKind::Budget));
        }
        self.graph.charge(values.len())?;
        self.charge(product(values.len(), size_of::<Option<GenomeRef>>())?)?;
        let mut roots = Vec::new();
        roots
            .try_reserve_exact(values.len())
            .map_err(|_| error(GraphErrorKind::Allocation))?;
        self.charge(product(
            roots.capacity() - values.len(),
            size_of::<Option<GenomeRef>>(),
        )?)?;
        for value in values {
            roots.push(self.genome(*value)?);
        }
        Ok(roots)
    }
    pub(crate) fn object(&self, id: NodeId, class: &'static str) -> Result<(), GraphError> {
        self.graph.expect_class(id, class)?;
        let Node::Object(object) = self.graph.node(id)? else {
            return Err(error(GraphErrorKind::Type));
        };
        self.colors.validate_descriptor(object.descriptor)?;
        if self.graph.slots(id)?.iter().any(|s| !s.custom.is_empty()) {
            return Err(GraphError::at(id, None, GraphErrorKind::ContainerData));
        }
        Ok(())
    }
    pub(crate) fn text(&mut self, value: Value) -> Result<Option<JavaText>, GraphError> {
        self.graph.charge(1)?;
        let Value::Handle(id) = value else {
            return Ok(None);
        };
        self.graph.node(id)?;
        if let Some(v) = &self.texts[id.0 as usize] {
            return Ok(Some(v.clone()));
        }
        let units = self.graph.string_units(id)?;
        self.charge(
            product(units.len(), 4)?
                .checked_add(2 * size_of::<usize>())
                .ok_or(error(GraphErrorKind::Budget))?,
        )?;
        let mut copy = Vec::new();
        copy.try_reserve_exact(units.len())
            .map_err(|_| error(GraphErrorKind::Allocation))?;
        self.charge(product(copy.capacity() - units.len(), 2)?)?;
        copy.extend_from_slice(units);
        let value = JavaText::from_units(copy);
        self.texts[id.0 as usize] = Some(value.clone());
        Ok(Some(value))
    }
    pub(crate) fn gene(&mut self, value: Value) -> Result<Option<GeneRef>, GraphError> {
        self.graph.charge(1)?;
        let Value::Handle(id) = value else {
            return Ok(None);
        };
        self.graph.node(id)?;
        if let Some(v) = &self.genes[id.0 as usize] {
            return Ok(Some(Rc::clone(v)));
        }
        self.object(id, GENE)?;
        let length = f64::from_bits(self.graph.double_bits(id, GENE, "_length")?);
        let theta = f64::from_bits(self.graph.double_bits(id, GENE, "_theta")?);
        let branch = self.graph.int(id, GENE, "_branch")?;
        let mut reactions = [0; 36];
        for field in class_schema(class_by_name(GENE).unwrap()).fields {
            if let Some(index) = reaction_index(field.name) {
                reactions[index] = self.graph.int(id, GENE, field.name)?;
            }
        }
        self.charge(size_of::<RefCell<Gene>>() + 2 * size_of::<usize>())?;
        let color = self.color(self.graph.reference(id, GENE, "_color")?)?;
        let gene = Gene::new_with_color(length, theta, color, branch, reactions).shared();
        self.genes[id.0 as usize] = Some(Rc::clone(&gene));
        Ok(Some(gene))
    }
    pub(crate) fn array(&mut self, value: Value) -> Result<Option<GeneArrayRef>, GraphError> {
        self.graph.charge(1)?;
        let Value::Handle(id) = value else {
            return Ok(None);
        };
        self.graph.node(id)?;
        if let Some(v) = &self.arrays[id.0 as usize] {
            return Ok(Some(v.clone()));
        }
        self.graph.expect_class(id, ARRAY)?;
        let Node::Array(array) = self.graph.node(id)? else {
            return Err(error(GraphErrorKind::Type));
        };
        self.colors.validate_descriptor(array.descriptor)?;
        let ArrayData::References(values) = &array.data else {
            return Err(error(GraphErrorKind::Type));
        };
        let total = self
            .elements
            .checked_add(values.len())
            .ok_or(error(GraphErrorKind::Budget))?;
        if values.len() > self.graph.limits().array_elements
            || values.len() > i32::MAX as usize
            || total > self.graph.limits().total_array_elements
        {
            return Err(error(GraphErrorKind::Budget));
        }
        self.graph.count_array(values.len())?;
        self.colors.set_other_array_elements(total)?;
        self.elements = total;
        self.charge(
            product(values.len(), size_of::<Option<GeneRef>>())?
                .checked_add(size_of::<RefCell<Vec<Option<GeneRef>>>>() + 2 * size_of::<usize>())
                .ok_or(error(GraphErrorKind::Budget))?,
        )?;
        let mut slots = Vec::new();
        slots
            .try_reserve_exact(values.len())
            .map_err(|_| error(GraphErrorKind::Allocation))?;
        self.charge(product(
            slots.capacity() - values.len(),
            size_of::<Option<GeneRef>>(),
        )?)?;
        for value in values {
            slots.push(self.gene(*value)?);
        }
        let array = GeneArrayRef::from_slots(slots);
        self.arrays[id.0 as usize] = Some(array.clone());
        self.elements = total;
        Ok(Some(array))
    }
    pub(crate) fn genome(&mut self, value: Value) -> Result<Option<GenomeRef>, GraphError> {
        self.graph.charge(1)?;
        let Value::Handle(id) = value else {
            return Ok(None);
        };
        self.graph.node(id)?;
        if let Some(v) = &self.codes[id.0 as usize] {
            return Ok(Some(Rc::clone(v)));
        }
        self.object(id, CODE)?;
        let mut modifiers = GenomeModifiers::default();
        modifiers.cream = self.graph.int(id, CODE, "_modifiescream")?;
        modifiers.fallow = self.graph.int(id, CODE, "_modifiesfallow")?;
        modifiers.spore = self.graph.int(id, CODE, "_modifiesspore")?;
        modifiers.adapt_spore = self.graph.int(id, CODE, "_adaptspore")?;
        modifiers.plague = self.graph.boolean(id, CODE, "_plague")?;
        modifiers.disperse_children = self.graph.boolean(id, CODE, "_disperseChildren")?;
        modifiers.generation_battle = self.graph.boolean(id, CODE, "_generationBattle")?;
        modifiers.sibling_battle = self.graph.boolean(id, CODE, "_siblingBattle")?;
        modifiers.altruist = self.graph.boolean(id, CODE, "_altruist")?;
        modifiers.familial = self.graph.boolean(id, CODE, "_familial")?;
        modifiers.social = self.graph.boolean(id, CODE, "_social")?;
        modifiers.peaceful = self.graph.boolean(id, CODE, "_peaceful")?;
        modifiers.passive = self.graph.boolean(id, CODE, "_passive")?;
        modifiers.clockwise = self.graph.boolean(id, CODE, "_clockwise")?;
        modifiers.mimic_all = self.graph.boolean(id, CODE, "_mimicall")?;
        modifiers.modifies_pink = self.graph.boolean(id, CODE, "_modifiespink")?;
        modifiers.modifies_lilac = self.graph.boolean(id, CODE, "_modifieslilac")?;
        modifiers.modifies_sky = self.graph.boolean(id, CODE, "_modifiessky")?;
        modifiers.selfish = self.graph.boolean(id, CODE, "_selfish")?;
        let properties = GenomeProperties {
            symmetry: self.graph.int(id, CODE, "_symmetry")?,
            mirror: self.graph.int(id, CODE, "_mirror")?,
            mutation_rate: self.graph.int(id, CODE, "_mutationrate")?,
            activity: self.graph.int(id, CODE, "_activity")?,
            modifiers,
        };
        let generation = self.graph.int(id, CODE, "_generation")?;
        let update = self.graph.boolean(id, CODE, "_updateClade")?;
        let clade = self.text(self.graph.reference(id, CODE, "_cladeID")?)?;
        let array = self.array(self.graph.reference(id, CODE, "_genes")?)?;
        self.charge(size_of::<RefCell<Genome>>() + 2 * size_of::<usize>())?;
        let genome = Rc::new(RefCell::new(Genome::from_raw_array(
            array, properties, generation, clade, update,
        )));
        self.codes[id.0 as usize] = Some(Rc::clone(&genome));
        Ok(Some(genome))
    }
}

pub(crate) fn reaction_index(name: &str) -> Option<usize> {
    let name = name.strip_prefix('_')?.strip_suffix("reaction")?;
    REACTION_NAMES.iter().position(|value| *value == name)
}
pub(crate) fn code_scalar(code: &Genome, name: &str) -> Option<FieldValue> {
    let m = code.modifiers();
    Some(match name {
        "_symmetry" => FieldValue::Int(code.symmetry()),
        "_mirror" => FieldValue::Int(code.mirror()),
        "_mutationrate" => FieldValue::Int(code.mutation_rate()),
        "_activity" => FieldValue::Int(code.activity()),
        "_generation" => FieldValue::Int(code.generation()),
        "_updateClade" => FieldValue::Boolean(u8::from(code.update_clade())),
        "_modifiescream" => FieldValue::Int(m.cream),
        "_modifiesfallow" => FieldValue::Int(m.fallow),
        "_modifiesspore" => FieldValue::Int(m.spore),
        "_adaptspore" => FieldValue::Int(m.adapt_spore),
        "_plague" => FieldValue::Boolean(u8::from(m.plague)),
        "_disperseChildren" => FieldValue::Boolean(u8::from(m.disperse_children)),
        "_generationBattle" => FieldValue::Boolean(u8::from(m.generation_battle)),
        "_siblingBattle" => FieldValue::Boolean(u8::from(m.sibling_battle)),
        "_altruist" => FieldValue::Boolean(u8::from(m.altruist)),
        "_familial" => FieldValue::Boolean(u8::from(m.familial)),
        "_social" => FieldValue::Boolean(u8::from(m.social)),
        "_peaceful" => FieldValue::Boolean(u8::from(m.peaceful)),
        "_passive" => FieldValue::Boolean(u8::from(m.passive)),
        "_clockwise" => FieldValue::Boolean(u8::from(m.clockwise)),
        "_mimicall" => FieldValue::Boolean(u8::from(m.mimic_all)),
        "_modifiespink" => FieldValue::Boolean(u8::from(m.modifies_pink)),
        "_modifieslilac" => FieldValue::Boolean(u8::from(m.modifies_lilac)),
        "_modifiessky" => FieldValue::Boolean(u8::from(m.modifies_sky)),
        "_selfish" => FieldValue::Boolean(u8::from(m.selfish)),
        _ => return None,
    })
}

fn color_payload_elements(color: &crate::JavaColor) -> Result<usize, GraphError> {
    let mut count = 0usize;
    for array in [color.rgb_components(), color.components()]
        .into_iter()
        .flatten()
    {
        count = count
            .checked_add(array.bits().len())
            .ok_or(error(GraphErrorKind::Budget))?;
    }
    if let Some(space) = color.color_space() {
        for array in [
            space.diff_min_max(),
            space.inv_diff_min_max(),
            space.max_val(),
            space.min_val(),
        ]
        .into_iter()
        .flatten()
        {
            count = count
                .checked_add(array.bits().len())
                .ok_or(error(GraphErrorKind::Budget))?;
        }
    }
    Ok(count)
}

/// Export a batch of related GeneticCodes using one typed owner map per allocation class
/// All output is detached; no existing destination graph is modified on failure
pub(crate) fn export_genomes(
    roots: &[Option<&GenomeRef>],
    limits: Limits,
    additional_bytes: usize,
) -> Result<Document, GraphError> {
    export_genome_color_roots(roots, &[], limits, additional_bytes)
}

/// Additional Color holders preserve aliases across actor roles in this same detached batch
pub(crate) fn export_genome_color_roots(
    roots: &[Option<&GenomeRef>],
    extra_colors: &[ColorRoot<'_>],
    limits: Limits,
    additional_bytes: usize,
) -> Result<Document, GraphError> {
    let mut budget = Budget {
        used: 0,
        maximum: additional_bytes.min(limits.allocated_bytes),
        work: 0,
        work_limit: limits.work,
    };
    let record_count = roots
        .len()
        .checked_add(extra_colors.len())
        .ok_or(error(GraphErrorKind::Budget))?;
    if record_count > limits.records {
        return Err(error(GraphErrorKind::Budget));
    }
    budget.visit(
        extra_colors
            .len()
            .checked_mul(3)
            .ok_or(error(GraphErrorKind::Budget))?,
    )?;
    for root in extra_colors {
        match *root {
            ColorRoot::Colors(Some(array)) => {
                // Count the wrapper pass and all three exporter passes, including null slots
                budget.visit(product(array.len(), 4)?)?;
                for color in array.slots().iter().flatten() {
                    budget.visit(color_payload_elements(color)?)?;
                }
            }
            ColorRoot::Color(Some(color)) => budget.visit(color_payload_elements(color)?)?,
            ColorRoot::Floats(Some(array)) => budget.visit(array.bits().len())?,
            _ => {}
        }
    }

    let (mut codes, mut arrays, mut genes) = (
        Vec::<GenomeRef>::new(),
        Vec::<GeneArrayRef>::new(),
        Vec::<GeneRef>::new(),
    );
    let (mut code_index, mut array_index, mut gene_index) =
        (HashMap::new(), HashMap::new(), HashMap::new());
    for root in roots {
        budget.visit(1)?;
        if let Some(root) = root {
            budget.visit(1)?;
            let key = Rc::as_ptr(root) as usize;
            if !code_index.contains_key(&key) {
                budget.index(&mut code_index, key, codes.len())?;
                budget.push(&mut codes, Rc::clone(root))?;
            }
        }
    }
    for code in &codes {
        budget.visit(1)?;
        if let Some(array) = code.borrow().gene_array() {
            budget.visit(1)?;
            let key = array.identity();
            if !array_index.contains_key(&key) {
                budget.index(&mut array_index, key, arrays.len())?;
                budget.push(&mut arrays, array.clone())?;
            }
        }
    }
    let mut elements = 0usize;
    for array in &arrays {
        elements = elements
            .checked_add(array.len())
            .ok_or(error(GraphErrorKind::Budget))?;
        if array.len() > limits.array_elements
            || array.len() > i32::MAX as usize
            || elements > limits.total_array_elements
        {
            return Err(error(GraphErrorKind::Budget));
        }
        for gene in array.slots().iter() {
            budget.visit(1)?;
            if let Some(gene) = gene {
                budget.visit(1)?;
                let key = Rc::as_ptr(gene) as usize;
                if !gene_index.contains_key(&key) {
                    budget.index(&mut gene_index, key, genes.len())?;
                    budget.push(&mut genes, Rc::clone(gene))?;
                }
            }
        }
    }
    // Reserve the complete new closure before invoking the separately bounded Color exporter
    let mut code_ids = Vec::new();
    budget.reserve(&mut code_ids, codes.len())?;
    let mut array_ids = Vec::new();
    budget.reserve(&mut array_ids, arrays.len())?;
    let mut gene_ids = Vec::new();
    budget.reserve(&mut gene_ids, genes.len())?;
    let mut colors = Vec::new();
    budget.reserve(&mut colors, genes.len())?;
    for gene in &genes {
        let gene = gene.borrow();
        if let Some(color) = gene.color_handle() {
            budget.visit(color_payload_elements(color)?)?;
        }
        colors.push(gene.color_handle().cloned());
    }
    let mut color_roots = Vec::new();
    budget.reserve(
        &mut color_roots,
        colors
            .len()
            .checked_add(extra_colors.len())
            .ok_or(error(GraphErrorKind::Budget))?,
    )?;
    for color in &colors {
        color_roots.push(ColorRoot::Color(color.as_ref()));
    }
    color_roots.extend_from_slice(extra_colors);
    let mut output_records = Vec::new();
    budget.reserve(&mut output_records, record_count)?;
    let mut texts = Vec::<(JavaText, NodeId)>::new();
    budget.reserve(&mut texts, codes.len())?;
    let mut text_index = HashMap::new();
    budget.reserve_index(&mut text_index, codes.len())?;
    let mut descriptors = Vec::<(&'static str, NodeId)>::new();
    budget.reserve(&mut descriptors, 3)?;
    let mut signatures = Vec::<(&'static str, NodeId)>::new();
    budget.reserve(&mut signatures, 3)?;
    // New owner nodes and signature/clade strings remain live alongside the Color document
    let new_nodes = codes
        .len()
        .checked_add(arrays.len())
        .and_then(|v| v.checked_add(genes.len()))
        .and_then(|v| v.checked_add(codes.len()))
        .and_then(|v| v.checked_add(6))
        .ok_or(error(GraphErrorKind::Budget))?;
    budget.charge(product(new_nodes, size_of::<Node>())?)?;
    budget.charge(product(codes.len() + genes.len(), size_of::<ClassData>())?)?;
    budget.charge(product(codes.len(), 27 * size_of::<FieldValue>())?)?;
    budget.charge(product(genes.len(), 40 * size_of::<FieldValue>())?)?;
    budget.charge(product(elements, size_of::<Value>())?)?;
    budget.charge(product(69, size_of::<Option<NodeId>>())?)?;
    for code in &codes {
        if let Some(text) = code.borrow().clade_id() {
            let length = text.units().iter().try_fold(0usize, |count, unit| {
                count
                    .checked_add(if (1..=127).contains(unit) {
                        1
                    } else if *unit <= 0x7ff {
                        2
                    } else {
                        3
                    })
                    .ok_or(error(GraphErrorKind::Budget))
            })?;
            if text.units().len() > limits.string_units || length > limits.string_bytes {
                return Err(error(GraphErrorKind::Budget));
            }
            budget.charge(product(text.units().len(), 2)?)?;
            budget.visit(product(text.units().len(), 2)?)?;
        }
    }
    for text in [
        "Ljava/awt/Color;",
        "Ljava/lang/String;",
        "[Lbiogenesis/Gene;",
    ] {
        budget.charge(product(text.len(), 2)?)?;
    }
    budget.visit(product(
        codes.len() + genes.len() + elements + roots.len() + 6,
        80,
    )?)?;
    let mut color_values = Vec::new();
    budget.reserve(&mut color_values, color_roots.len())?;
    // Charge each emission lookup before entering the separately bounded Color exporter
    budget.visit(elements)?;
    budget.visit(roots.len())?;
    budget.visit(product(codes.len(), 3)?)?;
    budget.visit(product(genes.len(), 40 * 36)?)?;
    budget.visit(limits.handles)?;
    let mut color_limits = limits;
    color_limits.total_array_elements = color_limits
        .total_array_elements
        .checked_sub(elements)
        .ok_or(error(GraphErrorKind::Budget))?;
    color_limits.work = color_limits
        .work
        .checked_sub(budget.work)
        .ok_or(error(GraphErrorKind::Budget))?;
    let (mut document, color_bytes) =
        emit_color_documents_charged(&color_roots, color_limits, budget.maximum - budget.used)?;
    budget.maximum -= color_bytes;
    color_values.extend(document.records.iter().map(|v| match v {
        Record::Value(v) => *v,
        _ => unreachable!(),
    }));
    let base_capacity = document.arena.nodes.capacity();
    document
        .arena
        .nodes
        .try_reserve_exact(new_nodes)
        .map_err(|_| error(GraphErrorKind::Allocation))?;
    if document.arena.nodes.capacity() > base_capacity + new_nodes {
        budget.charge(product(
            document.arena.nodes.capacity() - base_capacity - new_nodes,
            size_of::<Node>(),
        )?)?;
    }
    if document
        .arena
        .nodes
        .len()
        .checked_add(new_nodes)
        .is_none_or(|v| v > limits.handles || v > u32::MAX as usize)
    {
        return Err(error(GraphErrorKind::Budget));
    }
    let mut emitter = Emitter {
        document,
        descriptors,
        signatures,
        limits,
        budget,
    };
    let gene_desc = emitter.descriptor(GENE)?;
    let array_desc = emitter.descriptor(ARRAY)?;
    let code_desc = emitter.descriptor(CODE)?;
    for (index, gene) in genes.iter().enumerate() {
        let gene = gene.borrow();
        let mut values = Vec::new();
        values
            .try_reserve_exact(40)
            .map_err(|_| error(GraphErrorKind::Allocation))?;
        emitter
            .budget
            .charge(product(values.capacity() - 40, size_of::<FieldValue>())?)?;
        for field in class_schema(class_by_name(GENE).unwrap()).fields {
            values.push(match field.name {
                "_length" => FieldValue::DoubleBits(gene.length().to_bits()),
                "_theta" => FieldValue::DoubleBits(gene.theta().to_bits()),
                "_branch" => FieldValue::Int(gene.branch()),
                "_color" => FieldValue::Reference(color_values[index]),
                name => FieldValue::Int(
                    gene.reaction(reaction_index(name).ok_or(error(GraphErrorKind::Field))?),
                ),
            });
        }
        let mut slots = Vec::new();
        slots
            .try_reserve_exact(1)
            .map_err(|_| error(GraphErrorKind::Allocation))?;
        emitter
            .budget
            .charge(product(slots.capacity() - 1, size_of::<ClassData>())?)?;
        slots.push(ClassData {
            descriptor: gene_desc,
            values,
            custom: Vec::new(),
        });
        gene_ids.push(emitter.push(Node::Object(JavaObject {
            descriptor: gene_desc,
            slots,
        }))?);
    }
    for array in &arrays {
        let mut values = Vec::new();
        values
            .try_reserve_exact(array.len())
            .map_err(|_| error(GraphErrorKind::Allocation))?;
        emitter.budget.charge(product(
            values.capacity() - array.len(),
            size_of::<Value>(),
        )?)?;
        for gene in array.slots().iter() {
            values.push(gene.as_ref().map_or(Value::Null, |gene| {
                Value::Handle(
                    gene_ids[*gene_index.get(&(Rc::as_ptr(gene) as usize))
                        .expect("Collected Gene")],
                )
            }));
        }
        array_ids.push(emitter.push(Node::Array(JavaArray {
            descriptor: array_desc,
            data: ArrayData::References(values),
        }))?);
    }
    for code in &codes {
        let code = code.borrow();
        let clade = if let Some(text) = code.clade_id() {
            if let Some(index) = text_index.get(&text.identity()) {
                Value::Handle(texts[*index].1)
            } else {
                let mut units = Vec::new();
                units
                    .try_reserve_exact(text.units().len())
                    .map_err(|_| error(GraphErrorKind::Allocation))?;
                emitter.budget.charge(product(
                    units.capacity() - text.units().len(),
                    size_of::<u16>(),
                )?)?;
                units.extend_from_slice(text.units());
                let bytes = units.iter().try_fold(0usize, |count, unit| {
                    count
                        .checked_add(if (1..=127).contains(unit) {
                            1
                        } else if *unit <= 0x7ff {
                            2
                        } else {
                            3
                        })
                        .ok_or(error(GraphErrorKind::Budget))
                })?;
                if units.len() > limits.string_units || bytes > limits.string_bytes {
                    return Err(error(GraphErrorKind::Budget));
                }
                let id = emitter.push(Node::String(JavaString {
                    units,
                    long: bytes > u16::MAX as usize,
                }))?;
                text_index.insert(text.identity(), texts.len());
                texts.push((text.clone(), id));
                Value::Handle(id)
            }
        } else {
            Value::Null
        };
        let array = code.gene_array().map_or(Value::Null, |array| {
            Value::Handle(
                array_ids[*array_index.get(&array.identity())
                    .expect("Collected array")],
            )
        });
        let mut values = Vec::new();
        values
            .try_reserve_exact(27)
            .map_err(|_| error(GraphErrorKind::Allocation))?;
        emitter
            .budget
            .charge(product(values.capacity() - 27, size_of::<FieldValue>())?)?;
        for field in class_schema(class_by_name(CODE).unwrap()).fields {
            values.push(match field.name {
                "_genes" => FieldValue::Reference(array),
                "_cladeID" => FieldValue::Reference(clade),
                name => code_scalar(&code, name).ok_or(error(GraphErrorKind::Field))?,
            });
        }
        let mut slots = Vec::new();
        slots
            .try_reserve_exact(1)
            .map_err(|_| error(GraphErrorKind::Allocation))?;
        emitter
            .budget
            .charge(product(slots.capacity() - 1, size_of::<ClassData>())?)?;
        slots.push(ClassData {
            descriptor: code_desc,
            values,
            custom: Vec::new(),
        });
        code_ids.push(emitter.push(Node::Object(JavaObject {
            descriptor: code_desc,
            slots,
        }))?);
    }
    for root in roots {
        output_records.push(Record::Value(root.map_or(Value::Null, |root| {
            Value::Handle(
                code_ids[*code_index.get(&(Rc::as_ptr(root) as usize))
                    .expect("Collected GeneticCode")],
            )
        })));
    }
    output_records.extend(
        color_values[genes.len()..]
            .iter()
            .map(|value| Record::Value(*value)),
    );
    emitter.document.records = output_records;
    let total_arrays = emitter
        .document
        .arena
        .nodes
        .iter()
        .try_fold(0usize, |total, node| {
            let length = match node {
                Node::Array(v) => match &v.data {
                    ArrayData::Ints(v) => v.len(),
                    ArrayData::Doubles(v) => v.len(),
                    ArrayData::Floats(v) => v.len(),
                    ArrayData::References(v) => v.len(),
                },
                _ => 0,
            };
            total
                .checked_add(length)
                .ok_or(error(GraphErrorKind::Budget))
        })?;
    if total_arrays > limits.total_array_elements {
        return Err(error(GraphErrorKind::Budget));
    }
    Ok(emitter.document)
}
struct Emitter {
    document: Document,
    descriptors: Vec<(&'static str, NodeId)>,
    signatures: Vec<(&'static str, NodeId)>,
    limits: Limits,
    budget: Budget,
}
impl Emitter {
    fn push(&mut self, node: Node) -> Result<NodeId, GraphError> {
        if self.document.arena.nodes.len() >= self.limits.handles {
            return Err(error(GraphErrorKind::Budget));
        }
        let id = NodeId(
            u32::try_from(self.document.arena.nodes.len())
                .map_err(|_| error(GraphErrorKind::Budget))?,
        );
        self.document.arena.nodes.push(node);
        Ok(id)
    }
    fn descriptor(&mut self, name: &'static str) -> Result<NodeId, GraphError> {
        if let Some((_, id)) = self.descriptors.iter().find(|(v, _)| *v == name) {
            return Ok(*id);
        }
        let class = class_by_name(name).ok_or(error(GraphErrorKind::Class))?;
        let schema = class_schema(class);
        if name.len() > self.limits.name_units
            || schema
                .fields
                .iter()
                .any(|field| field.name.len() > self.limits.name_units)
        {
            return Err(error(GraphErrorKind::Budget));
        }
        let mut values = Vec::new();
        values
            .try_reserve_exact(schema.fields.len())
            .map_err(|_| error(GraphErrorKind::Allocation))?;
        self.budget.charge(product(
            values.capacity() - schema.fields.len(),
            size_of::<Option<NodeId>>(),
        )?)?;
        for field in schema.fields {
            values.push(if let Some(text) = field.signature {
                if let Some((_, id)) = self.signatures.iter().find(|(v, _)| *v == text) {
                    Some(*id)
                } else {
                    if text.len() > self.limits.string_bytes
                        || text.encode_utf16().count() > self.limits.string_units
                    {
                        return Err(error(GraphErrorKind::Budget));
                    }
                    let mut units = Vec::new();
                    units
                        .try_reserve_exact(text.len())
                        .map_err(|_| error(GraphErrorKind::Allocation))?;
                    self.budget
                        .charge(product(units.capacity() - text.len(), size_of::<u16>())?)?;
                    units.extend(text.encode_utf16());
                    let id = self.push(Node::String(JavaString { units, long: false }))?;
                    self.signatures.push((text, id));
                    Some(id)
                }
            } else {
                None
            });
        }
        let id = self.push(Node::Descriptor(ClassDescriptor {
            class,
            signatures: values,
            annotations: Vec::new(),
            superclass: None,
        }))?;
        self.descriptors.push((name, id));
        Ok(id)
    }
}

#[cfg(test)]
#[path = "genetics_export_lookup_tests.rs"]
mod export_lookup_tests;
