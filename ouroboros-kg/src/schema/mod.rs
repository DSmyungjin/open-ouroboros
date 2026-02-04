//! Knowledge graph schema module
//!
//! This module defines the schema for the Ouroboros knowledge graph,
//! including Task, Result, Context, and Knowledge nodes and their relationships.

pub mod types;
pub mod task;
pub mod result;
pub mod context;
pub mod knowledge;
pub mod relationships;

pub use types::{Task, TaskStatus, Result as TaskResult, Context, Knowledge, KnowledgeType};
pub use task::{create_task, get_task, update_task};
pub use result::{create_result, link_result_to_task};
pub use context::{create_context, link_context_to_result};
pub use knowledge::{create_knowledge, get_knowledge, link_knowledge_to_task};
pub use relationships::{
    create_dependency,
    create_caused,
    create_supports,
    create_refutes,
    create_replaced_by,
    create_learned_from,
    get_related_knowledge,
    get_causal_chain,
};
