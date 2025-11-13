//! Demonstrates the PROBLEMS with Arc and Mutex approaches for sharing Tokio Runtime
//!
//! This example shows two problematic approaches:
//! 1. Arc<Runtime> - Cannot guarantee timely shutdown
//! 2. Mutex<Option<Runtime>> - Difficult to use in TSP (long-lived tasks)

mod arc_approach;
mod mutex_approach;

fn main() {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    println!("\n🔴 PROBLEMATIC APPROACHES FOR TOKIO RUNTIME SHARING");
    println!("====================================================\n");

    println!("This demonstrates two common but problematic approaches:");
    println!("1. Arc<Runtime> - Allows easy sharing but prevents controlled shutdown");
    println!("2. Mutex<Option<Runtime>> - Allows controlled shutdown but difficult to use\n");

    println!("--- Approach 1: Arc<Runtime> ---");
    arc_approach::demonstrate_problem();

    println!("\n--- Approach 2: Mutex<Option<Runtime>> ---");
    mutex_approach::demonstrate_problem();

    println!("\n⚠️  Both approaches have significant drawbacks in real applications.\n");
}
