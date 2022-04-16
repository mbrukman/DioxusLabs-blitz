use dioxus_core::*;

use crate::state::union_ordered_iter;

#[derive(Debug)]
pub struct NodeView<'a> {
    inner: &'a VNode<'a>,
    mask: NodeMask,
}
impl<'a> NodeView<'a> {
    pub fn new(mut vnode: &'a VNode<'a>, view: NodeMask, vdom: &'a VirtualDom) -> Self {
        if let VNode::Component(sc) = vnode {
            let scope = vdom.get_scope(sc.scope.get().unwrap()).unwrap();
            vnode = scope.root_node();
        }
        Self {
            inner: vnode,
            mask: view,
        }
    }

    pub fn id(&self) -> ElementId {
        self.inner.mounted_id()
    }

    pub fn tag(&self) -> Option<&'a str> {
        self.mask.tag.then(|| self.el().map(|el| el.tag)).flatten()
    }

    pub fn namespace(&self) -> Option<&'a str> {
        self.mask
            .namespace
            .then(|| self.el().map(|el| el.namespace).flatten())
            .flatten()
    }

    pub fn attributes(&self) -> impl Iterator<Item = &Attribute<'a>> {
        self.el()
            .map(|el| el.attributes)
            .unwrap_or_default()
            .iter()
            .filter(|a| self.mask.attritutes.contains_attribute(&a.name))
    }

    pub fn text(&self) -> Option<&str> {
        self.mask
            .text
            .then(|| self.txt().map(|txt| txt.text))
            .flatten()
    }

    fn el(&self) -> Option<&'a VElement<'a>> {
        if let VNode::Element(el) = &self.inner {
            Some(el)
        } else {
            None
        }
    }

    fn txt(&self) -> Option<&'a VText<'a>> {
        if let VNode::Text(txt) = &self.inner {
            Some(txt)
        } else {
            None
        }
    }
}

#[derive(PartialEq, Clone, Debug)]
pub enum AttributeMask {
    All,
    Dynamic(Vec<&'static str>),
    Static(&'static [&'static str]),
}

impl AttributeMask {
    pub const NONE: Self = Self::Static(&[]);

    fn contains_attribute(&self, attr: &'static str) -> bool {
        match self {
            AttributeMask::All => true,
            AttributeMask::Dynamic(l) => l.binary_search(&attr).is_ok(),
            AttributeMask::Static(l) => l.binary_search(&attr).is_ok(),
        }
    }

    pub fn single(new: &'static str) -> Self {
        Self::Dynamic(vec![new])
    }

    pub fn verify(&self) {
        match &self {
            AttributeMask::Static(attrs) => debug_assert!(
                attrs.windows(2).all(|w| w[0] < w[1]),
                "attritutes must be increasing"
            ),
            AttributeMask::Dynamic(attrs) => debug_assert!(
                attrs.windows(2).all(|w| w[0] < w[1]),
                "attritutes must be increasing"
            ),
            _ => (),
        }
    }

    pub fn union(&self, other: &Self) -> Self {
        let new = match (self, other) {
            (AttributeMask::Dynamic(s), AttributeMask::Dynamic(o)) => AttributeMask::Dynamic(
                union_ordered_iter(s.iter().copied(), o.iter().copied(), s.len() + o.len()),
            ),
            (AttributeMask::Static(s), AttributeMask::Dynamic(o)) => AttributeMask::Dynamic(
                union_ordered_iter(s.iter().copied(), o.iter().copied(), s.len() + o.len()),
            ),
            (AttributeMask::Dynamic(s), AttributeMask::Static(o)) => AttributeMask::Dynamic(
                union_ordered_iter(s.iter().copied(), o.iter().copied(), s.len() + o.len()),
            ),
            (AttributeMask::Static(s), AttributeMask::Static(o)) => AttributeMask::Dynamic(
                union_ordered_iter(s.iter().copied(), o.iter().copied(), s.len() + o.len()),
            ),
            _ => AttributeMask::All,
        };
        new.verify();
        new
    }

    fn overlaps(&self, other: &Self) -> bool {
        fn overlaps_iter(
            mut self_iter: impl Iterator<Item = &'static str>,
            mut other_iter: impl Iterator<Item = &'static str>,
        ) -> bool {
            if let Some(mut other_attr) = other_iter.next() {
                while let Some(self_attr) = self_iter.next() {
                    while other_attr < self_attr {
                        if let Some(attr) = other_iter.next() {
                            other_attr = attr;
                        } else {
                            return false;
                        }
                    }
                    if other_attr == self_attr {
                        return true;
                    }
                }
            }
            false
        }
        match (self, other) {
            (AttributeMask::All, AttributeMask::All) => true,
            (AttributeMask::All, AttributeMask::Dynamic(v)) => !v.is_empty(),
            (AttributeMask::All, AttributeMask::Static(s)) => !s.is_empty(),
            (AttributeMask::Dynamic(v), AttributeMask::All) => !v.is_empty(),
            (AttributeMask::Static(s), AttributeMask::All) => !s.is_empty(),
            (AttributeMask::Dynamic(v1), AttributeMask::Dynamic(v2)) => {
                overlaps_iter(v1.iter().copied(), v2.iter().copied())
            }
            (AttributeMask::Dynamic(v), AttributeMask::Static(s)) => {
                overlaps_iter(v.iter().copied(), s.iter().copied())
            }
            (AttributeMask::Static(s), AttributeMask::Dynamic(v)) => {
                overlaps_iter(v.iter().copied(), s.iter().copied())
            }
            (AttributeMask::Static(s1), AttributeMask::Static(s2)) => {
                overlaps_iter(s1.iter().copied(), s2.iter().copied())
            }
        }
    }
}

impl Default for AttributeMask {
    fn default() -> Self {
        AttributeMask::Static(&[])
    }
}

#[derive(Default, PartialEq, Clone, Debug)]
pub struct NodeMask {
    // must be sorted
    attritutes: AttributeMask,
    tag: bool,
    namespace: bool,
    text: bool,
}

impl NodeMask {
    pub const NONE: Self = Self::new(AttributeMask::Static(&[]), false, false, false);
    pub const ALL: Self = Self::new(AttributeMask::All, true, true, true);

    /// attritutes must be sorted!
    pub const fn new(attritutes: AttributeMask, tag: bool, namespace: bool, text: bool) -> Self {
        Self {
            attritutes,
            tag,
            namespace,
            text,
        }
    }

    pub fn overlaps(&self, other: &Self) -> bool {
        (self.tag && other.tag)
            || (self.namespace && other.namespace)
            || self.attritutes.overlaps(&other.attritutes)
            || (self.text && other.text)
    }

    pub fn union(&self, other: &Self) -> Self {
        Self {
            attritutes: self.attritutes.union(&other.attritutes),
            tag: self.tag | other.tag,
            namespace: self.namespace | other.namespace,
            text: self.text | other.text,
        }
    }
}
