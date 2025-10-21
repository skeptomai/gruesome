use gruesome::grue_compiler::ir::PropertyManager;
use gruesome::grue_compiler::ZMachineVersion;

fn main() {
    let mut pm = PropertyManager::new_with_version(ZMachineVersion::V3);
    
    // This should panic - property 32 exceeds V3 limit of 31
    pm.assign_property_number("invalid_prop", 32);
}
