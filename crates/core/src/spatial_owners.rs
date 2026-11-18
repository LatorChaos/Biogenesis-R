// SPDX-License-Identifier: GPL-2.0-or-later

//! Live STRtree operations over the same owners reached by saved histories

use crate::shared_world_owners::{BoundableRef, BoundableValue, EnvelopeRef, ListAccessError, SharedListRef, SharedSlot, TreeRef};
use super::Envelope;
use std::{cmp::Ordering, fmt};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpatialAccessError {
    List { operation: &'static str, error: ListAccessError },
    MissingBounds { operation: &'static str },
    MissingRoot,
    Capacity { operation: &'static str },
    Built,
}
impl fmt::Display for SpatialAccessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::List { operation, error } => write!(f, "Spatial {operation}: {error}"),
            Self::MissingBounds { operation } => write!(f, "Spatial {operation} reached null bounds"),
            Self::MissingRoot => f.write_str("Spatial query reached a null root"),
            Self::Capacity { operation } => write!(f, "Spatial {operation} cannot allocate"),
            Self::Built => f.write_str("Cannot insert into a built STRtree"),
        }
    }
}
impl std::error::Error for SpatialAccessError {}
type Result<T> = std::result::Result<T, SpatialAccessError>;
fn list_error(operation: &'static str, error: ListAccessError) -> SpatialAccessError { SpatialAccessError::List { operation, error } }
fn required(value: Option<SharedListRef>, field: &'static str, operation: &'static str) -> Result<SharedListRef> {
    value.ok_or_else(|| list_error(operation, ListAccessError::MissingList { field }))
}
fn boundable(slot: SharedSlot, field: &'static str, index: usize, operation: &'static str) -> Result<BoundableRef> {
    match slot {
        SharedSlot::Boundable(value) => Ok(value),
        SharedSlot::Null => Err(list_error(operation, ListAccessError::MissingElement { field, index })),
        value => Err(list_error(operation, ListAccessError::WrongClass { field, index, expected: "org.locationtech.jts.index.strtree.Boundable", actual: value.class_name() })),
    }
}
fn reserve<T>(values: &mut Vec<T>, count: usize, operation: &'static str) -> Result<()> {
    values.try_reserve(count).map_err(|_| SpatialAccessError::Capacity { operation })
}
fn node(level: i32, children: Vec<SharedSlot>) -> BoundableRef {
    BoundableRef::new(BoundableValue::Node { level, bounds: None, children: Some(SharedListRef::from_slots(children)) })
}

pub(super) fn bounds(value: &BoundableRef) -> Result<Option<EnvelopeRef>> {
    let children = match &*value.value() {
        BoundableValue::Item { bounds, .. } => return Ok(bounds.clone()),
        BoundableValue::Node { bounds: Some(bounds), .. } => return Ok(Some(bounds.clone())),
        BoundableValue::Node { children, .. } => children.clone(),
    };
    let children = required(children, "childBoundables", "computeBounds")?;
    let mut accumulated: Option<Envelope> = None;
    for index in 0..children.len() {
        let child = boundable(children.slot("childBoundables", index).map_err(|e| list_error("computeBounds", e))?, "childBoundables", index, "computeBounds")?;
        let child = bounds(&child)?.ok_or(SpatialAccessError::MissingBounds { operation: "computeBounds" })?.get();
        if let Some(envelope) = &mut accumulated { envelope.include(child); } else { accumulated = Some(child); }
    }
    let result = accumulated.map(EnvelopeRef::new);
    value.set_bounds(result.clone());
    Ok(result)
}
fn compare(left: &SharedSlot, right: &SharedSlot, horizontal: bool) -> Result<Ordering> {
    let left = boundable(left.clone(), "itemBoundables", 0, "compare")?;
    let a = bounds(&left)?.ok_or(SpatialAccessError::MissingBounds { operation: "compare" })?.get();
    let a = if horizontal { (a.min_x + a.max_x) / 2.0 } else { (a.min_y + a.max_y) / 2.0 };
    let right = boundable(right.clone(), "itemBoundables", 0, "compare")?;
    let b = bounds(&right)?.ok_or(SpatialAccessError::MissingBounds { operation: "compare" })?.get();
    let b = if horizontal { (b.min_x + b.max_x) / 2.0 } else { (b.min_y + b.max_y) / 2.0 };
    Ok(if a > b { Ordering::Greater } else if a < b { Ordering::Less } else { Ordering::Equal })
}
fn sort(values: &mut [SharedSlot], horizontal: bool) -> Result<()> {
    if values.len() > 1 {
        let mut keyed = Vec::new();
        reserve(&mut keyed, values.len(), "sort")?;
        // Node comparisons can publish bounds or fail in source order
        let finite_items = values.iter().all(|slot| {
            let SharedSlot::Boundable(value) = slot else { return false };
            let envelope = match &*value.value() {
                BoundableValue::Item { bounds: Some(bounds), .. } => bounds.get(),
                _ => return false,
            };
            let center = if horizontal {
                (envelope.min_x + envelope.max_x) / 2.0
            } else {
                (envelope.min_y + envelope.max_y) / 2.0
            };
            if !center.is_finite() { return false; }
            keyed.push((center, slot.clone()));
            true
        });
        if finite_items {
            keyed.sort_by(|left, right| left.0.partial_cmp(&right.0).expect("Finite sort centers"));
            for (destination, (_, value)) in values.iter_mut().zip(keyed) { *destination = value; }
            return Ok(());
        }
    }
    let mut failure = None;
    values.sort_by(|left, right| {
        if failure.is_some() { return Ordering::Equal; }
        match compare(left, right, horizontal) { Ok(order) => order, Err(error) => { failure = Some(error); Ordering::Equal } }
    });
    failure.map_or(Ok(()), Err)
}
fn parents(mut children: Vec<SharedSlot>, level: i32, capacity: usize) -> Result<Vec<SharedSlot>> {
    let minimum_leaves = children.len().div_ceil(capacity);
    let slices = (minimum_leaves as f64).sqrt().ceil() as usize;
    let slice_capacity = children.len().div_ceil(slices);
    sort(&mut children, true)?;
    let mut input = children.into_iter();
    let mut vertical_slices = Vec::new(); reserve(&mut vertical_slices, slices, "verticalSlices")?;
    for _ in 0..slices {
        let mut slice = Vec::new(); reserve(&mut slice, slice_capacity, "verticalSlices")?;
        for (index, value) in input.by_ref().take(slice_capacity).enumerate() {
            boundable(value.clone(), "itemBoundables", index, "verticalSlices")?;
            slice.push(value);
        }
        vertical_slices.push(slice);
    }
    let mut result = Vec::new();
    for mut slice in vertical_slices {
        sort(&mut slice, false)?;
        let mut slice = slice.into_iter();
        loop {
            let mut group = Vec::new(); reserve(&mut group, capacity.min(slice.len()), "createParentBoundables")?;
            group.extend(slice.by_ref().take(capacity));
            if group.is_empty() { break; }
            reserve(&mut result, 1, "createParentBoundables")?;
            result.push(SharedSlot::Boundable(node(level, group)));
        }
    }
    Ok(result)
}

pub(super) fn build(tree: &TreeRef) -> Result<()> {
    let (built, pending, capacity) = { let value = tree.value(); (value.built, value.pending.clone(), value.node_capacity) };
    if built { return Ok(()); }
    if capacity <= 1 { return Err(SpatialAccessError::Capacity { operation: "build" }); }
    let pending = required(pending, "itemBoundables", "build")?;
    let mut current = Vec::new(); reserve(&mut current, pending.len(), "build")?;
    current.extend(pending.slots().iter().cloned());
    let root = if current.is_empty() { node(0, Vec::new()) } else {
        let mut level = 0_i32;
        loop {
            current = parents(current, level, capacity as usize)?;
            if current.len() == 1 { break boundable(current.pop().unwrap(), "root", 0, "build")?; }
            level = level.wrapping_add(1);
        }
    };
    tree.complete_build(root);
    Ok(())
}
fn query_node(value: &BoundableRef, search: Envelope, output: &mut Vec<SharedSlot>) -> Result<()> {
    let children = match &*value.value() {
        BoundableValue::Node { children, .. } => required(children.clone(), "childBoundables", "query")?,
        _ => return Err(list_error("query", ListAccessError::WrongClass { field: "root", index: 0, expected: "org.locationtech.jts.index.strtree.AbstractNode", actual: value.class_name() })),
    };
    query_children(children, search, output)
}
fn query_children(children: SharedListRef, search: Envelope, output: &mut Vec<SharedSlot>) -> Result<()> {
    let children = children.slots();
    for (index, slot) in children.iter().enumerate() {
        let child = boundable(slot.clone(), "childBoundables", index, "query")?;
        let child_value = child.value();
        match &*child_value {
            BoundableValue::Item { bounds, item } => {
                let envelope = bounds.as_ref().ok_or(SpatialAccessError::MissingBounds { operation: "query" })?;
                if !envelope.get().intersects(search) { continue; }
                let item = item.clone();
                drop(child_value);
                reserve(output, 1, "query")?;
                output.push(item);
            }
            BoundableValue::Node { bounds: Some(envelope), children, .. } => {
                if !envelope.get().intersects(search) { continue; }
                let children = required(children.clone(), "childBoundables", "query")?;
                drop(child_value);
                query_children(children, search, output)?;
            }
            BoundableValue::Node { bounds: None, .. } => {
                // Lazy bounds can publish caches through this same owner
                drop(child_value);
                let envelope = bounds(&child)?.ok_or(SpatialAccessError::MissingBounds { operation: "query" })?;
                if envelope.get().intersects(search) { query_node(&child, search, output)?; }
            }
        }
    }
    Ok(())
}
pub(super) fn query(tree: &TreeRef, search: Envelope) -> Result<Vec<SharedSlot>> {
    build(tree)?;
    let root = tree.value().root.clone().ok_or(SpatialAccessError::MissingRoot)?;
    if matches!(&*root.value(), BoundableValue::Node { children: Some(children), .. } if children.len() == 0) { return Ok(Vec::new()); }
    let envelope = bounds(&root)?.ok_or(SpatialAccessError::MissingBounds { operation: "query" })?;
    let mut output = Vec::new();
    if envelope.get().intersects(search) { query_node(&root, search, &mut output)?; }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::JavaDoubleRef;

    #[test]
    fn comparator_keeps_left_bounds_cache_before_reached_right_cast_failure() {
        let original = EnvelopeRef::new(Envelope { min_x: 1.0, max_x: 3.0, min_y: -2.0, max_y: 4.0 });
        let item = BoundableRef::new(BoundableValue::Item { bounds: Some(original.clone()), item: SharedSlot::Null });
        let left = node(0, vec![SharedSlot::Boundable(item)]);
        let right = SharedSlot::Double(JavaDoubleRef::from_value(7.0));
        let error = compare(&SharedSlot::Boundable(left.clone()), &right, true).unwrap_err();
        assert!(matches!(error, SpatialAccessError::List { operation: "compare", error: ListAccessError::WrongClass { actual: "java.lang.Double", .. } }));
        let cache = match &*left.value() {
            BoundableValue::Node { bounds: Some(cache), .. } => cache.clone(),
            _ => panic!("The left projection publishes its computed cache before the right cast"),
        };
        assert_eq!(cache.get(), original.get());
        assert_ne!(cache.identity(), original.identity());
    }
    #[test]
    fn query_dispatches_by_actual_child_without_normalizing_parent_level() {
        for nested in [false,true] {
            let expected=JavaDoubleRef::from_value(17.0);
            let leaf=BoundableRef::new(BoundableValue::Item {
                bounds:Some(EnvelopeRef::new(Envelope { min_x:1.0,max_x:3.0,min_y:1.0,max_y:3.0 })),
                item:SharedSlot::Double(expected.clone()),
            });
            let child=if nested { node(9,vec![SharedSlot::Boundable(leaf)]) } else { leaf };
            let parent_level=if nested { 0 } else { 1 };
            let root=node(parent_level,vec![SharedSlot::Boundable(child)]);
            let tree=TreeRef::new(crate::shared_world_owners::TreeValue { built:true,node_capacity:2,pending:None,root:Some(root.clone()) });
            let result=query(&tree,Envelope { min_x:0.0,max_x:4.0,min_y:0.0,max_y:4.0 }).unwrap();
            assert_eq!(result.len(),1);
            let SharedSlot::Double(actual)=&result[0] else { panic!("Query returns the original raw Item value"); };
            assert!(actual.same_instance(&expected));
            assert!(matches!(&*root.value(),BoundableValue::Node { level,.. } if *level==parent_level));
        }
    }

    fn item_at(x: f64, y: f64) -> BoundableRef {
        BoundableRef::new(BoundableValue::Item {
            bounds: Some(EnvelopeRef::new(Envelope { min_x:x, max_x:x, min_y:y, max_y:y })),
            item: SharedSlot::Null,
        })
    }

    fn identities(values: &[SharedSlot]) -> Vec<usize> {
        values.iter().map(|value| match value {
            SharedSlot::Boundable(value) => value.identity(),
            _ => panic!("Expected original boundable identities"),
        }).collect()
    }

    #[test]
    fn sort_leaf_centers_preserves_identity_and_stable_zero_ties() {
        let leaves = [(3.0,0.0),(0.0,4.0),(-0.0,-1.0),(-2.0,4.0),(0.0,0.0)]
            .into_iter().map(|(x,y)| item_at(x,y)).collect::<Vec<_>>();
        for (horizontal,expected) in [(true,[3,1,2,4,0]),(false,[2,0,4,1,3])] {
            let mut values = leaves.iter().cloned().map(SharedSlot::Boundable).collect::<Vec<_>>();
            sort(&mut values,horizontal).unwrap();
            assert_eq!(identities(&values),expected.map(|index| leaves[index].identity()));
        }
    }

    #[test]
    fn sort_retains_node_cache_before_reached_class_failure() {
        let leaf = item_at(2.0,1.0);
        let left = node(0,vec![SharedSlot::Boundable(leaf)]);
        let mut values = vec![SharedSlot::Double(JavaDoubleRef::from_value(7.0)),SharedSlot::Boundable(left.clone())];
        let error = sort(&mut values,true).unwrap_err();
        assert!(matches!(error,SpatialAccessError::List { operation:"compare",error:ListAccessError::WrongClass { field:"itemBoundables",index:0,actual:"java.lang.Double",.. } }));
        match &*left.value() {
            BoundableValue::Node { bounds:Some(bounds),.. } => assert_eq!(bounds.get(),Envelope { min_x:2.0,max_x:2.0,min_y:1.0,max_y:1.0 }),
            _ => panic!("A reached Node comparison retains its computed bounds"),
        }
    }

    #[test]
    fn sort_retains_nonfinite_order_and_lazy_missing_bounds_failure() {
        for (coordinates,expected) in [
            ([2.0,f64::NAN,1.0],[0,1,2]),
            ([f64::INFINITY,f64::NEG_INFINITY,1.0],[1,2,0]),
            ([f64::MAX,1.0,-f64::MAX],[2,1,0]),
        ] {
            let leaves = coordinates.map(|x| item_at(x,0.0));
            let mut values = leaves.iter().cloned().map(SharedSlot::Boundable).collect::<Vec<_>>();
            sort(&mut values,true).unwrap();
            assert_eq!(identities(&values),expected.map(|index| leaves[index].identity()));
        }
        let missing = BoundableRef::new(BoundableValue::Item { bounds:None,item:SharedSlot::Null });
        let mut values = vec![SharedSlot::Boundable(missing.clone())];
        sort(&mut values,true).unwrap();
        assert_eq!(identities(&values),[missing.identity()]);
        values.push(SharedSlot::Boundable(item_at(1.0,1.0)));
        assert!(matches!(sort(&mut values,true),Err(SpatialAccessError::MissingBounds { operation:"compare" })));
    }

}
#[cfg(test)]
mod query_single_read_tests {
    use super::*;
    use crate::history::JavaDoubleRef;

    #[test]
    fn cached_aliases_and_uncached_failure_preserve_traversal() {
        let all = Envelope { min_x: 0.0, max_x: 10.0, min_y: 0.0, max_y: 10.0 };
        let point = |x| Envelope { min_x: x, max_x: x, min_y: x, max_y: x };
        let envelope = EnvelopeRef::new(point(1.0));
        let first = JavaDoubleRef::from_value(17.0);
        let leaf = BoundableRef::new(BoundableValue::Item { bounds: Some(envelope.clone()), item: SharedSlot::Double(first.clone()) });
        let live_children = SharedListRef::from_slots(vec![SharedSlot::Boundable(leaf)]);
        let cached = BoundableRef::new(BoundableValue::Node { level: 9, bounds: Some(EnvelopeRef::new(all)), children: Some(live_children.clone()) });
        let root_children = SharedListRef::from_slots(vec![SharedSlot::Boundable(cached)]);
        let root_cache = EnvelopeRef::new(all);
        let root = BoundableRef::new(BoundableValue::Node { level: 0, bounds: Some(root_cache.clone()), children: Some(root_children.clone()) });
        let tree = TreeRef::new(crate::shared_world_owners::TreeValue { built: true, node_capacity: 2, pending: None, root: Some(root.clone()) });
        let hits = query(&tree, point(1.0)).unwrap();
        assert_eq!(hits.len(), 1);
        let SharedSlot::Double(actual) = &hits[0] else { panic!("Expected original Item payload"); };
        assert!(actual.same_instance(&first));

        envelope.set(point(5.0));
        let second = JavaDoubleRef::from_value(23.0);
        live_children.slots_mut()[0] = SharedSlot::Boundable(BoundableRef::new(BoundableValue::Item { bounds: Some(envelope.clone()), item: SharedSlot::Double(second.clone()) }));
        assert!(query(&tree, point(1.0)).unwrap().is_empty());
        let hits = query(&tree, point(5.0)).unwrap();
        assert_eq!(hits.len(), 1);
        let SharedSlot::Double(actual) = &hits[0] else { panic!("Expected changed live Item payload"); };
        assert!(actual.same_instance(&second));

        let inner = node(0, vec![SharedSlot::Boundable(BoundableRef::new(BoundableValue::Item { bounds: Some(envelope.clone()), item: SharedSlot::Null }))]);
        let broken = node(1, vec![SharedSlot::Boundable(inner.clone()), SharedSlot::Double(JavaDoubleRef::from_value(7.0))]);
        root_children.slots_mut().push(SharedSlot::Boundable(broken.clone()));
        let mut prefix = Vec::new();
        let error = query_node(&root, all, &mut prefix).unwrap_err();
        assert_eq!(error, SpatialAccessError::List { operation: "computeBounds", error: ListAccessError::WrongClass { field: "childBoundables", index: 1, expected: "org.locationtech.jts.index.strtree.Boundable", actual: "java.lang.Double" } });
        assert_eq!(prefix.len(), 1);
        let SharedSlot::Double(actual) = &prefix[0] else { panic!("Expected earlier cached hit before failure"); };
        assert!(actual.same_instance(&second));
        match &*inner.value() {
            BoundableValue::Node { bounds: Some(cache), .. } => {
                assert_eq!(cache.get(), point(5.0));
                assert_ne!(cache.identity(), envelope.identity());
            }
            _ => panic!("Earlier lazy child publishes its cache before the reached failure"),
        }
        assert!(matches!(&*broken.value(), BoundableValue::Node { bounds: None, .. }));
        assert!(matches!(&*root.value(), BoundableValue::Node { bounds: Some(cache), .. } if cache.identity() == root_cache.identity()));
    }
}
