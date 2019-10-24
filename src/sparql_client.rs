use itertools::Itertools;
use log::{debug, trace};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SparqlError {
    #[error("No etities with topo id {0} found")]
    TopoIdNotFound(String),
    #[error("Several entities with topo id {0}")]
    DuplicatedTopoId(String),
    #[error("error: {0}")]
    ReqwestError(#[from] reqwest::Error),
    #[error("error: {0}")]
    InvalidJsonError(#[from] json::Error),
}

pub struct SparqlClient {
    client: reqwest::Client,
    config: crate::client::Config,
}

impl SparqlClient {
    pub fn new(config: crate::client::Config) -> Self {
        SparqlClient {
            client: reqwest::Client::new(),
            config,
        }
    }
    fn query(&self, query: &str) -> Result<json::JsonValue, SparqlError> {
        debug!("Sparql query: {}", query);
        let response = self
            .client
            .get(&self.config.sparql_endpoint)
            .query(&[("format", "json"), ("query", query)])
            .send()?
            .text()?;
        debug!("Query response: {:?}", response);
        Ok(json::parse(&response)?)
    }

    pub fn sparql(
        &self,
        variables: &[&str],
        where_clause: &str,
    ) -> Result<Vec<HashMap<String, String>>, SparqlError> {
        let vars = variables.iter().format(" ");
        let query = format!("SELECT {} WHERE {{ {} SERVICE wikibase:label {{ bd:serviceParam wikibase:language \"en\". }} }}", vars, where_clause);
        let res = self.query(&query)?;

        let mut result = Vec::new();
        for binding in res["results"]["bindings"].members() {
            let values = binding
                .entries()
                .map(|(k, v)| (k.to_string(), v["value"].as_str().unwrap_or("").into()))
                .collect();
            result.push(values);
        }
        Ok(result)
    }

    pub fn find_line(
        &self,
        producer_id: &str,
        gtfs_id: &str,
    ) -> Result<Vec<HashMap<String, String>>, SparqlError> {
        trace!("Finding line {} of producer {}", gtfs_id, producer_id);
        self.sparql(
            &[
                "?line",
                "?lineLabel",
                "?route_short_name",
                "?route_long_name",
                "?physical_mode",
                "?gtfs_id",
            ],
            &format!(
                "?line wdt:{instance_of} wd:{line}.
    ?line wdt:{gtfs_id_prop} \"{gtfs_id}\".
    ?line wdt:{producer_prop} wd:{producer_id}.
    ?line wdt:{route_short_name} ?route_short_name.
    ?line wdt:{route_long_name} ?route_long_name.
    ?line wdt:{physical_mode} ?physical_mode.",
                instance_of = self.config.properties.instance_of,
                line = self.config.items.line,
                gtfs_id_prop = self.config.properties.gtfs_id,
                producer_prop = self.config.properties.produced_by,
                route_short_name = self.config.properties.gtfs_short_name,
                route_long_name = self.config.properties.gtfs_long_name,
                physical_mode = self.config.properties.physical_mode,
                gtfs_id = gtfs_id,
                producer_id = producer_id
            ),
        )
    }

    /// Finds an entity id with a given topo_id
    /// Will fail if no item or strictly more than one is returned
    /// You must provide the id of the `topo tool id` property
    pub fn find_entity_by_topo_id(&self, item_topo_id: &str, topo_id_id: &str)  -> Result<String, SparqlError> {
        self.sparql(
            &["item_id"],
            &format!(
                "?item_id wdt:{topo_id_id} '{item_topo_id}'",
                topo_id_id = topo_id_id,
                item_topo_id = item_topo_id
            ),
        ).and_then(|items| match items.as_slice(){
            [] => Err(SparqlError::TopoIdNotFound(item_topo_id.to_string())),
            [item] => Ok(item["item_id"].to_owned()),
            _ => Err(SparqlError::DuplicatedTopoId(item_topo_id.to_string())),
        })
    }
}
