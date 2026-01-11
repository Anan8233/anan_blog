mod modules;

use modules::config;
use modules::cli;
use modules::database::{storage, comments, analytics};
use modules::content::{compiler, scanner, markdown, templates, theme};
use modules::web::{admin, routes, recommender};

use actix_web::{middleware, web, App, HttpServer};
use compiler::Compiler;
use config::Config;
use log::info;

fn main()  -> std::io::Result<()> {
    // Check for CLI mode
    if let Some(first_arg) = std::env::args().nth(1) {
        match first_arg.as_str() {
            "client" | "server" => {
                cli::run();
                return Ok(());
            }
            _ => {
                // Unknown argument, print help
                cli::run();
                return Ok(());

            }
        }
    }

    // Web server mode (original behavior)
    async_web_server()?;
    Ok(())
}

#[actix_web::main]
async fn async_web_server() -> std::io::Result<()> {
    // Initialize logger
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("debug"));

    info!("Starting LF Blog...");

    // Load configuration
    let config = Config::load().unwrap_or_else(|e| {
        eprintln!("Failed to load config, using defaults: {}", e);
        Config::default()
    });

    // Save default config if it doesn't exist
    if let Err(e) = config.save() {
        info!("Failed to save config file: {}", e);
    }

    info!("Configuration loaded");
    info!(
        "Server will listen on {}:{}",
        config.server.host, config.server.port
    );
    info!("Content directory: {}", config.paths.content_dir.display());
    info!(
        "Generated directory: {}",
        config.paths.generated_dir.display()
    );

    // Initial compilation
    let mut compiler = match Compiler::new(config.clone()) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to create compiler: {}", e);
            std::process::exit(1);
        }
    };

    info!("Performing initial compilation...");
    match compiler.compile() {
        Ok(result) => {
            info!(
                "Initial compilation successful: {} categories, {} items, {} attachments",
                result.total_categories, result.total_items, result.total_attachments
            );
        }
        Err(e) => {
            eprintln!("Initial compilation failed: {}", e);
            // Continue anyway, server will start but may serve old/missing files
        }
    }

    // Start HTTP server
    let bind_address = format!("{}:{}", config.server.host, config.server.port);
    info!("Server starting on http://{}", bind_address);

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(config.clone()))
            .wrap(middleware::Logger::default())
            .wrap(middleware::Compress::default())
            .configure(routes::configure_routes)
    })
    .bind(&bind_address)?
    .run()
    .await
}