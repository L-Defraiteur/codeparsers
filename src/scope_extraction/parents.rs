//! **The parent of a node, read in one place** (19 November 2026). The shared extraction asks the parent of a node
//! for each identifier; `Node::parent` of tree-sitter descends from the root and scans the children of every level
//! to find it — about half of the analysis of rag3weaver/src and of the engine (sampled release profile). Every
//! extraction asks through [`parent_of`], which counts the walks it makes down the tree ([`tree_parent_calls`]).

use std::sync::atomic::{AtomicUsize, Ordering};

use super::base_scope_extraction_parser::SyntaxNode;

static TREE_PARENT_CALLS: AtomicUsize = AtomicUsize::new(0);

/// How many times `Node::parent` of tree-sitter has walked down a tree for the extraction, in this process.
pub fn tree_parent_calls() -> usize {
    TREE_PARENT_CALLS.load(Ordering::Relaxed)
}

/// The parent of `node`, asked to the tree.
pub fn parent_of(node: SyntaxNode) -> Option<SyntaxNode> {
    TREE_PARENT_CALLS.fetch_add(1, Ordering::Relaxed);
    node.parent()
}
