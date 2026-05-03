/// Where a vehicle part can be placed on the vehicle frame.
#[derive(Debug, Clone, PartialEq)]
pub struct VehiclePartLocationTemplate {
    pub name: String,
    pub symbol: char,
    pub x: i32,
    pub y: i32,
}
