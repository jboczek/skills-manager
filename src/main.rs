fn main() -> anyhow::Result<()> {
    tracing::info!("starting skills-manager");
    skills_manager::run()
}
