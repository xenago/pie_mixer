use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use anyhow::{anyhow, Result};
use pipewire;
use tracing::{debug, error, info, warn};
use tracing_subscriber::filter::{EnvFilter, LevelFilter};

/// Retain relevant metadata associated with a PipeWire node
struct NodeInfo {
    global_id: u32,
    description: String,
    media_class: String,
    input: bool, // True if the node is an input (like a mic), False if the node is an output (like a speaker)
    ports: Vec<(u32, String, String)>, // Port ID, Channel Name, Direction
}

/// Entrypoint
fn main() -> Result<()> {
    // Initialize log/tracing
    tracing_subscriber::fmt()
        // Control verbosity with RUST_LOG environment variable, falling back to INFO as the default
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();
    info!("pie_mixer init...");

    // Initialize PipeWire
    pipewire::init();

    // Use a reference-counted main loop to allow sharing with callbacks
    let pipewire_main_loop = pipewire::main_loop::MainLoopRc::new(None)
        .map_err(|error| anyhow!("Failed to initialize PipeWire main loop: {:?}", error))?;

    // Manage local resources and the PipeWire session configuration
    let pipewire_context = pipewire::context::ContextBox::new(&pipewire_main_loop.loop_(), None)
        .map_err(|error| anyhow!("Failed to create PipeWire context: {:?}", error))?;

    // Connect to the PipeWire daemon
    let pipewire_core = pipewire_context
        .connect(None)
        .map_err(|error| anyhow!("Failed to connect to PipeWire core: {:?}", error))?;

    // The registry provides access to global objects like nodes and devices
    let pipewire_registry = pipewire_core
        .get_registry()
        .map_err(|error| anyhow!("Failed to retrieve PipeWire registry: {:?}", error))?;

    // Shared storage between the main thread and local callbacks
    let discovered_nodes = Rc::new(RefCell::new(HashMap::<u32, NodeInfo>::new()));
    let discovered_nodes_collection = discovered_nodes.clone();
    let discovered_nodes_removal = discovered_nodes.clone();

    // Listener reacting to global events (i.e. nodes and ports) from the registry
    // Must be kept in scope to continue receiving callbacks
    let _registry_listener = pipewire_registry
        .add_listener_local()
        .global(move |global_object| {
            // Node discovery
            if global_object.type_ == pipewire::types::ObjectType::Node {
                if let Some(props) = global_object.props {
                    let description = props
                        .get(*pipewire::keys::NODE_DESCRIPTION)
                        .or_else(|| props.get(*pipewire::keys::NODE_NAME))
                        .unwrap_or("Unknown");
                    let media_class = props.get(*pipewire::keys::MEDIA_CLASS).unwrap_or("Unknown");
                    let input = media_class.to_string().contains("Source")
                        || media_class.to_string().contains("Input");
                    // Save the discovered node
                    discovered_nodes_collection
                        .borrow_mut()
                        .entry(global_object.id)
                        .or_insert(NodeInfo {
                            global_id: global_object.id,
                            description: description.to_string(),
                            media_class: media_class.to_string(),
                            input,
                            ports: Vec::new(),
                        });
                }
            }
            // Port discovery (required for Stereo pairing)
            if global_object.type_ == pipewire::types::ObjectType::Port {
                if let Some(props) = global_object.props {
                    if let Some(node_id) = props
                        .get(*pipewire::keys::NODE_ID)
                        .and_then(|s| s.parse::<u32>().ok())
                    {
                        let channel = props
                            .get(*pipewire::keys::AUDIO_CHANNEL)
                            .or(props.get(*pipewire::keys::PORT_NAME))
                            .unwrap_or("unknown")
                            .to_string();
                        let dir = props
                            .get(*pipewire::keys::PORT_DIRECTION)
                            .unwrap_or("unknown")
                            .to_string();
                        // Save the discovered port
                        if let Some(node) =
                            discovered_nodes_collection.borrow_mut().get_mut(&node_id)
                        {
                            node.ports.push((global_object.id, channel, dir));
                        }
                    }
                }
            }
        })
        .global_remove(move |id| {
            // Evict node from cache if destroyed in the PipeWire graph
            discovered_nodes_removal.borrow_mut().remove(&id);
        })
        .register();

    // Set up a listener that only quits when our specific sync is finished
    let main_loop_handle = pipewire_main_loop.clone();
    let pending_sync = Rc::new(RefCell::new(None));
    let pending_sync_check = pending_sync.clone();
    let _core_listener = pipewire_core
        .add_listener_local()
        .done(move |_object_id, seq| {
            // Check if we have a sequence number to wait for
            if let Some(target_seq) = *pending_sync_check.borrow() {
                // seq and target_seq are both of the correct internal SPA type
                if seq == target_seq {
                    main_loop_handle.quit();
                }
            }
        })
        .register();

    // Trigger a sync event and store the sequence number
    let sync_seq = pipewire_core
        .sync(0)
        .map_err(|error| anyhow!("PipeWire sync failed: {:?}", error))?;
    *pending_sync.borrow_mut() = Some(sync_seq);

    // Run the loop until the 'done' event with the matching sequence number is received
    pipewire_main_loop.run();

    // Output the results in a readable format

    // 1. Collect values for sorting
    let nodes_borrow = discovered_nodes.borrow();
    let mut sorted_nodes: Vec<&NodeInfo> = nodes_borrow.values().collect();

    // 2. Sort by global_id in ascending order
    sorted_nodes.sort_by_key(|n| n.global_id);

    // 3. Determine the longest description for table-like alignment
    let max_desc_len = sorted_nodes
        .iter()
        .map(|n| n.description.len())
        .max()
        .unwrap_or(40);

    // 4. Print table
    info!("PipeWire nodes found: {}", sorted_nodes.len());
    for node in &sorted_nodes {
        debug!(
            "[ID: {:3}]  Description: {:<width$}  [Type: {}  Ports: {:?}",
            node.global_id,
            node.description,
            match node.media_class.as_str() {
                "Audio/Sink" | "Stream/Input/Audio" => " Audio Output]",
                "Audio/Source" | "Stream/Output/Audio" => "  Audio Input]",
                "Video/Source" | "Stream/Output/Video" => "  Video Input]",
                "Video/Sink" | "Stream/Input/Video" => " Video Output]",
                _ => "Other/Virtual]",
            },
            node.ports,
            width = max_desc_len
        );
    }

    // Filter down separate lists for selected nodes
    // TODO FIXME: this is hard-coded and should be more flexible
    //   to support arbitrary inputs and outputs of any kind, like HDMI audio
    let selected_inputs: Vec<&NodeInfo> = sorted_nodes
        .iter()
        .filter(|node| node.description.to_uppercase().contains("SPDIF") && node.input)
        .cloned()
        .collect();
    if !selected_inputs.is_empty() {
        info!("Matching inputs: {}", selected_inputs.len());
        for node in &selected_inputs {
            debug!("[ID: {:3}] {}", node.global_id, node.description);
        }
    }
    let selected_outputs: Vec<&NodeInfo> = sorted_nodes
        .iter()
        .filter(|node| node.description.to_uppercase().contains("SPDIF") && !node.input)
        .cloned()
        .collect();
    if !selected_outputs.is_empty() {
        info!("Matching outputs: {}", selected_outputs.len());
        for node in &selected_outputs {
            debug!("[ID: {:3}] {}", node.global_id, node.description);
        }
    }

    // Create mixer by mapping all matching inputs to the output(s)
    if selected_outputs.is_empty() {
        Err(anyhow!("No matching output found"))
    } else if selected_inputs.is_empty() {
        Err(anyhow!("No matching input(s) found"))
    } else {
        info!("Configuring mixer...");

        // Target the first discovered matching output
        // TODO FIXME: this should support sending to multiple outputs simultaneously
        let target_output_node = selected_outputs[0];
        debug!(
            "Mapping all matching inputs to output [ID: {}, {}]",
            target_output_node.global_id, target_output_node.description
        );

        // Keep the link proxies in-scope to retain them in the PipeWire graph
        let mut links = Vec::new();

        // Link each input node to the output
        for input_node in selected_inputs {
            debug!(
                "Stereo linking: [ID: {}, {}]=>[ID: {}, {}]",
                input_node.global_id,
                input_node.description,
                target_output_node.global_id,
                target_output_node.description
            );

            // Pair ports by direction: Outbound from Source to Inbound at Sink
            let src_ports: Vec<_> = input_node
                .ports
                .iter()
                .filter(|(_, _, dir)| dir == "out")
                .collect();
            let snk_ports: Vec<_> = target_output_node
                .ports
                .iter()
                .filter(|(_, _, dir)| dir == "in")
                .collect();

            // Explicitly link matching pairs (FL->FL, FR->FR, etc)
            for (out_id, out_chan, _) in src_ports {
                // Find a destination port that matches the specific channel name
                if let Some((in_id, _in_chan, _)) =
                    snk_ports.iter().find(|(_, name, _)| name == out_chan)
                {
                    debug!("Linking channel {}: [{}]->[{}]", out_chan, out_id, in_id);
                    let props = pipewire::__properties__! {
                        *pipewire::keys::LINK_OUTPUT_NODE => input_node.global_id.to_string(),
                        *pipewire::keys::LINK_OUTPUT_PORT => out_id.to_string(),
                        *pipewire::keys::LINK_INPUT_NODE => target_output_node.global_id.to_string(),
                        *pipewire::keys::LINK_INPUT_PORT => in_id.to_string(),
                        *pipewire::keys::LINK_PASSIVE => "false", // Activate the link (wakes hardware)
                        // "object.linger" => "true", // Persistent link FIXME TODO: first need to establish teardown process
                    };
                    // Request the core to create the link
                    match pipewire_core
                        .create_object::<pipewire::link::Link>("link-factory", &props)
                    {
                        Ok(link) => links.push(link),
                        Err(e) => error!("Failed to create link: {:?}", e),
                    }
                } else {
                    warn!("No matching input port found for channel {}", out_chan);
                }
            }
        }
        info!("Mixer links established!");
        // Run the main loop endlessly-ish
        info!("Keep program active to maintain connections, or press Ctrl+C to stop the mixer...");
        pipewire_main_loop.run();
        Ok(())
    }
}
