use crate::render::{
    block::Block,
    widget::{DynamicWidget, Widget},
};
use std::{
    future::Future,
    sync::{Arc, Mutex},
};
use tokio::sync::{Notify, mpsc};

/// Represents a single node in a hierarchical tree structure.
#[derive(Debug, Clone)]
pub struct TreeNode {
    /// Display text or identifier of the node.
    pub label: String,
    /// List of child nodes belonging to this node.
    pub children: Vec<TreeNode>,
}

impl TreeNode {
    /// Creates a new `TreeNode` with the specified label and no children.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            children: Vec::new(),
        }
    }

    /// Appends a single child node using a builder pattern.
    pub fn child(mut self, child: TreeNode) -> Self {
        // push the child node into the local vector
        self.children.push(child);
        self
    }

    /// Appends multiple child nodes using a builder pattern.
    pub fn children(mut self, children: Vec<TreeNode>) -> Self {
        // extend children vector with incoming nodes
        self.children.extend(children);
        self
    }

    /// Recursively searches for a node by label and attaches a child node to it.
    /// Returns `true` if the node was successfully found and updated.
    pub fn add_to_path(&mut self, target_label: &str, child: TreeNode) -> bool {
        // check if current node is the target
        if self.label == target_label {
            self.children.push(child);
            return true;
        }

        // recursively traverse children to find the matching node
        for c in &mut self.children {
            if c.add_to_path(target_label, child.clone()) {
                return true;
            }
        }

        false
    }
}

/// Operations for dynamically altering the tree state.
pub enum TreeOp {
    /// Replaces the existing root node with a new one.
    SetRoot(TreeNode),
    /// Adds a new child directly to the current root node.
    AddRootChild(TreeNode),
}

/// Thread-safe handle used to dispatch mutation events to the tree.
pub struct TreeHandle {
    /// Asynchronous channel sender for tree mutation operations.
    sender: mpsc::UnboundedSender<TreeOp>,
}

impl TreeHandle {
    /// Dispatches a command to replace the root node.
    pub fn set_root(&self, root: TreeNode) {
        // send operation ignoring channel closure errors
        let _ = self.sender.send(TreeOp::SetRoot(root));
    }

    /// Dispatches a command to append a child node to the root.
    pub fn push_child(&self, child: TreeNode) {
        // send operation ignoring channel closure errors
        let _ = self.sender.send(TreeOp::AddRootChild(child));
    }
}

/// Dynamic tree widget container supporting concurrent mutations.
pub struct Tree {
    /// Shared reference-counted wrapper around the root node.
    pub(crate) root: Arc<Mutex<TreeNode>>,
    /// Signal used to notify subscribers when tree task processing is complete.
    pub(crate) done_signal: Arc<Notify>,
}

impl Tree {
    /// Wraps a root `TreeNode` inside a dynamic render `Block`.
    pub fn new(root: TreeNode) -> Block<Self> {
        let done_signal = Arc::new(Notify::new());
        // pre-arm signal for static rendering scenarios
        done_signal.notify_one();

        Block::new(Self {
            root: Arc::new(Mutex::new(root)),
            done_signal,
        })
    }

    /// Internal recursive helper function to render tree elements into formatted ASCII strings.
    fn render_node(
        node: &TreeNode,
        prefix: &str,
        is_last: bool,
        is_root: bool,
        lines: &mut Vec<String>,
    ) {
        // render line representation for current node
        if is_root {
            lines.push(node.label.clone());
        } else {
            let connector = if is_last { "└── " } else { "├── " };
            lines.push(format!("{}{}{}", prefix, connector, node.label));
        }

        // iterate over children to build recursive prefix structures
        let child_count = node.children.len();
        for (idx, child) in node.children.iter().enumerate() {
            let last_child = idx == child_count - 1;

            let new_prefix = if is_root {
                ""
            } else if is_last {
                &format!("{}    ", prefix)
            } else {
                &format!("{}│   ", prefix)
            };

            Self::render_node(child, &new_prefix, last_child, false, lines);
        }
    }
}

impl Block<Tree> {
    /// Attaches an async execution handler to manage asynchronous tree state modifications.
    pub fn handler<F, Fut>(self, task: F) -> Self
    where
        F: FnOnce(TreeHandle) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let done_signal = Arc::new(Notify::new());
        let done_notifier = Arc::clone(&done_signal);

        let (tx, mut rx) = mpsc::unbounded_channel::<TreeOp>();

        // spawn background task running user code
        tokio::spawn(async move {
            let handle = TreeHandle { sender: tx };
            task(handle).await;
        });

        // spawn background handler for state updates
        let root_writer = Arc::clone(&self.inner.root);
        tokio::spawn(async move {
            while let Some(op) = rx.recv().await {
                if let Ok(mut lock) = root_writer.lock() {
                    match op {
                        TreeOp::SetRoot(new_root) => {
                            *lock = new_root;
                        }
                        TreeOp::AddRootChild(child) => {
                            lock.children.push(child);
                        }
                    }
                }
            }
            // signal task completion after channel closure
            done_notifier.notify_one();
        });

        let mut block = self;
        block.inner.done_signal = done_signal;
        block
    }
}

impl Widget for Tree {
    type Output = ();

    /// Formats the tree into lines for terminal rendering.
    fn render_content(&self, _width: usize) -> Vec<String> {
        let mut lines = Vec::new();
        // lock state and kick off recursive string formatting
        let root = self.root.lock().unwrap();
        Self::render_node(&root, "", true, true, &mut lines);
        lines
    }
}

impl DynamicWidget for Tree {
    /// Returns the completion signal reference for dynamic render monitoring.
    fn completion_signal(&self) -> Arc<Notify> {
        // return shared handle to completion notification
        Arc::clone(&self.done_signal)
    }

    /// Extracts final widget output value.
    fn extract_output(self) -> Self::Output {
        ()
    }
}
