use crate::api_client::ApiClient;
use crate::sparql_client::SparqlClient;
use anyhow::Error;
use log::{info, warn};
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone, Default)]
pub struct EntitiesId {
    pub properties: Properties,
    pub items: Items,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct Properties {
    pub topo_id_id: String,
    pub produced_by: String,
    pub instance_of: String,
    pub physical_mode: String,
    pub gtfs_short_name: String,
    pub gtfs_long_name: String,
    pub gtfs_id: String,
    pub data_source: String,
    pub first_seen_in: String,
    pub source: String,
    pub file_format: String,
    pub sha_256: String,
    pub has_physical_mode: String,
    pub tool_version: String,
    /// Shows a relation of inclusion: a stop point is part_of a stop area
    pub part_of: String,
    /// Shows that a stop is connected to a line https://www.wikidata.org/wiki/Property:P81
    pub connecting_line: String,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct Items {
    pub route: String,
    pub producer: String,
    pub tramway: String,
    pub subway: String,
    pub railway: String,
    pub bus: String,
    pub ferry: String,
    pub cable_car: String,
    pub gondola: String,
    pub funicular: String,
    pub stop_point: String,
    pub stop_area: String,
    pub stop_entrance: String,
    pub stop_boarding_area: String,
    pub stop_generic_node: String,
}

pub struct Client {
    pub api: ApiClient,
    pub sparql: SparqlClient,
}

impl EntitiesId {
    pub fn physical_mode(&self, route: &gtfs_structures::Route) -> &str {
        use gtfs_structures::RouteType::*;
        match route.route_type {
            Tramway => &self.items.tramway,
            Subway => &self.items.subway,
            Rail => &self.items.railway,
            Bus => &self.items.bus,
            Ferry => &self.items.ferry,
            CableCar => &self.items.cable_car,
            Gondola => &self.items.gondola,
            Funicular => &self.items.funicular,
            _ => &self.items.bus,
        }
    }
}

impl Client {
    pub fn new(api_endpoint: &str, sparql_enpoint: &str, topo_id_id: &str) -> Result<Self, Error> {
        let sparql = SparqlClient::new(sparql_enpoint, topo_id_id)?;
        Ok(Self {
            api: ApiClient::new(api_endpoint, sparql.config.clone())?,
            sparql,
        })
    }

    pub fn import_gtfs(
        &self,
        gtfs_filename: &str,
        producer_id: &str,
        producer_name: &str,
    ) -> Result<(), anyhow::Error> {
        let gtfs = gtfs_structures::RawGtfs::from_zip(gtfs_filename).map_err(|e| e.compat())?;

        let routes = gtfs.routes.map_err(|e| e.compat())?;
        log::info!("import gtfs version {}", crate::GIT_VERSION);
        let data_source_id =
            self.api
                .insert_data_source(&gtfs.sha256, &producer_id, gtfs_filename)?;

        for route in routes {
            let r = self.sparql.find_route(&producer_id, &route.id)?;
            match r.as_slice() {
                [] => {
                    info!(
                        "Route “{}” ({}) does not exist, inserting",
                        route.long_name, route.short_name
                    );
                    self.api
                        .insert_route(&route, &data_source_id, producer_name)?;
                }
                [e] => {
                    info!(
                        "Route “{}” ({}) already exists with id {}, skipping",
                        route.long_name, route.short_name, e["route"]
                    );
                }
                _ => warn!(
                    "Route “{}” ({}) exists many times. Something is not right",
                    route.long_name, route.short_name
                ),
            }
        }
        Ok(())
    }
}
