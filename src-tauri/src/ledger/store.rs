use axon_ledger::DataFrame;
use serde::Serialize;
use std::collections::HashMap;

pub struct LedgerDataset {
    pub name: String,
    pub parent_id: Option<String>,
    pub df: DataFrame,
}

#[derive(Serialize, Clone)]
pub struct DatasetNode {
    pub id: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub row_count: usize,
    pub col_count: usize,
}

#[derive(Default)]
pub struct DatasetStore {
    datasets: HashMap<String, LedgerDataset>,
    order: Vec<String>,
    next_id: u32,
    active: Option<String>,
}

impl DatasetStore {
    pub fn insert(&mut self, name: String, df: DataFrame, parent_id: Option<String>) -> String {
        self.next_id += 1;
        let id = format!("ds-{}", self.next_id);
        self.order.push(id.clone());
        self.datasets.insert(
            id.clone(),
            LedgerDataset { name, parent_id, df },
        );
        self.active = Some(id.clone());
        id
    }

    pub fn get(&self, id: &str) -> Option<&LedgerDataset> {
        self.datasets.get(id)
    }

    pub fn active_df(&self) -> Option<&DataFrame> {
        self.active.as_ref().and_then(|id| self.datasets.get(id)).map(|d| &d.df)
    }

    pub fn active_id(&self) -> Option<&str> {
        self.active.as_deref()
    }

    pub fn set_active(&mut self, id: &str) -> bool {
        if self.datasets.contains_key(id) {
            self.active = Some(id.to_string());
            true
        } else {
            false
        }
    }

    pub fn list(&self) -> Vec<DatasetNode> {
        self.order
            .iter()
            .filter_map(|id| {
                let ds = self.datasets.get(id)?;
                Some(DatasetNode {
                    id: id.clone(),
                    name: ds.name.clone(),
                    parent_id: ds.parent_id.clone(),
                    row_count: ds.df.height(),
                    col_count: ds.df.width(),
                })
            })
            .collect()
    }

    pub fn remove(&mut self, id: &str) -> bool {
        if self.datasets.remove(id).is_none() {
            return false;
        }
        self.order.retain(|x| x != id);
        let children: Vec<String> = self
            .datasets
            .iter()
            .filter(|(_, ds)| ds.parent_id.as_deref() == Some(id))
            .map(|(k, _)| k.clone())
            .collect();
        for child_id in children {
            self.remove(&child_id);
        }
        if self.active.as_deref() == Some(id) {
            self.active = self.order.last().cloned();
        }
        true
    }

    pub fn rename(&mut self, id: &str, name: String) -> bool {
        if let Some(ds) = self.datasets.get_mut(id) {
            ds.name = name;
            true
        } else {
            false
        }
    }
}
