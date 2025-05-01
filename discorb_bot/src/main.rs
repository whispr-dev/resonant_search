// src/main.rs

// Declare the modules from the resonant search engine project
// These files should exist in the src/ directory alongside main.rs
mod tokenizer;
mod entropy;
mod prime_hilbert;
mod engine;

// Use necessary components from the engine and standard library
// ***** REMINDER: Ensure SearchResult struct in engine.rs is marked as `pub` *****
use engine::{ResonantEngine, SearchResult};
use std::env;      // For reading environment variables (DISCORD_TOKEN, RESONANT_DATA_DIR)
use std::path::Path; // For handling the data directory path
use std::sync::Arc; // For Atomic Reference Counting (safe shared ownership)

// Use necessary components from the Serenity library for Discord interaction
use serenity::async_trait; // Macro for async functions in traits
use serenity::model::channel::Message; // Struct representing a Discord message
use serenity::model::gateway::Ready;   // Struct representing the bot's ready state
use serenity::prelude::*; // Imports common Serenity traits and types (Context, EventHandler, GatewayIntents, TypeMapKey)

// Optional: Uncomment the line below if using dotenv for .env file support
// use dotenv::dotenv;

// --- Shared State Management ---

// Define a struct to act as a key in Serenity's TypeMap.
// This allows us to store and retrieve our ResonantEngine instance within the Serenity context.
struct EngineContainer;

impl TypeMapKey for EngineContainer {
    // The value associated with this key will be our ResonantEngine, wrapped for thread-safe access.
    // Arc: Allows multiple owners (e.g., multiple event handlers running concurrently).
    // RwLock: Allows many readers OR one writer at a time, preventing data races.
    type Value = Arc<RwLock<ResonantEngine>>;
}

// --- Discord Event Handler ---

// Define the main struct for handling Discord events.
struct Handler;

// Implement the EventHandler trait from Serenity for our Handler struct.
// The `async_trait` macro allows using `async fn` within the trait implementation.
#[async_trait]
impl EventHandler for Handler {
    // This async function is called by Serenity whenever a new message is created
    // in a channel the bot has access to and the necessary intents for.
    async fn message(&self, ctx: Context, msg: Message) {
        // --- Basic Message Filtering ---

        // Ignore messages sent by any bot (including this one) to prevent loops or unwanted interactions.
        if msg.author.bot {
            return;
        }

        // --- Command Parsing ---

        // Define the prefix required for commands directed at this bot.
        const PREFIX: &str = "!search ";

        // Check if the message content starts with the defined prefix.
        // `strip_prefix` returns an Option<&str> containing the rest of the message if the prefix matches.
        if let Some(query_content) = msg.content.strip_prefix(PREFIX) {
            let query = query_content.trim(); // Remove leading/trailing whitespace from the actual query part.

            // Handle cases where the user typed the prefix but no query text.
            if query.is_empty() {
                // Attempt to send a helpful message back to the channel.
                if let Err(why) = msg.channel_id.say(&ctx.http, "Please provide a search term after `!search `.").await {
                    // Log an error if sending the message failed.
                    eprintln!("Error sending empty query message: {:?}", why);
                }
                return; // Stop processing this message further.
            }

            // --- Access Shared Engine & Perform Search ---

            // Retrieve the shared ResonantEngine instance from the Serenity context's data storage.
            // Need to await reading the context data, then get the engine using our TypeMapKey.
            // Cloning the Arc is very cheap (it only increases the reference count).
            let engine_lock = {
                let data_read = ctx.data.read().await;
                data_read.get::<EngineContainer>().expect("FATAL: Expected ResonantEngine in TypeMap.").clone()
            }; // The read lock on context data is released here.

            // Perform the search. This requires mutable access (`write` lock) to the engine,
            // primarily because the tokenizer might need to add new words encountered in the query
            // to its internal vocabulary, which modifies its state.
            let results: Vec<SearchResult> = {
                let mut engine = engine_lock.write().await; // Acquire write lock. Waits if another write or reads are active.
                engine.search(query, 5) // Execute the search, requesting top 5 results.
                // The write lock (`engine` guard) is automatically released when it goes out of scope here.
            }; // The `results` vector is now owned by this scope.


            // --- Format and Send Results Back to Discord ---

            if results.is_empty() {
                // If the search yielded no results, inform the user.
                let response = format!("No resonant results found for '{}'.", query);
                if let Err(why) = msg.channel_id.say(&ctx.http, &response).await {
                    eprintln!("Error sending 'no results' message: {:?}", why);
                }
            } else {
                // If results were found, format them into one or more messages.
                // Start building the response string. Using Discord markdown for titles.
                let mut response_buffer = format!("Top Resonant Matches for `{} S:`\n\n", query);

                // Iterate through the found search results.
                for (idx, result) in results.iter().enumerate() {
                    // Limit the snippet length for display purposes to avoid huge messages.
                    let snippet_preview = result.snippet.chars().take(150).collect::<String>();
                    let snippet_suffix = if result.snippet.chars().count() > 150 { "..." } else { "" }; // Add "..." if truncated.

                    // Format the details for a single result. Using Discord markdown for emphasis and code blocks.
                    let result_line = format!(
                        "**{}. {}**\n   *Path:* `{}`\n   *Score:* {:.4}\n   *Preview:* {}{}\n\n",
                        idx + 1,                       // Result number (1-based index).
                        result.title,                  // Document title.
                        result.path,                   // Document file path.
                        result.score,                  // Resonance score.
                        snippet_preview,               // Limited preview snippet.
                        snippet_suffix                 // Ellipsis if snippet was cut short.
                    );

                    // --- Handle Discord Message Length Limit (2000 chars) ---
                    // Check if adding the *next* formatted result line would exceed the limit.
                    // Using a conservative threshold (e.g., 1950) to leave room for formatting/overhead.
                    if response_buffer.len() + result_line.len() > 1950 {
                        // If the limit would be exceeded, send the current contents of the buffer first.
                        if let Err(why) = msg.channel_id.say(&ctx.http, &response_buffer).await {
                             eprintln!("Error sending message chunk: {:?}", why);
                        }
                        // Start the *new* buffer with the current result line that didn't fit in the previous one.
                        response_buffer = result_line;
                    } else {
                        // If it fits, append the current result line to the buffer.
                        response_buffer.push_str(&result_line);
                    }
                } // End of loop iterating through results.

                // After the loop, send any remaining content left in the buffer.
                // This handles the last few results or the entire response if it was short.
                if !response_buffer.is_empty() {
                    if let Err(why) = msg.channel_id.say(&ctx.http, &response_buffer).await {
                        eprintln!("Error sending final message chunk: {:?}", why);
                    }
                }
            } // End of formatting/sending results.
        } // End of `if let Some(query_content) = msg.content.strip_prefix(PREFIX)` block.

        // Potential place to add more command handlers:
        // else if msg.content.starts_with("!help") { /* handle help */ }
        // else if msg.content.starts_with("!stats") { /* handle stats */ }

    } // End of `message` function.

    // This async function is called once by Serenity after the bot has successfully
    // connected to Discord and completed the initial handshake.
    async fn ready(&self, _ctx: Context, ready: Ready) {
        // Log that the bot is connected and ready to operate.
        println!("Bot connected successfully!");
        println!("Logged in as: {}#{}", ready.user.name, ready.user.discriminator.unwrap_or(0000)); // Show username and discriminator
        // You could potentially set the bot's activity/status here using ctx.set_activity().
    }

    // You can implement other EventHandler trait methods here if needed (e.g., reaction_add, guild_create).

} // End of `impl EventHandler for Handler`.


// --- Application Entry Point ---

// The `tokio::main` macro transforms the async main function into a synchronous one
// by setting up and managing the Tokio runtime needed for async operations.
#[tokio::main]
async fn main() {
    println!("Starting Resonant Search Bot application...");

    // --- Optional: Load .env file ---
    // Uncomment the following lines if you add the `dotenv` crate to Cargo.toml
    // and want to load variables from a `.env` file in the project root.
    // if dotenv::dotenv().is_err() {
    //     println!("Note: .env file not found or failed to load. Relying solely on environment variables.");
    // } else {
    //     println!("Loaded configuration from .env file.");
    // }

    // --- Load Essential Configuration ---
    println!("Loading configuration from environment variables...");

    // Retrieve the Discord bot token. Panic if not set, as the bot cannot run without it.
    let token = env::var("DISCORD_TOKEN")
        .expect("FATAL ERROR: DISCORD_TOKEN environment variable not set.");

    // Retrieve the path to the directory containing documents to index. Panic if not set.
    let data_dir_str = env::var("RESONANT_DATA_DIR")
        .expect("FATAL ERROR: RESONANT_DATA_DIR environment variable not set.");
    let data_path = Path::new(&data_dir_str);

    // --- Validate Data Directory ---
    // Crucial check: Ensure the specified data directory exists and is actually a directory.
    if !data_path.exists() {
         eprintln!("FATAL ERROR: Data directory specified by RESONANT_DATA_DIR does not exist: '{}'", data_dir_str);
         std::process::exit(1); // Exit the application immediately.
     }
     if !data_path.is_dir() {
        eprintln!("FATAL ERROR: Path specified by RESONANT_DATA_DIR is not a directory: '{}'", data_dir_str);
        std::process::exit(1); // Exit the application immediately.
    }
    println!("Confirmed data directory exists and is valid: {}", data_dir_str);


    // --- Initialize the Resonant Search Engine ---
    println!("Initializing Resonant Search Engine core...");
    let mut engine = ResonantEngine::new(); // Create a new engine instance.

    println!("Loading and indexing documents from '{}'...", data_dir_str);
    // Attempt to load and index documents from the specified directory.
    // If this fails, the bot cannot function, so treat it as a fatal error.
    if let Err(e) = engine.load_directory(data_path) {
        eprintln!("FATAL ERROR: Failed to load document directory '{}': {}", data_dir_str, e);
        std::process::exit(1);
    }
    println!("Document loading and initial indexing complete.");

    // --- Prepare Engine for Shared Access ---
    // Wrap the initialized engine in Arc<RwLock<>> to allow safe sharing across
    // potentially concurrent event handlers managed by Serenity/Tokio.
    let engine_arc = Arc::new(RwLock::new(engine));
    println!("Engine prepared for shared access.");


    // --- Configure and Build the Discord Client (Serenity) ---
    println!("Configuring Discord client connection...");

    // Define the "Gateway Intents" - these specify which types of events the bot
    // needs to receive from Discord's gateway (WebSocket connection).
    let intents = GatewayIntents::GUILD_MESSAGES      // Receive message creation/update/delete events in servers.
        | GatewayIntents::DIRECT_MESSAGES   // Receive message events in direct messages.
        | GatewayIntents::MESSAGE_CONTENT;  // **CRITICAL**: Allows reading the actual content of messages.
                                            // Requires enabling this specific privileged intent in the Discord Developer Portal for your bot.

    // Use the Client builder pattern to configure and create the Serenity client.
    let mut client = Client::builder(&token, intents) // Provide the bot token and requested intents.
        .event_handler(Handler) // Register our custom struct `Handler` to handle events.
        .await // Building the client involves async operations (like fetching initial gateway info).
        .expect("FATAL ERROR: Failed to create Discord client"); // Panic if client creation fails.

    // --- Store Shared State in Client ---
    // Place the thread-safe wrapper around the engine into the client's shared data storage (TypeMap).
    // This makes it accessible from within event handlers via the `ctx.data` field.
    { // Use a block to clearly define the scope of the mutable borrow of client.data.
        let mut data = client.data.write().await; // Get write access to the client's internal TypeMap.
        data.insert::<EngineContainer>(engine_arc); // Insert the Arc<RwLock<Engine>> using our custom key.
    } // The write lock on `client.data` is released here.
    println!("Discord client configured and shared state injected.");


    // --- Start the Bot ---
    println!("Attempting to connect to Discord and start listening for events...");
    println!("Press Ctrl+C to shut down the bot.");

    // `client.start()` connects to the Discord gateway, starts the event loop,
    // and runs indefinitely until an error occurs or the process is terminated.
    // It will automatically handle shard management, heartbeating, and event dispatching.
    if let Err(why) = client.start().await {
        // If `start()` returns an error, it usually indicates a non-recoverable issue
        // (e.g., invalid token, network problems, intent misconfiguration).
        eprintln!("FATAL ERROR: Discord client encountered an unrecoverable error: {:?}", why);
        std::process::exit(1); // Exit the application on fatal client error.
    }

    // This line is usually only reached if the bot somehow stops gracefully,
    // which typically doesn't happen with `client.start()` unless specifically handled.
    println!("Bot shutdown sequence initiated.");
}