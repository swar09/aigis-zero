import unittest

from isolation_manager import IsolationManager


class TestIsolationManager(unittest.TestCase):

    def test_ruleset_generation_ipv4(self):
        manager = IsolationManager("1.2.3.4", 8443)

        ruleset = manager._build_ruleset()
      

        self.assertIn(
            "ip saddr 1.2.3.4 tcp sport 8443 accept",
            ruleset,
        )

        self.assertIn(
            "ip daddr 1.2.3.4 tcp dport 8443 accept",
            ruleset,
        )


if __name__ == "__main__":
    unittest.main()
