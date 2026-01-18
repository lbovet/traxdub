use anyhow::{anyhow, Result};
use log::debug;
use sophia::api::ns::{Namespace, rdf};
use sophia::api::{MownStr, prelude::*};
use sophia::api::serializer::TripleSerializer;
use sophia::api::source::TripleSource;
use sophia::api::term::{BnodeId, IriRef, SimpleTerm};
use sophia::inmem::graph::FastGraph;
use sophia_turtle::{parser::turtle, serializer::turtle::TurtleSerializer};
use std::sync::atomic::{AtomicU32, Ordering};

use super::{PortType, PortDirection};

// Define namespaces
const INGEN_NS: &str = "http://drobilla.net/ns/ingen#";
const LV2_NS: &str = "http://lv2plug.in/ns/lv2core#";
const PATCH_NS: &str = "http://lv2plug.in/ns/ext/patch#";

// Global sequence number counter
static SEQUENCE_NUMBER: AtomicU32 = AtomicU32::new(0);
// Global blank node ID counter
static BLANK_NODE_COUNTER: AtomicU32 = AtomicU32::new(1);

/// Ingen protocol message builder and parser using RDF/Turtle
pub struct IngenProtocol;

impl IngenProtocol {
    
    /// Get the initialization message with RDF prefixes
    pub fn get_init_message() -> &'static str {
        "@prefix atom: <http://lv2plug.in/ns/ext/atom#> .
@prefix doap: <http://usefulinc.com/ns/doap#> .
@prefix ingen: <http://drobilla.net/ns/ingen#> .
@prefix lv2: <http://lv2plug.in/ns/lv2core#> .
@prefix midi: <http://lv2plug.in/ns/ext/midi#> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix patch: <http://lv2plug.in/ns/ext/patch#> .
@prefix pg: <http://lv2plug.in/ns/ext/port-groups#> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix rsz: <http://lv2plug.in/ns/ext/resize-port#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
"
    }
    
    /// Create a blank node with a unique globally incrementing ID
    fn create_blank_node() -> SimpleTerm<'static> {
        let id = BLANK_NODE_COUNTER.fetch_add(1, Ordering::SeqCst);
        SimpleTerm::BlankNode(BnodeId::new(MownStr::from(format!("bt{}", id))).ok().unwrap())
    }
    
    /// Build an RDF graph to create a port
    pub fn build_create_port(port_name: &str, port_type: &PortType, direction: &PortDirection) -> Result<String> {
        debug!("Building create_port message for '{}'", port_name);
        
        let mut graph = FastGraph::new();
        let lv2 = Namespace::new(LV2_NS)?;
        let patch = Namespace::new(PATCH_NS)?;
        
        let port_path = format!("ingen:/main/{}", port_name);
        let subject = IriRef::new_unchecked(port_path.as_str());
                
        let body_node = Self::create_blank_node();
                
        // Build the body (port description)
        graph.insert(&body_node, &lv2.get("name")?, &SimpleTerm::LiteralDatatype(
            MownStr::from(port_name),
            IriRef::new_unchecked("http://www.w3.org/2001/XMLSchema#string".into())
        ))?;
        
        let type_local = match port_type {
            PortType::Audio => "AudioPort",
            PortType::Midi => "CVPort", // Ingen uses CV ports for MIDI
        };
        graph.insert(&body_node, &rdf::type_, &lv2.get(type_local)?)?;
        
        let direction_local = match direction {
            PortDirection::Input => "InputPort",
            PortDirection::Output => "OutputPort",
        };
        graph.insert(&body_node, &rdf::type_, &lv2.get(direction_local)?)?;
        
        let put_node = Self::create_blank_node();

        // Build patch:Put structure
        graph.insert(&put_node, &rdf::type_, &patch.get("Put")?)?;
        graph.insert(&put_node, &patch.get("subject")?, &subject)?;
        graph.insert(&put_node, &patch.get("body")?, &body_node)?;        

        Self::serialize_graph(&graph, &put_node)
    }
    
    /// Build an RDF graph to create a block (plugin instance)
    pub fn build_create_block(block_id: &str, plugin_uri: &str) -> Result<String> {
        debug!("Building create_block message for '{}' with plugin '{}'", block_id, plugin_uri);
        
        let mut graph = FastGraph::new();
        let lv2 = Namespace::new(LV2_NS)?;
        let ingen = Namespace::new(INGEN_NS)?;
        let patch = Namespace::new(PATCH_NS)?;
        
        let block_path = format!("ingen:/main/{}", block_id);
        
        // Create blank nodes
        let body_node = Self::create_blank_node();
        let put_node = Self::create_blank_node();
        
        // Build the body (block description)
        graph.insert(&body_node, &rdf::type_, &ingen.get("Block")?)?;
        graph.insert(&body_node, &lv2.get("prototype")?, &IriRef::new_unchecked(plugin_uri))?;
        
        // Build patch:Put structure
        graph.insert(&put_node, &rdf::type_, &patch.get("Put")?)?;
        graph.insert(&put_node, &patch.get("subject")?, &IriRef::new_unchecked(block_path.as_str()))?;
        graph.insert(&put_node, &patch.get("body")?, &body_node)?;
        
        Self::serialize_graph(&graph, &put_node)
    }

    /// Build an RDF graph to connect two ports
    pub fn build_connect(source: &str, destination: &str) -> Result<String> {
        debug!("Building connect message: '{}' -> '{}'", source, destination);
        
        let mut graph = FastGraph::new();
        let ingen = Namespace::new(INGEN_NS)?;
        let patch = Namespace::new(PATCH_NS)?;
        
        // Create blank nodes for patch:Put structure
        let arc_node = Self::create_blank_node();
        let put_node = Self::create_blank_node();
        
        // Build the Arc (connection)
        graph.insert(&arc_node, &rdf::type_, &ingen.get("Arc")?)?;
        graph.insert(&arc_node, &ingen.get("tail")?, &IriRef::new_unchecked(source))?;
        graph.insert(&arc_node, &ingen.get("head")?, &IriRef::new_unchecked(destination))?;
                
        // Build patch:Put structure
        graph.insert(&put_node, &rdf::type_, &patch.get("Put")?)?;
        graph.insert(&put_node, &patch.get("subject")?, &IriRef::new_unchecked("ingen:/main/"))?;
        graph.insert(&put_node, &patch.get("body")?, &arc_node)?;
        Self::serialize_graph(&graph, &put_node)
    }

    /// Build an RDF graph to disconnect two ports
    pub fn build_disconnect(source: &str, destination: &str) -> Result<String> {
        debug!("Building disconnect message: '{}' -X- '{}'", source, destination);
        
        let mut graph = FastGraph::new();
        let ingen = Namespace::new(INGEN_NS)?;
        let patch = Namespace::new(PATCH_NS)?;
        
        // Create blank nodes for patch:Delete structure
        let arc_node = Self::create_blank_node();
        let delete_node = Self::create_blank_node();
        
        // Build the Arc (connection) to delete
        graph.insert(&arc_node, &rdf::type_, &ingen.get("Arc")?)?;
        graph.insert(&arc_node, &ingen.get("tail")?, &IriRef::new_unchecked(source))?;
        graph.insert(&arc_node, &ingen.get("head")?, &IriRef::new_unchecked(destination))?;
                
        // Build patch:Delete structure
        graph.insert(&delete_node, &rdf::type_, &patch.get("Delete")?)?;
        graph.insert(&delete_node, &patch.get("subject")?, &IriRef::new_unchecked("ingen:/main/"))?;
        graph.insert(&delete_node, &patch.get("body")?, &arc_node)?;
        
        Self::serialize_graph(&graph, &delete_node)
    }

    /// Build an RDF graph to delete a block or port
    pub fn build_delete(path: &str) -> Result<String> {
        debug!("Building delete message for '{}'", path);
        
        let graph = FastGraph::new();
        let root = Self::create_blank_node();

        // TODO: Implement
        
        Self::serialize_graph(&graph, &root)
    }

    /// Build an RDF graph to set a property/parameter
    pub fn build_set_property(subject: &str, property: &str, value: &str) -> Result<String> {
        debug!("Building set_property message for '{}'", subject);
        
        let graph = FastGraph::new();
        let root = Self::create_blank_node();

        // TODO: Implement
        
        Self::serialize_graph(&graph, &root)
    }

    /// Build an RDF graph to query for available plugins
    pub fn build_get_plugins() -> Result<String> {
        debug!("Building get_plugins message");
        
        let mut graph = FastGraph::new();
        let patch = Namespace::new(PATCH_NS)?;
        
        let get_node = Self::create_blank_node();
        
        // Build patch:Get structure to request plugins
        graph.insert(&get_node, &rdf::type_, &patch.get("Get")?)?;
        graph.insert(&get_node, &patch.get("subject")?, &IriRef::new_unchecked("ingen:/plugins"))?;
        
        Self::serialize_graph(&graph, &get_node)
    }

    /// Build an RDF graph to get the full engine state
    pub fn build_get_state() -> Result<String> {
        debug!("Building get_state message");
        
        let mut graph = FastGraph::new();
        let patch = Namespace::new(PATCH_NS)?;
        
        let get_node = Self::create_blank_node();
        
        // Build patch:Get structure to request engine state
        graph.insert(&get_node, &rdf::type_, &patch.get("Get")?)?;
        graph.insert(&get_node, &patch.get("subject")?, &IriRef::new_unchecked("ingen:/main/"))?;
        
        Self::serialize_graph(&graph, &get_node)
    }

    /// Parse an RDF response from Ingen
    pub fn parse_response(turtle_data: &str) -> Result<FastGraph> {
        debug!("Parsing Ingen response");
        
        // Prepend common prefixes if not already present
        let prefixes = "@prefix atom: <http://lv2plug.in/ns/ext/atom#> .
@prefix doap: <http://usefulinc.com/ns/doap#> .
@prefix ingen: <http://drobilla.net/ns/ingen#> .
@prefix lv2: <http://lv2plug.in/ns/lv2core#> .
@prefix midi: <http://lv2plug.in/ns/ext/midi#> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix patch: <http://lv2plug.in/ns/ext/patch#> .
@prefix pg: <http://lv2plug.in/ns/ext/port-groups#> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix rsz: <http://lv2plug.in/ns/ext/resize-port#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

";
        
        // Combine prefixes with data
        let full_data = format!("{}{}", prefixes, turtle_data);
        
        let graph = turtle::parse_str(&full_data)
            .collect_triples()
            .map_err(|e| anyhow!("Failed to parse Ingen RDF response: {}", e))?;
        
        Ok(graph)
    }



    /// Parse plugin list from a get_plugins response
    pub fn parse_get_plugins(response: &str) -> Result<Vec<String>> {
        debug!("Parsing plugin list from response");
        
        // Parse the response into a graph
        let graph = Self::parse_response(response)?;
        
        let patch = Namespace::new(PATCH_NS)?;
        let lv2 = Namespace::new(LV2_NS)?;
        
        let mut plugins = Vec::new();
        
        // Find all patch:Put statements
        let patch_put = patch.get("Put")?;
        let patch_subject = patch.get("subject")?;
        let patch_body = patch.get("body")?;
        let lv2_plugin = lv2.get("Plugin")?;
        
        // Iterate over all triples to find patch:Put nodes
        for triple in graph.triples() {
            let triple = triple.map_err(|e| anyhow!("Error iterating triples: {}", e))?;
            
            // Check if this is a patch:Put
            if triple.p() == &rdf::type_ && triple.o() == &patch_put {
                let put_node = triple.s();
                
                // Find the subject and body of this Put
                let mut subject_uri: Option<String> = None;
                let mut body_node = None;
                
                for t in graph.triples_matching([put_node], [&patch_subject], Any) {
                    let t = t.map_err(|e| anyhow!("Error finding subject: {}", e))?;
                    // Extract the IRI value from the object
                    if let SimpleTerm::Iri(iri) = t.o() {
                        subject_uri = Some(iri.as_str().to_string());
                    }
                }
                
                for t in graph.triples_matching([put_node], [&patch_body], Any) {
                    let t = t.map_err(|e| anyhow!("Error finding body: {}", e))?;
                    body_node = Some(t.o());
                }
                
                // Check if the body contains lv2:Plugin type
                if let (Some(uri), Some(body)) = (subject_uri, body_node) {
                    for t in graph.triples_matching([body], [&rdf::type_], [&lv2_plugin]) {
                        if t.is_ok() {
                            plugins.push(uri.clone());
                            break;
                        }
                    }
                }
            }
        }
        
        debug!("Found {} plugins", plugins.len());
        Ok(plugins)
    }

    /// Serialize a graph to Turtle format
    fn serialize_graph(graph: &FastGraph, root_node: &SimpleTerm) -> Result<String> {
        // Clone the graph to add sequence number
        let mut graph_with_seq = graph.clone();
        
        // Increment and get the sequence number
        let seq_num = SEQUENCE_NUMBER.fetch_add(1, Ordering::SeqCst);
        
        // Add sequence number to the provided root node
        let patch = Namespace::new(PATCH_NS)?;
        
        // Create the sequence number literal with xsd:int datatype
        let seq_literal = SimpleTerm::LiteralDatatype(
            MownStr::from(seq_num.to_string()),
            IriRef::new_unchecked("http://www.w3.org/2001/XMLSchema#int".into())
        );
        
        graph_with_seq.insert(root_node, &patch.get("sequenceNumber")?, &seq_literal)?;
        
        let mut serializer = TurtleSerializer::new_stringifier();
        
        let result = serializer.serialize_graph(&graph_with_seq)
            .map_err(|e| anyhow!("Failed to serialize graph: {}", e))?
            .to_string();
        
        Ok(result)
    }

    /// Parse a graph structure from an Ingen response
    pub fn parse_graph(response: &str) -> Result<super::Graph> {
        use super::{Graph, Block, Connection, Port, PortType, PortDirection};
        
        debug!("Parsing graph from Ingen response");
        
        // Parse the response into an RDF graph
        let graph = Self::parse_response(response)?;
        
        let ingen = Namespace::new(INGEN_NS)?;
        let lv2 = Namespace::new(LV2_NS)?;
        let patch = Namespace::new(PATCH_NS)?;
        
        let mut blocks = Vec::new();
        let mut connections = Vec::new();
        
        // Find all blocks from patch:Put messages
        let ingen_block = ingen.get("Block")?;
        let lv2_name = lv2.get("name")?;
        let lv2_symbol = lv2.get("symbol")?;
        let lv2_prototype = lv2.get("prototype")?;
        let lv2_audio_port = lv2.get("AudioPort")?;
        let lv2_cv_port = lv2.get("CVPort")?;
        let lv2_input_port = lv2.get("InputPort")?;
        let lv2_output_port = lv2.get("OutputPort")?;
        let patch_put = patch.get("Put")?;
        let patch_subject = patch.get("subject")?;
        let patch_body = patch.get("body")?;
        
        // Collect all block subjects from patch:Put messages
        let mut block_subjects = std::collections::HashSet::new();
        for triple in graph.triples() {
            let triple = triple.map_err(|e| anyhow!("Error iterating triples: {}", e))?;
            
            // Check if this is a patch:Put
            if triple.p() == &rdf::type_ && triple.o() == &patch_put {
                let put_node = triple.s();
                
                // Find the subject and body of this Put
                let mut subject_uri: Option<String> = None;
                let mut body_node = None;
                
                for t in graph.triples_matching([put_node], [&patch_subject], sophia::api::term::matcher::Any) {
                    let t = t.map_err(|e| anyhow!("Error finding subject: {}", e))?;
                    if let Some(iri) = t.o().iri() {
                        subject_uri = Some(iri.to_string());
                    }
                }
                
                for t in graph.triples_matching([put_node], [&patch_body], sophia::api::term::matcher::Any) {
                    let t = t.map_err(|e| anyhow!("Error finding body: {}", e))?;
                    body_node = Some(t.o());
                }
                
                // Check if the body contains ingen:Block type
                if let (Some(uri), Some(body)) = (subject_uri, body_node) {
                    for t in graph.triples_matching([body], [&rdf::type_], [&ingen_block]) {
                        if t.is_ok() {
                            block_subjects.insert(uri.clone());
                            break;
                        }
                    }
                }
            }
        }
        
        // Process each block
        for block_id in block_subjects {
            let block_iri = IriRef::new_unchecked(block_id.as_str());
            
            // Get block name from the block subject itself or from patch:Put body
            let mut name = block_id.split('/').last().unwrap_or(&block_id).to_string();
            
            // Try to get name from direct triples on the block
            for triple in graph.triples_matching([&block_iri], [&lv2_name], sophia::api::term::matcher::Any) {
                let triple = triple.map_err(|e| anyhow!("Error finding name: {}", e))?;
                if let Some(literal) = triple.o().lexical_form() {
                    name = literal.to_string();
                    break;
                }
            }
            
            // Find all ports for this block from patch:Put messages
            let mut ports = Vec::new();
            
            // Look for ports in patch:Put messages
            for triple in graph.triples() {
                let triple = triple.map_err(|e| anyhow!("Error iterating triples: {}", e))?;
                
                // Check if this is a patch:Put
                if triple.p() == &rdf::type_ && triple.o() == &patch_put {
                    let put_node = triple.s();
                    
                    // Find the subject and body of this Put
                    let mut subject_uri: Option<String> = None;
                    let mut body_node = None;
                    
                    for t in graph.triples_matching([put_node], [&patch_subject], sophia::api::term::matcher::Any) {
                        let t = t.map_err(|e| anyhow!("Error finding subject: {}", e))?;
                        if let Some(iri) = t.o().iri() {
                            subject_uri = Some(iri.to_string());
                        }
                    }
                    
                    for t in graph.triples_matching([put_node], [&patch_body], sophia::api::term::matcher::Any) {
                        let t = t.map_err(|e| anyhow!("Error finding body: {}", e))?;
                        body_node = Some(t.o());
                    }
                    
                    // Check if this port belongs to this block
                    if let (Some(port_uri), Some(body)) = (subject_uri, body_node) {
                        if port_uri.starts_with(&block_id) && port_uri != block_id {
                            let mut is_audio = false;
                            let mut is_cv = false;
                            let mut is_input = false;
                            let mut is_output = false;
                            let mut port_symbol = port_uri.split('/').last().unwrap_or("").to_string();
                            
                            // Check port properties in the body
                            for t in graph.triples_matching([body], sophia::api::term::matcher::Any, sophia::api::term::matcher::Any) {
                                let t = t.map_err(|e| anyhow!("Error finding port properties: {}", e))?;
                                
                                if t.p() == &rdf::type_ {
                                    if t.o() == &lv2_audio_port {
                                        is_audio = true;
                                    } else if t.o() == &lv2_cv_port {
                                        is_cv = true;
                                    } else if t.o() == &lv2_input_port {
                                        is_input = true;
                                    } else if t.o() == &lv2_output_port {
                                        is_output = true;
                                    }
                                } else if t.p() == &lv2_symbol {
                                    if let Some(literal) = t.o().lexical_form() {
                                        port_symbol = literal.to_string();
                                    }
                                }
                            }
                            
                            // Only add if we have both type and direction
                            if (is_audio || is_cv) && (is_input || is_output) {
                                ports.push(Port {
                                    id: port_symbol,
                                    port_type: if is_audio { PortType::Audio } else { PortType::Midi },
                                    direction: if is_input { PortDirection::Input } else { PortDirection::Output },
                                });
                            }
                        }
                    }
                }
            }
            
            // Deduplicate ports
            ports.sort_by(|a, b| a.id.cmp(&b.id));
            ports.dedup_by(|a, b| a.id == b.id);
            
            blocks.push(Block {
                id: block_id,
                name,
                ports,
            });
        }
        
        // Find all connections (ingen:Arc)
        let ingen_arc = ingen.get("Arc")?;
        let ingen_tail = ingen.get("tail")?;
        let ingen_head = ingen.get("head")?;
        
        for triple in graph.triples() {
            let triple = triple.map_err(|e| anyhow!("Error iterating triples: {}", e))?;
            
            if triple.p() == &rdf::type_ && triple.o() == &ingen_arc {
                let arc_subject = triple.s();
                
                let mut source = None;
                let mut destination = None;
                
                // Find tail (source)
                for t in graph.triples_matching([arc_subject], [&ingen_tail], sophia::api::term::matcher::Any) {
                    let t = t.map_err(|e| anyhow!("Error finding arc tail: {}", e))?;
                    if let Some(iri) = t.o().iri() {
                        source = Some(iri.to_string());
                        break;
                    }
                }
                
                // Find head (destination)
                for t in graph.triples_matching([arc_subject], [&ingen_head], sophia::api::term::matcher::Any) {
                    let t = t.map_err(|e| anyhow!("Error finding arc head: {}", e))?;
                    if let Some(iri) = t.o().iri() {
                        destination = Some(iri.to_string());
                        break;
                    }
                }
                
                if let (Some(src), Some(dst)) = (source, destination) {
                    connections.push(Connection {
                        source: src,
                        destination: dst,
                    });
                }
            }
        }
        
        // Find system ports (ports directly under ingen:/main/)
        let mut system_ports = Vec::new();
        let main_prefix = "ingen:/main/";
        
        // Parse ports from patch:Put messages
        let patch = Namespace::new(PATCH_NS)?;
        let patch_put = patch.get("Put")?;
        let patch_subject = patch.get("subject")?;
        let patch_body = patch.get("body")?;
        
        // Iterate over all triples to find patch:Put nodes
        for triple in graph.triples() {
            let triple = triple.map_err(|e| anyhow!("Error iterating triples: {}", e))?;
            
            // Check if this is a patch:Put
            if triple.p() == &rdf::type_ && triple.o() == &patch_put {
                let put_node = triple.s();
                
                // Find the subject and body of this Put
                let mut subject_uri: Option<String> = None;
                let mut body_node = None;
                
                for t in graph.triples_matching([put_node], [&patch_subject], sophia::api::term::matcher::Any) {
                    let t = t.map_err(|e| anyhow!("Error finding subject: {}", e))?;
                    // Extract the IRI value from the object
                    if let Some(iri) = t.o().iri() {
                        subject_uri = Some(iri.to_string());
                    }
                }
                
                for t in graph.triples_matching([put_node], [&patch_body], sophia::api::term::matcher::Any) {
                    let t = t.map_err(|e| anyhow!("Error finding body: {}", e))?;
                    body_node = Some(t.o());
                }
                
                // Check if this is a system port (subject is directly under ingen:/main/)
                if let (Some(uri), Some(body)) = (subject_uri, body_node) {
                    if uri.starts_with(main_prefix) && uri != "ingen:/main/" {
                        let suffix = &uri[main_prefix.len()..];
                        
                        // System port: no slashes in the suffix (not a block or block port)
                        if !suffix.contains('/') {
                            let mut is_audio = false;
                            let mut is_cv = false;
                            let mut is_input = false;
                            let mut is_output = false;
                            let mut port_name = suffix.to_string();
                            
                            // Check port properties in the body
                            for t in graph.triples_matching([body], sophia::api::term::matcher::Any, sophia::api::term::matcher::Any) {
                                let t = t.map_err(|e| anyhow!("Error finding port properties: {}", e))?;
                                
                                if t.p() == &rdf::type_ {
                                    if t.o() == &lv2_audio_port {
                                        is_audio = true;
                                    } else if t.o() == &lv2_cv_port {
                                        is_cv = true;
                                    } else if t.o() == &lv2_input_port {
                                        is_input = true;
                                    } else if t.o() == &lv2_output_port {
                                        is_output = true;
                                    }
                                } else if t.p() == &lv2_name {
                                    if let Some(literal) = t.o().lexical_form() {
                                        port_name = literal.to_string();
                                    }
                                } else if t.p() == &lv2_symbol {
                                    if let Some(literal) = t.o().lexical_form() {
                                        port_name = literal.to_string();
                                    }
                                }
                            }
                            
                            // Only add if we have both type and direction
                            if (is_audio || is_cv) && (is_input || is_output) {
                                system_ports.push(Port {
                                    id: port_name,
                                    port_type: if is_audio { PortType::Audio } else { PortType::Midi },
                                    direction: if is_input { PortDirection::Input } else { PortDirection::Output },
                                });
                            }
                        }
                    }
                }
            }
        }
        
        Ok(Graph { blocks, connections, ports: system_ports })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_create_port() {       
        let message = IngenProtocol::build_create_port("audio_out", &PortType::Audio, &PortDirection::Output).unwrap();
        println!("\nCreate Audio output port message:");
        println!("{}", message);
    }
    
    #[test]
    fn test_build_connect() {
        let message = IngenProtocol::build_connect("ingen:/main/audio_in_1", "ingen:/main/audio_out_1").unwrap();
        println!("\nConnect ports message:");
        println!("{}", message);
    }
}

