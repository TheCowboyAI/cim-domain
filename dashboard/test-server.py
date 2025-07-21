#!/usr/bin/env python3
"""
Simple HTTP server for testing the dashboard locally.
Serves the dashboard and test results JSON.

Usage:
    python3 test-server.py
    
Then open http://localhost:8080 in your browser.
"""

import http.server
import socketserver
import json
import os
from pathlib import Path

PORT = 8080

class TestDashboardHandler(http.server.SimpleHTTPRequestHandler):
    def do_GET(self):
        if self.path == '/':
            self.path = '/index.html'
        elif self.path == '/test-results/summary.json':
            # Serve the actual test results if they exist
            test_results_path = Path(__file__).parent.parent / 'test-results' / 'summary.json'
            if test_results_path.exists():
                self.send_response(200)
                self.send_header('Content-type', 'application/json')
                self.send_header('Access-Control-Allow-Origin', '*')
                self.end_headers()
                with open(test_results_path, 'rb') as f:
                    self.wfile.write(f.read())
                return
            else:
                # Return mock data for demo
                self.send_response(200)
                self.send_header('Content-type', 'application/json')
                self.send_header('Access-Control-Allow-Origin', '*')
                self.end_headers()
                mock_data = {
                    "timestamp": "2025-01-21T12:00:00Z",
                    "status": "success",
                    "total_tests": 437,
                    "passed": 437,
                    "failed": 0,
                    "ignored": 0,
                    "pass_rate": 100.00,
                    "test_suites": {
                        "library": {
                            "name": "Library Unit Tests",
                            "count": 396
                        },
                        "infrastructure": {
                            "name": "Infrastructure Integration Tests",
                            "count": 19
                        },
                        "jetstream": {
                            "name": "JetStream Event Store Tests",
                            "count": 6
                        },
                        "persistence": {
                            "name": "Persistence Integration Tests",
                            "count": 7
                        }
                    },
                    "environment": {
                        "rust_version": "1.75.0",
                        "nats_required": True,
                        "nats_endpoint": "localhost:4222"
                    }
                }
                self.wfile.write(json.dumps(mock_data, indent=2).encode())
                return
        
        # Default file serving
        return super().do_GET()

if __name__ == "__main__":
    os.chdir(Path(__file__).parent)
    
    with socketserver.TCPServer(("", PORT), TestDashboardHandler) as httpd:
        print(f"Dashboard server running at http://localhost:{PORT}")
        print("Press Ctrl+C to stop")
        try:
            httpd.serve_forever()
        except KeyboardInterrupt:
            print("\nShutting down...")
            httpd.shutdown()