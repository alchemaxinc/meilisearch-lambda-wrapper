import unittest

import get_latest_meilisearch_version


class SyncVersionsCliTests(unittest.TestCase):
    def test_fetch_tag_names_returns_only_names(self):
        names = get_latest_meilisearch_version.fetch_versioned_tag_names()
        self.assertTrue(names)
