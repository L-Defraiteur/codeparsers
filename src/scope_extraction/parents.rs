//! **The parents of a tree, read once** (19 November 2026). `Node::parent` of tree-sitter descends from the root and
//! scans the children of every level to find the node: called for each identifier by the shared extraction, it was
//! about half of the analysis of rag3weaver/src and of the engine (sampled release profile). At the start of the
//! parsing of a file, [`index`] notes the parent of every node in one walk; [`parent_of`] reads it.
//!
//! The index is a stack, one entry per tree being parsed on this thread, held by the [`ParentsGuard`] that `index`
//! gives back and taken away when it drops: a nested parse has its own, and outside any parse `parent_of` asks the
//! tree itself. A node of another tree is not in the index, and `parent_of` asks the tree too.

use std::cell::RefCell;
use std::collections::HashMap;

use super::base_scope_extraction_parser::SyntaxNode;

thread_local! {
    static PARENTS: RefCell<Vec<HashMap<usize, SyntaxNode>>> = const { RefCell::new(Vec::new()) };
}

/// Takes the index of its tree away when it drops. Keep it for as long as the nodes of the tree are read.
#[must_use = "the index lives as long as its guard"]
pub struct ParentsGuard(());

impl Drop for ParentsGuard {
    fn drop(&mut self) {
        PARENTS.with(|p| {
            p.borrow_mut().pop();
        });
    }
}

/// Note the parent of every node under `root`, in one walk with a tree cursor.
pub fn index(root: SyntaxNode) -> ParentsGuard {
    let mut parents: HashMap<usize, SyntaxNode> = HashMap::new();
    let mut cursor = root.walk();
    let mut stack: Vec<SyntaxNode> = Vec::new();
    loop {
        let node = cursor.node();
        if cursor.goto_first_child() {
            stack.push(node);
            parents.insert(cursor.node().id(), node);
            continue;
        }
        loop {
            if cursor.goto_next_sibling() {
                if let Some(parent) = stack.last() {
                    parents.insert(cursor.node().id(), *parent);
                }
                break;
            }
            if !cursor.goto_parent() {
                PARENTS.with(|p| p.borrow_mut().push(parents));
                return ParentsGuard(());
            }
            stack.pop();
        }
    }
}

/// The parent of `node`: from the index of the tree being parsed, or from the tree itself.
pub fn parent_of(node: SyntaxNode) -> Option<SyntaxNode> {
    PARENTS.with(|p| p.borrow().last().and_then(|m| m.get(&node.id()).copied())).or_else(|| node.parent())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree(language: tree_sitter::Language, source: &str) -> tree_sitter::Tree {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&language).expect("the grammar");
        parser.parse(source, None).expect("a tree")
    }

    fn root_of(tree: &tree_sitter::Tree) -> SyntaxNode {
        // SAFETY: the tree outlives every node read below, as in the parsers.
        unsafe { std::mem::transmute(tree.root_node()) }
    }

    fn every_node(root: SyntaxNode) -> Vec<SyntaxNode> {
        let mut out = Vec::new();
        let mut stack = vec![root];
        while let Some(n) = stack.pop() {
            out.push(n);
            let mut c = n.walk();
            stack.extend(n.children(&mut c));
        }
        out
    }

    /// **The index gives the parent the tree gives**, for every node of a file of each language of the router.
    #[test]
    fn the_index_gives_the_parent_the_tree_gives_in_every_language() {
        let cases: Vec<(&str, tree_sitter::Language, &str)> = vec![
            ("rust", tree_sitter_rust::LANGUAGE.into(), "mod m { pub struct S { x: u8 } impl S { fn f(&self) -> u8 { let y = self.x; if y > 0 { g(y) } else { 0 } } } }\nfn g(n: u8) -> u8 { n }\n"),
            ("c", tree_sitter_c::LANGUAGE.into(), "struct s { int x; };\nint g(int n) { return n; }\nint f(struct s *p) { int y = p->x; return y > 0 ? g(y) : 0; }\n"),
            ("cpp", tree_sitter_cpp::LANGUAGE.into(), "namespace m { template <class T> struct S { T x; T f() const { auto y = x; return y > 0 ? g(y) : T{}; } }; }\n"),
            ("python", tree_sitter_python::LANGUAGE.into(), "class C:\n    def f(self, a):\n        y = [g(x) for x in a if x]\n        return lambda z: y + z\n"),
            ("typescript", tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(), "export class C { f(a: number[]): number { const y = a.map((x) => g(x)); return y.length; } }\n"),
            ("go", tree_sitter_go::LANGUAGE.into(), "package m\ntype S struct { x int }\nfunc (s S) f() int { y := s.x; if y > 0 { return g(y) }; return 0 }\n"),
            ("csharp", tree_sitter_c_sharp::LANGUAGE.into(), "namespace M { class C { int x; int F() { var y = x; return y > 0 ? G(y) : 0; } } }\n"),
        ];
        for (name, language, source) in cases {
            let t = tree(language, source);
            let root = root_of(&t);
            let _guard = index(root);
            let nodes = every_node(root);
            assert!(nodes.len() > 20, "{name}: a tree of some size");
            for n in nodes {
                assert_eq!(parent_of(n).map(|p| p.id()), n.parent().map(|p| p.id()), "{name}: the parent of a {} at {:?}", n.kind(), n.start_position());
            }
        }
    }

    /// **A nested index is taken away with its guard**, and outside any guard the tree is asked.
    #[test]
    fn a_nested_index_is_taken_away_and_outside_the_tree_is_asked() {
        let outer = tree(tree_sitter_rust::LANGUAGE.into(), "fn f() { g(1); }\n");
        let inner = tree(tree_sitter_python::LANGUAGE.into(), "def h():\n    return k(2)\n");
        let (o, i) = (root_of(&outer), root_of(&inner));
        let first = every_node(o).into_iter().find(|n| n.kind() == "call_expression").expect("a call");
        let _outer_guard = index(o);
        {
            let _inner_guard = index(i);
            let call = every_node(i).into_iter().find(|n| n.kind() == "call").expect("a call");
            assert_eq!(parent_of(call).map(|p| p.id()), call.parent().map(|p| p.id()));
        }
        assert_eq!(parent_of(first).map(|p| p.id()), first.parent().map(|p| p.id()), "the outer index is back");
        drop(_outer_guard);
        PARENTS.with(|p| assert!(p.borrow().is_empty(), "no index is left on the thread"));
        assert_eq!(parent_of(first).map(|p| p.id()), first.parent().map(|p| p.id()), "outside any guard, the tree is asked");
    }
}
