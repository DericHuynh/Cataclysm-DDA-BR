use cdda_core::coords::{OmtPos, SubmapPos, WorldPos, ZLevel};

#[test]
fn worldpos_to_submap_positive() {
    let wp = WorldPos::new(13, 25, ZLevel::new(0));
    let (sm, local) = wp.to_submap();
    assert_eq!((sm.x, sm.y), (1, 2));
    assert_eq!((local.x, local.y), (1, 1));
}

#[test]
fn worldpos_to_submap_negative() {
    let wp = WorldPos::new(-1, -1, ZLevel::new(0));
    let (sm, local) = wp.to_submap();
    assert_eq!((sm.x, sm.y), (-1, -1));
    assert_eq!((local.x, local.y), (11, 11));
}

#[test]
fn worldpos_submap_roundtrip() {
    let wp = WorldPos::new(42, -17, ZLevel::new(3));
    let (sm, local) = wp.to_submap();
    let reconstructed = WorldPos::from_submap(sm, local);
    assert_eq!(reconstructed, wp);
}

#[test]
fn submap_to_worldpos_top_left() {
    let sm = SubmapPos::new(1, 2, ZLevel::new(0));
    let wp = sm.to_worldpos();
    assert_eq!((wp.x, wp.y), (12, 24));
}

#[test]
fn worldpos_to_omt() {
    let wp = WorldPos::new(50, 50, ZLevel::new(0));
    let omt = wp.to_omt();
    assert_eq!((omt.x, omt.y), (2, 2));
}

#[test]
fn worldpos_to_omt_negative() {
    let wp = WorldPos::new(-1, -1, ZLevel::new(0));
    let omt = wp.to_omt();
    assert_eq!((omt.x, omt.y), (-1, -1));
}

#[test]
fn omt_to_worldpos_top_left() {
    let omt = OmtPos::new(2, 3, ZLevel::new(0));
    let wp = omt.to_worldpos();
    assert_eq!((wp.x, wp.y), (48, 72));
}

#[test]
fn worldpos_to_om() {
    let wp = WorldPos::new(4320, 4320, ZLevel::new(0));
    let om = wp.to_om();
    assert_eq!((om.x, om.y), (1, 1));
}

#[test]
fn worldpos_to_om_negative() {
    let wp = WorldPos::new(-1, -1, ZLevel::new(0));
    let om = wp.to_om();
    assert_eq!((om.x, om.y), (-1, -1));
}

#[test]
fn manhattan_2d_distance() {
    let a = WorldPos::new(0, 0, ZLevel::new(0));
    let b = WorldPos::new(3, 4, ZLevel::new(0));
    assert_eq!(a.dist_manhattan_2d(b), 7);
}

#[test]
fn manhattan_3d_distance_includes_z() {
    let a = WorldPos::new(0, 0, ZLevel::new(0));
    let b = WorldPos::new(3, 4, ZLevel::new(5));
    assert_eq!(a.dist_manhattan(b), 12); // 3 + 4 + 5
}

#[test]
fn chebyshev_2d_distance() {
    let a = WorldPos::new(0, 0, ZLevel::new(0));
    let b = WorldPos::new(3, 4, ZLevel::new(0));
    assert_eq!(a.dist_chebyshev_2d(b), 4);
}

#[test]
fn chebyshev_3d_distance() {
    let a = WorldPos::new(0, 0, ZLevel::new(0));
    let b = WorldPos::new(3, 4, ZLevel::new(5));
    assert_eq!(a.dist_chebyshev(b), 5);
}

#[test]
fn zlevel_clamps_negative() {
    let z = ZLevel::new(-20);
    assert_eq!(z.0, -10);
}

#[test]
fn zlevel_clamps_positive() {
    let z = ZLevel::new(20);
    assert_eq!(z.0, 10);
}

#[test]
fn zlevel_checked_add_overflow() {
    let z = ZLevel::new(9);
    assert!(z.checked_add(2).is_none());
}

#[test]
fn pos_add_works() {
    let a = WorldPos::new(10, 10, ZLevel::new(0));
    let b = WorldPos::new(5, -3, ZLevel::new(2));
    let c = a + b;
    assert_eq!((c.x, c.y, c.z.0), (15, 7, 2));
}

#[test]
fn pos_sub_works() {
    let a = WorldPos::new(10, 10, ZLevel::new(0));
    let b = WorldPos::new(5, -3, ZLevel::new(2));
    let c = a - b;
    assert_eq!((c.x, c.y, c.z.0), (5, 13, -2));
}
