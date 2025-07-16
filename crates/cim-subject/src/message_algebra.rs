//! # Message Algebra for Correlation Chains
//!
//! This module implements algebraic operations on message correlation chains,
//! enabling complex message flow analysis and validation.

use std::collections::{HashMap, HashSet, VecDeque};
use crate::correlation::{MessageIdentity, IdType, CorrelationError, Result};

/// Represents a correlation chain - a sequence of related messages
#[derive(Debug, Clone)]
pub struct CorrelationChain {
    /// The root message that started this chain
    pub root: MessageIdentity,
    
    /// All messages in the chain, indexed by their ID
    pub messages: HashMap<IdType, MessageIdentity>,
    
    /// Causation relationships: child -> parent
    pub causation_graph: HashMap<IdType, IdType>,
    
    /// Reverse causation: parent -> children
    pub caused_messages: HashMap<IdType, Vec<IdType>>,
}

impl CorrelationChain {
    /// Create a new chain from a root message
    ///
    /// # Errors
    ///
    /// Returns an error if the provided message is not a root message
    pub fn new(root: MessageIdentity) -> Result<Self> {
        if !root.is_root() {
            return Err(CorrelationError::InvalidIdentity(
                "Chain must start with a root message".to_string()
            ));
        }
        
        let mut messages = HashMap::new();
        messages.insert(root.message_id.clone(), root.clone());
        
        Ok(Self {
            root,
            messages,
            causation_graph: HashMap::new(),
            caused_messages: HashMap::new(),
        })
    }
    
    /// Add a message to the chain
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The message's correlation ID doesn't match the chain
    /// - The message's parent is not found in the chain
    pub fn add_message(&mut self, message: MessageIdentity) -> Result<()> {
        // Verify correlation matches
        if message.correlation_id != self.root.correlation_id {
            return Err(CorrelationError::InvalidIdentity(
                "Message correlation ID doesn't match chain".to_string()
            ));
        }
        
        // For non-root messages, verify parent exists
        if !message.is_root() {
            let parent_id = &message.causation_id.0;
            if !self.messages.contains_key(parent_id) {
                return Err(CorrelationError::InvalidIdentity(
                    "Parent message not found in chain".to_string()
                ));
            }
            
            // Add to causation graph
            self.causation_graph.insert(message.message_id.clone(), parent_id.clone());
            
            // Add to reverse causation
            self.caused_messages
                .entry(parent_id.clone())
                .or_default()
                .push(message.message_id.clone());
        }
        
        // Add message
        self.messages.insert(message.message_id.clone(), message);
        
        Ok(())
    }
    
    /// Get all messages caused by a specific message
    #[must_use]
    pub fn get_caused_by(&self, message_id: &IdType) -> Vec<&MessageIdentity> {
        self.caused_messages
            .get(message_id)
            .map(|children| {
                children.iter()
                    .filter_map(|id| self.messages.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }
    
    /// Get the parent message that caused this one
    #[must_use]
    pub fn get_parent(&self, message_id: &IdType) -> Option<&MessageIdentity> {
        self.causation_graph
            .get(message_id)
            .and_then(|parent_id| self.messages.get(parent_id))
    }
    
    /// Get the path from root to a specific message
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The message is not found in the chain
    /// - The chain is broken (parent missing)
    pub fn get_path_to(&self, message_id: &IdType) -> Result<Vec<&MessageIdentity>> {
        if !self.messages.contains_key(message_id) {
            return Err(CorrelationError::InvalidIdentity(
                "Message not found in chain".to_string()
            ));
        }
        
        let mut path = Vec::new();
        let mut current_id = message_id;
        
        // Walk backwards to root
        loop {
            let message = self.messages.get(current_id)
                .ok_or_else(|| CorrelationError::InvalidIdentity(
                    "Broken chain detected".to_string()
                ))?;
            
            path.push(message);
            
            if message.is_root() {
                break;
            }
            
            current_id = &message.causation_id.0;
        }
        
        path.reverse();
        Ok(path)
    }
    
    /// Check if the chain contains cycles
    #[must_use]
    pub fn has_cycles(&self) -> bool {
        // Use DFS to detect cycles
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        
        for message_id in self.messages.keys() {
            if self.detect_cycle_dfs(message_id, &mut visited, &mut rec_stack) {
                return true;
            }
        }
        
        false
    }
    
    fn detect_cycle_dfs(
        &self,
        node: &IdType,
        visited: &mut HashSet<IdType>,
        rec_stack: &mut HashSet<IdType>,
    ) -> bool {
        if rec_stack.contains(node) {
            return true;
        }
        
        if visited.contains(node) {
            return false;
        }
        
        visited.insert(node.clone());
        rec_stack.insert(node.clone());
        
        if let Some(children) = self.caused_messages.get(node) {
            for child in children {
                if self.detect_cycle_dfs(child, visited, rec_stack) {
                    return true;
                }
            }
        }
        
        rec_stack.remove(node);
        false
    }
    
    /// Get the depth of the chain (longest path from root)
    #[must_use]
    pub fn depth(&self) -> usize {
        let mut max_depth = 0;
        let mut queue = VecDeque::new();
        queue.push_back((&self.root.message_id, 0));
        
        while let Some((node_id, depth)) = queue.pop_front() {
            max_depth = max_depth.max(depth);
            
            if let Some(children) = self.caused_messages.get(node_id) {
                for child in children {
                    queue.push_back((child, depth + 1));
                }
            }
        }
        
        max_depth
    }
}

/// Algebra operations on correlation chains
pub struct MessageAlgebra;

impl MessageAlgebra {
    /// Merge two chains that share a common message
    ///
    /// # Errors
    ///
    /// Returns an error if the chains have different correlation IDs
    pub fn merge_chains(
        chain1: &CorrelationChain,
        chain2: &CorrelationChain,
    ) -> Result<CorrelationChain> {
        // Chains must have same correlation ID
        if chain1.root.correlation_id != chain2.root.correlation_id {
            return Err(CorrelationError::InvalidIdentity(
                "Cannot merge chains with different correlation IDs".to_string()
            ));
        }
        
        // Start with chain1
        let mut merged = chain1.clone();
        
        // Add all messages from chain2
        for message in chain2.messages.values() {
            if !merged.messages.contains_key(&message.message_id) {
                merged.add_message(message.clone())?;
            }
        }
        
        Ok(merged)
    }
    
    /// Find common ancestors of two messages in a chain
    ///
    /// # Errors
    ///
    /// Returns an error if either message is not found in the chain
    pub fn find_common_ancestors<'a>(
        chain: &'a CorrelationChain,
        msg1_id: &IdType,
        msg2_id: &IdType,
    ) -> Result<Vec<&'a MessageIdentity>> {
        let path1 = chain.get_path_to(msg1_id)?;
        let path2 = chain.get_path_to(msg2_id)?;
        
        let set1: HashSet<_> = path1.iter().map(|m| &m.message_id).collect();
        let set2: HashSet<_> = path2.iter().map(|m| &m.message_id).collect();
        
        let common: Vec<_> = set1.intersection(&set2)
            .filter_map(|id| chain.messages.get(id))
            .collect();
        
        Ok(common)
    }
    
    /// Calculate the distance between two messages in a chain
    ///
    /// # Errors
    ///
    /// Returns an error if either message is not found in the chain
    pub fn distance(
        chain: &CorrelationChain,
        msg1_id: &IdType,
        msg2_id: &IdType,
    ) -> Result<usize> {
        let path1 = chain.get_path_to(msg1_id)?;
        let path2 = chain.get_path_to(msg2_id)?;
        
        // Find lowest common ancestor
        let mut lca_index = 0;
        for i in 0..path1.len().min(path2.len()) {
            if path1[i].message_id == path2[i].message_id {
                lca_index = i;
            } else {
                break;
            }
        }
        
        // Distance is sum of distances from each node to LCA
        let dist1 = path1.len() - lca_index - 1;
        let dist2 = path2.len() - lca_index - 1;
        
        Ok(dist1 + dist2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;
    use crate::correlation::MessageFactory;
    
    #[test]
    fn test_correlation_chain_creation() {
        let root_id = Uuid::new_v4();
        let root = MessageFactory::create_root_command(root_id);
        
        let chain = CorrelationChain::new(root.clone()).unwrap();
        assert_eq!(chain.messages.len(), 1);
        assert_eq!(chain.root.message_id, root.message_id);
    }
    
    #[test]
    fn test_add_caused_messages() {
        let root_id = Uuid::new_v4();
        let root = MessageFactory::create_root_command(root_id);
        
        let mut chain = CorrelationChain::new(root.clone()).unwrap();
        
        // Add child message
        let child_id = Uuid::new_v4();
        let child = MessageFactory::command_from_command(child_id, &root);
        chain.add_message(child.clone()).unwrap();
        
        assert_eq!(chain.messages.len(), 2);
        assert_eq!(chain.get_parent(&child.message_id).unwrap().message_id, root.message_id);
        assert_eq!(chain.get_caused_by(&root.message_id).len(), 1);
    }
    
    #[test]
    fn test_path_to_message() {
        let root_id = Uuid::new_v4();
        let root = MessageFactory::create_root_command(root_id);
        
        let mut chain = CorrelationChain::new(root.clone()).unwrap();
        
        // Create a chain: root -> child1 -> child2
        let child1_id = Uuid::new_v4();
        let child1 = MessageFactory::command_from_command(child1_id, &root);
        chain.add_message(child1.clone()).unwrap();
        
        let child2_id = Uuid::new_v4();
        let child2 = MessageFactory::command_from_command(child2_id, &child1);
        chain.add_message(child2.clone()).unwrap();
        
        let path = chain.get_path_to(&child2.message_id).unwrap();
        assert_eq!(path.len(), 3);
        assert_eq!(path[0].message_id, root.message_id);
        assert_eq!(path[1].message_id, child1.message_id);
        assert_eq!(path[2].message_id, child2.message_id);
    }
    
    #[test]
    fn test_chain_depth() {
        let root_id = Uuid::new_v4();
        let root = MessageFactory::create_root_command(root_id);
        
        let mut chain = CorrelationChain::new(root.clone()).unwrap();
        assert_eq!(chain.depth(), 0);
        
        // Add child
        let child_id = Uuid::new_v4();
        let child = MessageFactory::command_from_command(child_id, &root);
        chain.add_message(child.clone()).unwrap();
        assert_eq!(chain.depth(), 1);
        
        // Add grandchild
        let grandchild_id = Uuid::new_v4();
        let grandchild = MessageFactory::command_from_command(grandchild_id, &child);
        chain.add_message(grandchild).unwrap();
        assert_eq!(chain.depth(), 2);
    }
    
    #[test]
    fn test_cycle_detection() {
        let root_id = Uuid::new_v4();
        let root = MessageFactory::create_root_command(root_id);
        
        let chain = CorrelationChain::new(root.clone()).unwrap();
        
        // Normal chain has no cycles
        assert!(!chain.has_cycles());
        
        // Note: Creating actual cycles would require bypassing the MessageFactory
        // which enforces proper causation rules
    }
} 