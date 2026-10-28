"""Source scope must be enforced before Graphify traverses the graph."""

import unittest

from tools.graphify_query import filter_graph


class GraphifyQueryScopeTests(unittest.TestCase):
    def test_scope_removes_other_projects_and_their_edges(self):
        graph = {
            "nodes": [
                {"id": "our_paula", "source_file": "crates/paula/src/lib.rs"},
                {"id": "minimig_paula", "source_file": "ref_src/minimig/Paula.v"},
                {"id": "vamiga_paula", "source_file": "ref_src/vAmiga/Paula.cpp"},
            ],
            "links": [
                {"source": "our_paula", "target": "minimig_paula", "source_file": "crates/paula/src/lib.rs"},
                {"source": "our_paula", "target": "vamiga_paula", "source_file": "ref_src/vAmiga/Paula.cpp"},
            ],
        }

        project = filter_graph(graph, ["project"])
        self.assertEqual([node["id"] for node in project["nodes"]], ["our_paula"])
        self.assertEqual(project["links"], [])

        comparison = filter_graph(graph, ["project", "minimig"])
        self.assertEqual({node["id"] for node in comparison["nodes"]}, {"our_paula", "minimig_paula"})
        self.assertEqual(len(comparison["links"]), 1)


if __name__ == "__main__":
    unittest.main()
