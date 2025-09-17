.PHONY: diagrams attach-diagrams

DOTS = $(wildcard doc/act/diagrams/*.dot)

diagrams:
	@for f in $(DOTS); do \
		echo "Rendering $$f"; \
		dot -Tsvg $$f -O; \
	done

# Attach diagram entries into domain-graph.json (requires SVGs to exist)
attach-diagrams:
	cargo run -q -p domain_graph_tools --bin add_diagram -- --id event_pipeline_v2 --path doc/act/diagrams/event_pipeline_v2.dot.svg --describes handled_by,causes_event,emits_event,wraps_event,references_payload_cid,appended_to_stream,collects_envelope
	cargo run -q -p domain_graph_tools --bin add_diagram -- --id identity_envelope_v2 --path doc/act/diagrams/identity_envelope_v2.dot.svg --describes identified_by_command_id,encloses_command,command_carries_identity,identified_by_query_id,encloses_query,query_carries_identity,provides_correlation_id,provides_causation_id,provides_event_id,identifies_event,identifies_aggregate,correlates_with,was_caused_by,describes_payload,command_correlates_to_event,query_correlates_to_event,precedes_envelope,acknowledged_by_command,acknowledged_by_query
	cargo run -q -p domain_graph_tools --bin add_diagram -- --id read_path_v2 --path doc/act/diagrams/read_path_v2.dot.svg --describes subscribes_to_stream,consumes_event,updates_read_model,reads_from,responds_with
	cargo run -q -p domain_graph_tools --bin add_diagram -- --id addressing_v2 --path doc/act/diagrams/addressing_v2.dot.svg --describes domain_cid_defines_node,uses_payload_codec,payload_is,annotated_by_metadata,defined_by_ipld
	cargo run -q -p domain_graph_tools --bin add_diagram -- --id bounded_context_scope_v2 --path doc/act/diagrams/bounded_context_scope_v2.dot.svg --describes scopes_aggregate,scopes_projection,scopes_read_model,scopes_event_stream,scopes_command,scopes_query,scopes_policy,scopes_state_machine,scopes_saga

