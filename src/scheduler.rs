//! DAG scheduler with bounded parallelism.
//!
//! Schedules independent crates in parallel levels while respecting
//! dependency order. Each level contains crates that can run concurrently.

use crate::discovery::CrateInfo;
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone)]
pub struct ScheduleLevel {
    pub level: usize,
    pub crates: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Schedule {
    pub levels: Vec<ScheduleLevel>,
    pub total_crates: usize,
}

pub fn build_schedule(crates: &[CrateInfo], max_jobs: usize) -> Schedule {
    if crates.is_empty() {
        return Schedule {
            levels: vec![],
            total_crates: 0,
        };
    }

    let graph = build_dependency_graph(crates);
    let levels = topological_levels(&graph, max_jobs);

    Schedule {
        levels,
        total_crates: crates.len(),
    }
}

struct DepGraph {
    nodes: HashSet<String>,
    edges: HashMap<String, Vec<String>>,
    reverse: HashMap<String, Vec<String>>,
}

fn build_dependency_graph(crates: &[CrateInfo]) -> DepGraph {
    let crate_names: HashSet<_> = crates.iter().map(|c| c.name.clone()).collect();
    let mut edges: HashMap<String, Vec<String>> = HashMap::new();
    let mut reverse: HashMap<String, Vec<String>> = HashMap::new();

    for c in crates {
        edges.entry(c.name.clone()).or_default();
        for dep in &c.dependencies {
            if crate_names.contains(dep) {
                edges.entry(c.name.clone()).or_default().push(dep.clone());
                reverse.entry(dep.clone()).or_default().push(c.name.clone());
            }
        }
    }

    DepGraph {
        nodes: crate_names,
        edges,
        reverse,
    }
}

fn topological_levels(graph: &DepGraph, max_jobs: usize) -> Vec<ScheduleLevel> {
    let mut in_degree: HashMap<&str, usize> =
        graph.nodes.iter().map(|n| (n.as_str(), 0usize)).collect();

    for (node, deps) in &graph.edges {
        for _dep in deps {
            if let Some(count) = in_degree.get_mut(node.as_str()) {
                *count = count.saturating_add(1);
            }
        }
    }

    let mut queue: VecDeque<&str> = in_degree
        .iter()
        .filter(|&(_, &deg)| deg == 0)
        .map(|(&name, _)| name)
        .collect();

    let mut levels: Vec<ScheduleLevel> = Vec::new();
    let mut level_num = 0;

    while !queue.is_empty() {
        let batch: Vec<&str> = queue.drain(..).collect();
        let chunked = batch.chunks(max_jobs.max(1));

        for chunk in chunked {
            let crates: Vec<String> = chunk.iter().map(|s| s.to_string()).collect();
            levels.push(ScheduleLevel {
                level: level_num,
                crates,
            });
            level_num = level_num.saturating_add(1);
        }

        for node in &batch {
            if let Some(dependents) = graph.reverse.get(*node) {
                for dep in dependents {
                    if let Some(deg) = in_degree.get_mut(dep.as_str()) {
                        *deg = deg.saturating_sub(1);
                        if *deg == 0 {
                            queue.push_back(dep.as_str());
                        }
                    }
                }
            }
        }
    }

    levels
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_schedule() {
        let schedule = build_schedule(&[], 4);
        assert!(schedule.levels.is_empty());
        assert_eq!(schedule.total_crates, 0);
    }

    #[test]
    fn test_single_crate() {
        let crates = vec![make_crate("a", &[])];
        let schedule = build_schedule(&crates, 4);
        assert_eq!(schedule.levels.len(), 1);
        assert_eq!(schedule.levels[0].crates, vec!["a"]);
    }

    #[test]
    fn test_linear_chain() {
        let crates = vec![
            make_crate("a", &[]),
            make_crate("b", &["a"]),
            make_crate("c", &["b"]),
        ];
        let schedule = build_schedule(&crates, 4);
        assert_eq!(schedule.levels.len(), 3);
    }

    #[test]
    fn test_independent_crates() {
        let crates = vec![
            make_crate("a", &[]),
            make_crate("b", &[]),
            make_crate("c", &[]),
        ];
        let schedule = build_schedule(&crates, 4);
        assert_eq!(schedule.levels.len(), 1);
        assert_eq!(schedule.levels[0].crates.len(), 3);
    }

    #[test]
    fn test_bounded_parallelism() {
        let crates = vec![
            make_crate("a", &[]),
            make_crate("b", &[]),
            make_crate("c", &[]),
            make_crate("d", &[]),
        ];
        let schedule = build_schedule(&crates, 2);
        assert_eq!(schedule.levels.len(), 2);
        assert_eq!(schedule.levels[0].crates.len(), 2);
    }

    fn make_crate(name: &str, deps: &[&str]) -> CrateInfo {
        CrateInfo {
            name: name.to_string(),
            manifest_path: std::path::PathBuf::from(format!("crates/{name}/Cargo.toml")),
            dependencies: deps.iter().map(|s| s.to_string()).collect(),
        }
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    fn arb_dag(max_nodes: usize) -> impl Strategy<Value = Vec<CrateInfo>> {
        proptest::collection::vec(0..max_nodes, 1..max_nodes).prop_map(|edges| {
            let n = edges.len();
            let mut crates: Vec<CrateInfo> = (0..n)
                .map(|i| CrateInfo {
                    name: format!("crate_{i}"),
                    manifest_path: std::path::PathBuf::from("Cargo.toml"),
                    dependencies: vec![],
                })
                .collect();

            for (i, &dep_idx) in edges.iter().enumerate() {
                if dep_idx < i {
                    crates[i].dependencies.push(format!("crate_{dep_idx}"));
                }
            }
            crates
        })
    }

    proptest! {
        #[test]
        fn proptest_topological_order(dag in arb_dag(20)) {
            let schedule = build_schedule(&dag, 4);
            let mut scheduled: HashSet<String> = HashSet::new();

            for level in &schedule.levels {
                for name in &level.crates {
                    let crate_info = dag.iter().find(|c| &c.name == name);
                    if let Some(info) = crate_info {
                        for dep in &info.dependencies {
                            prop_assert!(
                                scheduled.contains(dep),
                                "Crate {} scheduled before dependency {}",
                                name, dep
                            );
                        }
                    }
                    scheduled.insert(name.clone());
                }
            }
        }

        #[test]
        fn proptest_dependency_order(dag in arb_dag(20)) {
            let schedule = build_schedule(&dag, 4);
            let mut position: HashMap<String, usize> = HashMap::new();

            for (level_idx, level) in schedule.levels.iter().enumerate() {
                for name in &level.crates {
                    position.insert(name.clone(), level_idx);
                }
            }

            for crate_info in &dag {
                for dep in &crate_info.dependencies {
                    if let (Some(&dep_pos), Some(&self_pos)) =
                        (position.get(dep), position.get(&crate_info.name))
                    {
                        prop_assert!(
                            dep_pos <= self_pos,
                            "Dependency {} (level {}) after {} (level {})",
                            dep, dep_pos, crate_info.name, self_pos
                        );
                    }
        }
    }
        }

        #[test]
        fn proptest_bounded_parallelism(dag in arb_dag(20), max_jobs in 1usize..8) {
            let schedule = build_schedule(&dag, max_jobs);
            for level in &schedule.levels {
                prop_assert!(
                    level.crates.len() <= max_jobs,
                    "Level {} has {} crates, max_jobs={}",
                    level.level, level.crates.len(), max_jobs
                );
            }
        }
    }
}
