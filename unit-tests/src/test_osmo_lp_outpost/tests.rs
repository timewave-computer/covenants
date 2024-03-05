use crate::test_osmo_lp_outpost::suite::OsmoLpOutpostBuilder;

#[test]
fn test_covenant() {
    let suite = OsmoLpOutpostBuilder::default().build();
    println!("outpost addr: {:?}", suite.outpost);
}
