use std::collections::{HashMap, HashSet};
use crush_types::{Result, CrushError};
use crate::parser::CrushfileStage;

pub struct MultiStageGraph {
    stages: Vec<CrushfileStage>,
    dependencies: HashMap<String, Vec<String>>,
    named_stages: HashMap<String, usize>,
}

impl MultiStageGraph {
    pub fn new(stages: Vec<CrushfileStage>) -> Self {
        let mut named = HashMap::new();
        let mut deps: HashMap<String, Vec<String>> = HashMap::new();

        for (i, stage) in stages.iter().enumerate() {
            if let Some(ref name) = stage.name {
                named.insert(name.clone(), i);
            }

            let mut stage_deps = Vec::new();
            if let Some(ref from) = stage.from {
                stage_deps.push(from.clone());
            }
            deps.insert(format!("stage_{}", i), stage_deps);
        }

        Self {
            stages,
            dependencies: deps,
            named_stages: named,
        }
    }

    pub fn topological_order(&self) -> Result<Vec<usize>> {
        let n = self.stages.len();
        let mut visited = vec![false; n];
        let mut in_stack = vec![false; n];
        let mut order = Vec::new();

        for i in 0..n {
            if !visited[i] {
                self.dfs(i, &mut visited, &mut in_stack, &mut order)?;
            }
        }

        order.reverse();
        Ok(order)
    }

    fn dfs(&self, node: usize, visited: &mut [bool], in_stack: &mut [bool], order: &mut Vec<usize>) -> Result<()> {
        visited[node] = true;
        in_stack[node] = true;

        let deps_key = format!("stage_{}", node);
        if let Some(deps) = self.dependencies.get(&deps_key) {
            for dep_name in deps {
                if let Some(&dep_idx) = self.named_stages.get(dep_name) {
                    if in_stack[dep_idx] {
                        return Err(CrushError::ImageError(format!(
                            "Circular dependency detected: stage {} → {}",
                            self.stages[node].name.as_deref().unwrap_or("?"),
                            dep_name
                        )));
                    }
                    if !visited[dep_idx] {
                        self.dfs(dep_idx, visited, in_stack, order)?;
                    }
                }
            }
        }

        in_stack[node] = false;
        order.push(node);
        Ok(())
    }

    pub fn independent_stages(&self) -> Vec<usize> {
        let mut independent = Vec::new();
        for (i, stage) in self.stages.iter().enumerate() {
            if stage.from.is_none() && stage.stage_type != "from" {
                independent.push(i);
            }
        }
        independent
    }

    pub fn stage_count(&self) -> usize {
        self.stages.len()
    }

    pub fn get_stage(&self, index: usize) -> Option<&CrushfileStage> {
        self.stages.get(index)
    }

    pub fn has_named_stage(&self, name: &str) -> bool {
        self.named_stages.contains_key(name)
    }

    pub fn final_stage_index(&self) -> usize {
        self.stages.len().saturating_sub(1)
    }
}
