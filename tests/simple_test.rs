mod utils;
use maplit::btreeset;
use pretty_assertions::assert_eq;
use transit_topo::api_client::{ObjectType, PropertyDataType};
use utils::wikibase::DataSourceItem;

fn check_initiale_state(wikibase: &utils::Wikibase) {
    // we first check that our exists method cannot find a unknown object
    assert!(!wikibase.exists(ObjectType::Item, "pouet"));

    // then we check the real objects
    assert!(wikibase.exists(
        ObjectType::Property(PropertyDataType::String),
        "instance of"
    ));
    assert!(wikibase.exists(
        ObjectType::Property(PropertyDataType::String),
        "gtfs short name"
    ));
    assert!(wikibase.exists(
        ObjectType::Property(PropertyDataType::String),
        "gtfs long name"
    ));
    assert!(wikibase.exists(ObjectType::Property(PropertyDataType::String), "gtfs id"));
    assert!(wikibase.exists(
        ObjectType::Property(PropertyDataType::Item),
        "Topo tools id"
    ));

    assert!(wikibase.exists(ObjectType::Item, "physical mode"));
    assert!(wikibase.exists(ObjectType::Item, "producer"));
    assert!(wikibase.exists(ObjectType::Item, "route"));
    assert!(wikibase.exists(ObjectType::Item, "bus"));
    assert!(wikibase.exists(ObjectType::Item, "bob the bus mapper"));

    // we check all the objects with a topo_id
    assert_eq!(
        wikibase.get_topo_objects(),
        btreeset![
            "producer".to_owned(),
            "route".to_owned(),
            "bob_the_bus_mapper".to_owned(),
            "instance_of".to_owned(),
            "physical_mode".to_owned(),
            "gtfs_short_name".to_owned(),
            "gtfs_long_name".to_owned(),
            "gtfs_id".to_owned(),
            "produced_by".to_owned(),
            "physical_mode".to_owned(),
            "has_physical_mode".to_owned(),
            "tramway".to_owned(),
            "subway".to_owned(),
            "railway".to_owned(),
            "bus".to_owned(),
            "ferry".to_owned(),
            "cable_car".to_owned(),
            "gondola".to_owned(),
            "funicular".to_owned(),
            "content_id".to_owned(),
            "data_source".to_owned(),
            "file_format".to_owned(),
            "file_link".to_owned(),
            "first_seen_in".to_owned(),
            "tool_version".to_owned(),
        ],
    );
}

fn import_gtfs(docker: &utils::DockerContainerWrapper) {
    utils::run(
        "import-gtfs",
        &[
            "--producer",
            "Q12",
            "--input-gtfs",
            &format!(
                "{}/tests/fixtures/gtfs.zip",
                std::env::var("CARGO_MANIFEST_DIR").expect("impossible to find env var")
            ),
            "--api",
            &docker.api_endpoint,
            "--sparql",
            &docker.sparql_endpoint,
        ],
    );
}

#[test]
fn simple_test() {
    let docker = utils::DockerContainerWrapper::new();

    utils::run("prepopulate", &["--api", &docker.api_endpoint]);

    let wikibase = utils::Wikibase::new(&docker);
    check_initiale_state(&wikibase);

    // We call again the prepopulate, there shouldn't be any differences
    // since it should be idempotent
    utils::run("prepopulate", &["--api", &docker.api_endpoint]);
    check_initiale_state(&wikibase);

    // we now import a gtfs
    import_gtfs(&docker);

    // there are 1 data sources with routes imported
    let datasources = wikibase.get_producer_datasources_id("Q12");
    assert_eq!(datasources.len(), 1);

    let all_objects = wikibase.get_all_items_for_datasource(datasources.iter().next().unwrap());
    assert_eq!(all_objects.len(), 5);

    let find_by_gtfs_id = |gtfs_id: &str| {
        all_objects
            .iter()
            .find(|o| o.gtfs_id == Some(gtfs_id.to_owned()))
    };

    let ab = find_by_gtfs_id("AB").expect(&format!("impossible to find AB"));
    assert_eq!(
        ab.label,
        "Airport - Bullfrog – (bob the bus mapper)".to_owned()
    );
    assert_eq!(ab.instance_of, "route".to_owned());

    assert_eq!(
        find_by_gtfs_id("BFC")
            .expect("impossible to find obj")
            .instance_of,
        "route".to_owned()
    );
    assert_eq!(
        find_by_gtfs_id("STBA")
            .expect("impossible to find obj")
            .instance_of,
        "route".to_owned()
    );
    assert_eq!(
        find_by_gtfs_id("CITY")
            .expect("impossible to find obj")
            .instance_of,
        "route".to_owned()
    );
    assert_eq!(
        find_by_gtfs_id("AAMV")
            .expect("impossible to find obj")
            .instance_of,
        "route".to_owned()
    );

    // we reimport the gtfs
    import_gtfs(&docker);

    // there are now 2 datasources, because we do no merge.
    // It might change in the futur
    let new_datasources = wikibase.get_producer_datasources_id("Q12");
    assert_eq!(new_datasources.len(), 2);

    let new_datasource: std::collections::BTreeSet<_> =
        new_datasources.difference(&datasources).collect();
    assert_eq!(new_datasource.len(), 1);

    let all_objects = wikibase.get_all_items_for_datasource(new_datasource.iter().next().unwrap());
    assert_eq!(all_objects.len(), 5);

    let ab = find_by_gtfs_id("AB").expect(&format!("impossible to find AB"));
    assert_eq!(
        ab.label,
        "Airport - Bullfrog – (bob the bus mapper)".to_owned()
    );
    assert_eq!(ab.instance_of, "route".to_owned());
}
