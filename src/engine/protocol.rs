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
        
        let graph = turtle::parse_str(turtle_data)
            .collect_triples()
            .map_err(|e| anyhow!("Failed to parse Ingen RDF response: {}", e))?;
        
        Ok(graph)
    }

    /// Check if the message is only a bundle boundary (contains only ingen:BundleEnd nodes)
    pub fn is_bundle_boundary(turtle_data: &str) -> bool {
        debug!("Checking if message is a bundle boundary");
        
        // Try to parse the response
        let graph = match Self::parse_response(turtle_data) {
            Ok(g) => g,
            Err(_) => return false,
        };
        
        // Get the ingen:BundleEnd type
        let ingen = match Namespace::new(INGEN_NS) {
            Ok(ns) => ns,
            Err(_) => return false,
        };
        
        let bundle_end = match ingen.get("BundleEnd") {
            Ok(be) => be,
            Err(_) => return false,
        };
        
        // Check if there are any triples
        let mut has_triples = false;
        let mut all_bundle_end = true;
        
        // Iterate over all subjects with rdf:type statements
        for triple in graph.triples() {
            let triple = match triple {
                Ok(t) => t,
                Err(_) => continue,
            };
            
            has_triples = true;
            
            // If this is a type statement
            if triple.p() == &rdf::type_ {
                // Check if it's NOT a BundleEnd
                if triple.o() != &bundle_end {
                    all_bundle_end = false;
                    break;
                }
            }
        }
        
        // It's a bundle boundary only if it has triples AND all type statements are BundleEnd
        has_triples && all_bundle_end
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

